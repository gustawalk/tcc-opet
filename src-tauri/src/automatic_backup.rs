use crate::backup_service::{self, BackupSummary};
use crate::error::AppError;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, LazyLock, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const CONFIG_FORMAT_VERSION: u8 = 1;
const DESTINATION_FORMAT_VERSION: u8 = 1;
const DEFAULT_INTERVAL_HOURS: u16 = 24;
const MIN_INTERVAL_HOURS: u16 = 1;
const MAX_INTERVAL_HOURS: u16 = 168;
const INITIAL_CHECK_DELAY_HOURS: i64 = 1;
const DAILY_RETENTION: usize = 7;
const WEEKLY_RETENTION: usize = 4;
const MINIMUM_RETENTION: usize = 2;
const DESTINATION_MARKER: &str = ".opets-backup-destination.json";
const EVENT_NAME: &str = "automatic-backup-progress";
const DISK_SAFETY_MARGIN_BYTES: u64 = 16 * 1024 * 1024;
const FULL_VALIDATION_INTERVAL_DAYS: i64 = 7;

static OPERATION_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static CONFIG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static RUNTIME_STATUS: LazyLock<Mutex<RuntimeStatus>> =
    LazyLock::new(|| Mutex::new(RuntimeStatus::default()));
static AUTOMATIC_BACKUP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static INITIALIZATION_ERROR: OnceLock<(String, String)> = OnceLock::new();
static SCHEDULER_STOP: AtomicBool = AtomicBool::new(false);
static SCHEDULER_WAKE: LazyLock<(Mutex<u64>, Condvar)> =
    LazyLock::new(|| (Mutex::new(0), Condvar::new()));
static SCHEDULER_HANDLE: LazyLock<Mutex<Option<std::thread::JoinHandle<()>>>> =
    LazyLock::new(|| Mutex::new(None));
static SCHEDULER_DONE: LazyLock<(Mutex<bool>, Condvar)> =
    LazyLock::new(|| (Mutex::new(true), Condvar::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticBackupSettings {
    pub enabled: bool,
    pub destination: Option<String>,
    pub interval_hours: u16,
}

impl Default for AutomaticBackupSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            destination: None,
            interval_hours: DEFAULT_INTERVAL_HOURS,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
struct AutomaticBackupState {
    source_id: Option<String>,
    destination_id: Option<String>,
    next_backup_at: Option<String>,
    last_attempt_at: Option<String>,
    last_success_at: Option<String>,
    last_verified_at: Option<String>,
    last_full_validation_at: Option<String>,
    last_error: Option<String>,
    last_backup_path: Option<String>,
    last_backup_size_bytes: Option<u64>,
    last_backup_digest: Option<String>,
    source_fingerprint: Option<String>,
    owned_backups: Vec<OwnedBackupRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnedBackupRecord {
    file_name: String,
    created_at: String,
    size_bytes: u64,
    digest: String,
    last_verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutomaticBackupDocument {
    format_version: u8,
    settings: AutomaticBackupSettings,
    state: AutomaticBackupState,
}

impl Default for AutomaticBackupDocument {
    fn default() -> Self {
        Self {
            format_version: CONFIG_FORMAT_VERSION,
            settings: AutomaticBackupSettings::default(),
            state: AutomaticBackupState::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticBackupStatus {
    pub enabled: bool,
    pub destination: Option<String>,
    pub interval_hours: u16,
    pub next_backup_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_verified_at: Option<String>,
    pub last_error: Option<String>,
    pub last_backup_path: Option<String>,
    pub last_backup_size_bytes: Option<u64>,
    pub running: bool,
    pub progress_percent: u8,
    pub phase: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticBackupRunResult {
    pub created: bool,
    pub skipped_unchanged: bool,
    pub pruned_count: usize,
    pub backup: Option<BackupSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticBackupProgress {
    pub running: bool,
    pub percent: u8,
    pub phase: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
struct RuntimeStatus {
    running: bool,
    progress_percent: u8,
    phase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DestinationMarker {
    application: String,
    format_version: u8,
    destination_id: String,
}

#[derive(Debug, Clone)]
struct BackupCandidate {
    path: PathBuf,
    created_at: DateTime<Utc>,
    record: OwnedBackupRecord,
}

struct PruneResult {
    removed: usize,
    warning: Option<String>,
    retained: Vec<OwnedBackupRecord>,
}

struct ReservedBackupDestination {
    path: PathBuf,
    reservation: PathBuf,
}

impl Drop for ReservedBackupDestination {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.reservation);
    }
}

fn automatic_backup_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        format!("Automatic backup failed: {error}"),
        format!("O backup automático falhou: {error}"),
    )
}

fn config_path(database_path: &Path) -> Result<PathBuf, AppError> {
    if let Some(directory) = AUTOMATIC_BACKUP_DATA_DIR.get() {
        return Ok(directory.join("database.automatic-backup.json"));
    }
    let parent = database_path
        .parent()
        .ok_or_else(|| automatic_backup_error("Database path has no parent directory."))?;
    Ok(parent.join("database.automatic-backup.json"))
}

fn automatic_backup_data_dir(database_path: &Path) -> Result<PathBuf, AppError> {
    if let Some(directory) = AUTOMATIC_BACKUP_DATA_DIR.get() {
        return Ok(directory.clone());
    }
    database_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| automatic_backup_error("Database path has no parent directory."))
}

fn legacy_config_path(database_path: &Path) -> Result<PathBuf, AppError> {
    let parent = database_path
        .parent()
        .ok_or_else(|| automatic_backup_error("Database path has no parent directory."))?;
    Ok(parent.join("database.automatic-backup.json"))
}

fn initialize_automatic_backup_data_dir(database_path: &Path) -> Result<(), AppError> {
    let directory = crate::database::app_data_dir().join("automatic-backup");
    AUTOMATIC_BACKUP_DATA_DIR
        .set(directory.clone())
        .map_err(|_| {
            automatic_backup_error("Automatic backup data path is already initialized.")
        })?;
    crate::database::ensure_private_dir(&directory).map_err(automatic_backup_error)?;

    migrate_legacy_configuration(database_path, &directory)
}

fn migrate_legacy_configuration(database_path: &Path, directory: &Path) -> Result<(), AppError> {
    let current = directory.join("database.automatic-backup.json");
    let legacy = legacy_config_path(database_path)?;
    let legacy_previous = previous_config_path(&legacy);
    if !current.exists() && (legacy.exists() || legacy_previous.exists()) {
        let document = read_document(&legacy)?;
        write_document(&current, &document)?;
        let _ = fs::remove_file(&legacy);
        let _ = fs::remove_file(legacy_previous);
    }
    Ok(())
}

fn read_document(path: &Path) -> Result<AutomaticBackupDocument, AppError> {
    let previous = previous_config_path(path);
    if !path.exists() && previous.exists() {
        fs::rename(&previous, path).map_err(automatic_backup_error)?;
        sync_directory(path.parent().unwrap_or_else(|| Path::new(".")))?;
    }
    if !path.exists() {
        return Ok(AutomaticBackupDocument::default());
    }
    let document = match parse_document(path) {
        Ok(document) => document,
        Err(current_error) if previous.exists() => {
            let recovered = parse_document(&previous)?;
            fs::remove_file(path).map_err(automatic_backup_error)?;
            fs::rename(&previous, path).map_err(automatic_backup_error)?;
            sync_directory(path.parent().unwrap_or_else(|| Path::new(".")))?;
            eprintln!(
                "[BACKUP] Recovered automatic backup configuration after error: {}",
                current_error.en
            );
            recovered
        }
        Err(error) => return Err(error),
    };
    Ok(document)
}

fn parse_document(path: &Path) -> Result<AutomaticBackupDocument, AppError> {
    let document: AutomaticBackupDocument =
        serde_json::from_slice(&fs::read(path).map_err(automatic_backup_error)?)
            .map_err(automatic_backup_error)?;
    if document.format_version != CONFIG_FORMAT_VERSION
        || !(MIN_INTERVAL_HOURS..=MAX_INTERVAL_HOURS).contains(&document.settings.interval_hours)
    {
        return Err(automatic_backup_error(
            "Automatic backup configuration is invalid or unsupported.",
        ));
    }
    Ok(document)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), AppError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(automatic_backup_error)
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

fn previous_config_path(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("database.automatic-backup.previous.json")
}

fn write_document(path: &Path, document: &AutomaticBackupDocument) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| automatic_backup_error("Configuration path has no parent directory."))?;
    crate::database::ensure_private_dir(parent).map_err(automatic_backup_error)?;
    let temporary = parent.join(format!(".automatic-backup-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<(), AppError> {
        let mut file = File::create(&temporary).map_err(automatic_backup_error)?;
        file.write_all(&serde_json::to_vec_pretty(document).map_err(automatic_backup_error)?)
            .map_err(automatic_backup_error)?;
        file.sync_all().map_err(automatic_backup_error)?;
        crate::database::secure_private_file(&temporary).map_err(automatic_backup_error)?;
        let previous = previous_config_path(path);
        if previous.exists() {
            fs::remove_file(&previous).map_err(automatic_backup_error)?;
        }
        let had_previous = path.exists();
        if had_previous {
            fs::rename(path, &previous).map_err(automatic_backup_error)?;
        }
        if let Err(error) = fs::rename(&temporary, path) {
            if had_previous {
                let _ = fs::rename(&previous, path);
            }
            return Err(automatic_backup_error(error));
        }
        sync_directory(parent)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn load_document(database_path: &Path) -> Result<AutomaticBackupDocument, AppError> {
    ensure_automatic_backup_initialized()?;
    let _guard = CONFIG_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let path = config_path(database_path)?;
    let mut document = read_document(&path)?;
    if document.state.source_id.is_none() {
        document.state.source_id = Some(Uuid::new_v4().to_string());
        write_document(&path, &document)?;
    }
    Ok(document)
}

fn ensure_automatic_backup_initialized() -> Result<(), AppError> {
    match INITIALIZATION_ERROR.get() {
        Some((en, pt)) => Err(AppError::new(en.clone(), pt.clone())),
        None => Ok(()),
    }
}

fn save_document(database_path: &Path, document: &AutomaticBackupDocument) -> Result<(), AppError> {
    ensure_automatic_backup_initialized()?;
    let _guard = CONFIG_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    write_document(&config_path(database_path)?, document)
}

fn mutate_document<T>(
    database_path: &Path,
    mutation: impl FnOnce(&mut AutomaticBackupDocument) -> Result<T, AppError>,
) -> Result<T, AppError> {
    ensure_automatic_backup_initialized()?;
    let _guard = CONFIG_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let path = config_path(database_path)?;
    let mut document = read_document(&path)?;
    let result = mutation(&mut document)?;
    write_document(&path, &document)?;
    Ok(result)
}

fn status_from_document(document: AutomaticBackupDocument) -> AutomaticBackupStatus {
    let runtime = RUNTIME_STATUS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    AutomaticBackupStatus {
        enabled: document.settings.enabled,
        destination: document.settings.destination,
        interval_hours: document.settings.interval_hours,
        next_backup_at: document.state.next_backup_at,
        last_attempt_at: document.state.last_attempt_at,
        last_success_at: document.state.last_success_at,
        last_verified_at: document.state.last_verified_at,
        last_error: document.state.last_error,
        last_backup_path: document.state.last_backup_path,
        last_backup_size_bytes: document.state.last_backup_size_bytes,
        running: runtime.running,
        progress_percent: runtime.progress_percent,
        phase: runtime.phase,
    }
}

pub fn get_status() -> Result<AutomaticBackupStatus, AppError> {
    Ok(status_from_document(load_document(
        &crate::database::database_path(),
    )?))
}

fn marker_path(destination: &Path) -> PathBuf {
    destination.join(DESTINATION_MARKER)
}

fn ensure_destination_marker(
    destination: &Path,
    expected_id: Option<&str>,
) -> Result<String, AppError> {
    fs::create_dir_all(destination).map_err(automatic_backup_error)?;
    let metadata = fs::symlink_metadata(destination).map_err(automatic_backup_error)?;
    if !metadata.file_type().is_dir() {
        return Err(automatic_backup_error(
            "Automatic backup destination must be a directory.",
        ));
    }
    let path = marker_path(destination);
    if path.exists() {
        let marker: DestinationMarker =
            serde_json::from_slice(&fs::read(&path).map_err(automatic_backup_error)?)
                .map_err(automatic_backup_error)?;
        if marker.application != "com.walk.tcc-opet"
            || marker.format_version != DESTINATION_FORMAT_VERSION
            || expected_id.is_some_and(|value| value != marker.destination_id)
        {
            return Err(automatic_backup_error(
                "Automatic backup destination marker does not match this configuration.",
            ));
        }
        return Ok(marker.destination_id);
    }
    if expected_id.is_some() {
        return Err(automatic_backup_error(
            "Automatic backup destination is unavailable or was replaced.",
        ));
    }
    let lock_path = destination.join(".opets-backup-destination.lock");
    let stale_lock = fs::metadata(&lock_path)
        .and_then(|metadata| metadata.modified())
        .and_then(|modified| modified.elapsed().map_err(std::io::Error::other))
        .is_ok_and(|age| age > Duration::from_secs(5 * 60));
    if stale_lock {
        let _ = fs::remove_file(&lock_path);
    }
    let marker_lock = (0..40)
        .find_map(|_| {
            match OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(file) => Some(Ok(file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(Duration::from_millis(50));
                    None
                }
                Err(error) => Some(Err(error)),
            }
        })
        .transpose()
        .map_err(automatic_backup_error)?
        .ok_or_else(|| automatic_backup_error("Automatic backup destination is busy."))?;
    crate::database::secure_private_file(&lock_path).map_err(automatic_backup_error)?;
    if path.exists() {
        drop(marker_lock);
        let _ = fs::remove_file(&lock_path);
        return ensure_destination_marker(destination, expected_id);
    }
    let marker = DestinationMarker {
        application: "com.walk.tcc-opet".to_string(),
        format_version: DESTINATION_FORMAT_VERSION,
        destination_id: Uuid::new_v4().to_string(),
    };
    let temporary = destination.join(format!(".opets-destination-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<(), AppError> {
        let mut file = File::create(&temporary).map_err(automatic_backup_error)?;
        file.write_all(&serde_json::to_vec_pretty(&marker).map_err(automatic_backup_error)?)
            .map_err(automatic_backup_error)?;
        file.sync_all().map_err(automatic_backup_error)?;
        crate::database::secure_private_file(&temporary).map_err(automatic_backup_error)?;
        fs::rename(&temporary, &path).map_err(automatic_backup_error)?;
        sync_directory(destination)?;
        Ok(())
    })();
    drop(marker_lock);
    let _ = fs::remove_file(lock_path);
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result.map(|_| marker.destination_id)
}

pub fn update_settings(
    settings: AutomaticBackupSettings,
) -> Result<AutomaticBackupStatus, AppError> {
    let _operation = OPERATION_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !(MIN_INTERVAL_HOURS..=MAX_INTERVAL_HOURS).contains(&settings.interval_hours) {
        return Err(AppError::new(
            "Automatic backup interval must be between 1 and 168 hours.",
            "O intervalo do backup automático deve ficar entre 1 e 168 horas.",
        ));
    }
    let destination = settings
        .destination
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    if settings.enabled && destination.is_none() {
        return Err(AppError::new(
            "Select a destination before enabling automatic backups.",
            "Selecione uma pasta antes de ativar o backup automático.",
        ));
    }

    let database_path = crate::database::database_path();
    let previous = load_document(&database_path)?;
    let normalized_destination = destination
        .as_ref()
        .map(|value| value.to_string_lossy().to_string());
    let destination_changed = previous.settings.destination != normalized_destination;
    let interval_changed = previous.settings.interval_hours != settings.interval_hours;
    if let Some(destination) = destination
        .as_deref()
        .filter(|_| settings.enabled || destination_changed)
    {
        validate_destination_path(
            destination,
            &database_path,
            &crate::database::attachments_dir(),
        )?;
    }
    let destination_id = match destination
        .as_deref()
        .filter(|_| settings.enabled || destination_changed)
    {
        Some(path) => ensure_destination_marker(
            path,
            if destination_changed {
                None
            } else {
                previous.state.destination_id.as_deref()
            },
        )
        .map(Some)?,
        None if destination.is_some() && !destination_changed => {
            previous.state.destination_id.clone()
        }
        None => None,
    };
    let normalized = AutomaticBackupSettings {
        enabled: settings.enabled,
        destination: normalized_destination,
        interval_hours: settings.interval_hours,
    };
    let now = Utc::now();
    let mut document = previous;
    let enabling = normalized.enabled && !document.settings.enabled;
    document.settings = normalized;
    if document.state.source_id.is_none() {
        document.state.source_id = Some(Uuid::new_v4().to_string());
    }
    document.state.destination_id = destination_id;
    if destination_changed {
        document.state.source_fingerprint = None;
        document.state.last_attempt_at = None;
        document.state.last_success_at = None;
        document.state.last_verified_at = None;
        document.state.last_full_validation_at = None;
        document.state.last_error = None;
        document.state.last_backup_path = None;
        document.state.last_backup_size_bytes = None;
        document.state.last_backup_digest = None;
        document.state.owned_backups.clear();
    }
    if document.settings.enabled {
        if enabling
            || destination_changed
            || interval_changed
            || document.state.next_backup_at.is_none()
        {
            document.state.next_backup_at = Some(
                (now + ChronoDuration::hours(document.settings.interval_hours as i64)).to_rfc3339(),
            );
        }
    } else {
        document.state.next_backup_at = None;
    }
    save_document(&database_path, &document)?;
    notify_scheduler();
    Ok(status_from_document(document))
}

fn validate_destination_path(
    destination: &Path,
    database_path: &Path,
    attachments_path: &Path,
) -> Result<(), AppError> {
    fs::create_dir_all(destination).map_err(automatic_backup_error)?;
    let destination = fs::canonicalize(destination).map_err(automatic_backup_error)?;
    let database = fs::canonicalize(database_path).map_err(automatic_backup_error)?;
    let database_parent = database.parent();
    let attachments = if attachments_path.exists() {
        fs::canonicalize(attachments_path).map_err(automatic_backup_error)?
    } else {
        attachments_path.to_path_buf()
    };
    if destination == database
        || database_parent.is_some_and(|parent| destination == parent)
        || destination == attachments
        || destination.starts_with(&attachments)
    {
        return Err(AppError::new(
            "The backup destination cannot overlap the active database or attachment storage.",
            "A pasta de backup não pode ficar dentro do banco ativo ou do armazenamento de anexos.",
        ));
    }
    Ok(())
}

fn parse_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn notify_scheduler() {
    let (generation, wake) = &*SCHEDULER_WAKE;
    let mut generation = generation.lock().unwrap_or_else(|error| error.into_inner());
    *generation = generation.wrapping_add(1);
    wake.notify_all();
}

fn is_due(document: &AutomaticBackupDocument, now: DateTime<Utc>) -> bool {
    if !document.settings.enabled || document.settings.destination.is_none() {
        return false;
    }
    let Some(due) = parse_timestamp(document.state.next_backup_at.as_deref()) else {
        return true;
    };
    due <= now
        || due
            > now
                + ChronoDuration::hours(document.settings.interval_hours as i64)
                + ChronoDuration::hours(1)
}

fn emit_progress(app: Option<&AppHandle>, percent: u8, phase: &str, message: &str) {
    let mut status = RUNTIME_STATUS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    status.running = percent < 100;
    status.progress_percent = percent;
    status.phase = (percent < 100).then(|| phase.to_string());
    drop(status);
    if let Some(app) = app {
        let _ = app.emit(
            EVENT_NAME,
            AutomaticBackupProgress {
                running: percent < 100,
                percent,
                phase: phase.to_string(),
                message: message.to_string(),
            },
        );
    }
}

fn source_fingerprint(database_path: &Path, attachments_path: &Path) -> Result<String, AppError> {
    let mut hasher = blake3::Hasher::new();
    let mut database_file = File::open(database_path).map_err(automatic_backup_error)?;
    hasher
        .update_reader(&mut database_file)
        .map_err(automatic_backup_error)?;
    let connection =
        if crate::database::is_plaintext_database(database_path).map_err(automatic_backup_error)? {
            rusqlite::Connection::open(database_path)?
        } else {
            crate::database::open_encrypted_database(database_path)?
        };
    let mut statement = connection
        .prepare("SELECT storage_name FROM service_order_attachments ORDER BY storage_name")?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for name in names {
        if Path::new(&name).components().count() != 1
            || !matches!(
                Path::new(&name).components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return Err(automatic_backup_error(
                "Stored attachment filename is invalid.",
            ));
        }
        hasher.update(name.as_bytes());
        let mut attachment =
            File::open(attachments_path.join(name)).map_err(automatic_backup_error)?;
        hasher
            .update_reader(&mut attachment)
            .map_err(automatic_backup_error)?;
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn file_digest(path: &Path) -> Result<String, AppError> {
    let mut file = File::open(path).map_err(automatic_backup_error)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(automatic_backup_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn full_validation_is_due(document: &AutomaticBackupDocument, now: DateTime<Utc>) -> bool {
    parse_timestamp(document.state.last_full_validation_at.as_deref()).is_none_or(|last_verified| {
        last_verified <= now - ChronoDuration::days(FULL_VALIDATION_INTERVAL_DAYS)
            || last_verified > now + ChronoDuration::hours(1)
    })
}

fn source_size(database_path: &Path, attachments_path: &Path) -> Result<u64, AppError> {
    let mut size = fs::metadata(database_path)
        .map_err(automatic_backup_error)?
        .len();
    let connection =
        if crate::database::is_plaintext_database(database_path).map_err(automatic_backup_error)? {
            rusqlite::Connection::open(database_path)?
        } else {
            crate::database::open_encrypted_database(database_path)?
        };
    let mut statement = connection
        .prepare("SELECT storage_name FROM service_order_attachments ORDER BY storage_name")?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for name in names {
        let path = attachments_path.join(name);
        let metadata = fs::symlink_metadata(path).map_err(automatic_backup_error)?;
        if metadata.file_type().is_file() {
            size = size.saturating_add(metadata.len());
        }
    }
    Ok(size)
}

fn ensure_disk_capacity(destination: &Path, source_bytes: u64) -> Result<(), AppError> {
    let required = source_bytes
        .saturating_mul(3)
        .saturating_add(DISK_SAFETY_MARGIN_BYTES);
    let available = fs2::available_space(destination).map_err(automatic_backup_error)?;
    if available < required {
        return Err(AppError::new(
            format!(
                "Automatic backup needs approximately {required} free bytes, but only {available} are available."
            ),
            format!(
                "O backup automático precisa de aproximadamente {required} bytes livres, mas apenas {available} estão disponíveis."
            ),
        ));
    }
    Ok(())
}

fn ensure_staging_capacity(staging_parent: &Path, source_bytes: u64) -> Result<(), AppError> {
    let required = source_bytes.saturating_add(DISK_SAFETY_MARGIN_BYTES);
    let available = fs2::available_space(staging_parent).map_err(automatic_backup_error)?;
    if available < required {
        return Err(AppError::new(
            format!(
                "Backup staging needs approximately {required} free bytes, but only {available} are available."
            ),
            format!(
                "A preparação do backup precisa de aproximadamente {required} bytes livres, mas apenas {available} estão disponíveis."
            ),
        ));
    }
    Ok(())
}

fn backup_file_name(now: DateTime<Utc>, collision: usize) -> String {
    let timestamp = now.with_timezone(&Local).format("%Y%m%d-%H%M%S");
    if collision == 1 {
        format!("opets-auto-{timestamp}.osbkp")
    } else {
        format!("opets-auto-{timestamp}-{collision}.osbkp")
    }
}

fn reserve_backup_destination(
    destination: &Path,
    now: DateTime<Utc>,
) -> Result<ReservedBackupDestination, AppError> {
    for collision in 1..=9999 {
        let file_name = backup_file_name(now, collision);
        let path = destination.join(&file_name);
        if path.exists() {
            continue;
        }
        let reservation = destination.join(format!(".{file_name}.lock"));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&reservation)
        {
            Ok(_) if !path.exists() => {
                return Ok(ReservedBackupDestination { path, reservation });
            }
            Ok(_) => {
                let _ = fs::remove_file(reservation);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(automatic_backup_error(error)),
        }
    }
    Err(automatic_backup_error(
        "Could not reserve a unique automatic backup filename.",
    ))
}

fn candidate_from_record(destination: &Path, record: OwnedBackupRecord) -> Option<BackupCandidate> {
    let file_name = Path::new(&record.file_name);
    if file_name.components().count() != 1
        || !matches!(
            file_name.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return None;
    }
    let path = destination.join(file_name);
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    Some(BackupCandidate {
        path,
        created_at: parse_timestamp(Some(&record.created_at))?,
        record,
    })
}

fn legacy_record(path: &Path, source_id: &str) -> Option<OwnedBackupRecord> {
    let name = path.file_name()?.to_str()?;
    let remainder = name.strip_prefix(&format!("opets-auto-{source_id}-"))?;
    let timestamp = remainder.get(..15)?;
    let identifier = remainder.get(15..)?.strip_prefix('-')?;
    if !identifier.ends_with(".osbkp") {
        return None;
    }
    let identifier = identifier.strip_suffix(".osbkp")?;
    Uuid::parse_str(identifier).ok()?;
    let created_at = NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S")
        .ok()?
        .and_utc();
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    Some(OwnedBackupRecord {
        file_name: name.to_string(),
        created_at: created_at.to_rfc3339(),
        size_bytes: metadata.len(),
        digest: String::new(),
        last_verified_at: None,
    })
}

fn collect_owned_backups(
    document: &AutomaticBackupDocument,
    destination: &Path,
) -> Vec<OwnedBackupRecord> {
    let mut records = document.state.owned_backups.clone();
    if let Some(source_id) = document.state.source_id.as_deref() {
        if let Ok(entries) = fs::read_dir(destination) {
            records.extend(
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| legacy_record(&entry.path(), source_id)),
            );
        }
    }
    let mut names = HashSet::new();
    records.retain(|record| {
        let file_name = Path::new(&record.file_name);
        let safe_name = file_name.components().count() == 1
            && matches!(
                file_name.components().next(),
                Some(std::path::Component::Normal(_))
            );
        safe_name
            && names.insert(record.file_name.clone())
            && fs::symlink_metadata(destination.join(file_name))
                .is_ok_and(|metadata| metadata.file_type().is_file())
    });
    records
}

fn verify_next_retained_backup(
    records: &mut [OwnedBackupRecord],
    destination: &Path,
    latest_backup: &Path,
    verified_at: DateTime<Utc>,
) -> Result<Option<PathBuf>, AppError> {
    let Some(record) = records
        .iter_mut()
        .filter(|record| destination.join(&record.file_name) != latest_backup)
        .min_by_key(|record| parse_timestamp(record.last_verified_at.as_deref()))
    else {
        return Ok(None);
    };
    let path = destination.join(&record.file_name);
    let metadata = fs::symlink_metadata(&path).map_err(automatic_backup_error)?;
    if !metadata.file_type().is_file() {
        return Ok(Some(path));
    }
    let digest = file_digest(&path)?;
    if (!record.digest.is_empty() && record.digest != digest)
        || (record.size_bytes != 0 && record.size_bytes != metadata.len())
    {
        return Ok(Some(path));
    }
    if record.digest.is_empty() {
        if backup_service::validate_backup_contents_with_passphrase(&path, None).is_err() {
            return Ok(Some(path));
        }
        record.digest = digest;
        record.size_bytes = metadata.len();
    }
    record.last_verified_at = Some(verified_at.to_rfc3339());
    Ok(None)
}

fn retention_paths_to_delete(
    mut candidates: Vec<BackupCandidate>,
) -> (Vec<PathBuf>, HashSet<PathBuf>) {
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.created_at));
    let mut keep = HashSet::new();
    for candidate in candidates.iter().take(MINIMUM_RETENTION) {
        keep.insert(candidate.path.clone());
    }

    let mut daily_dates = HashSet::new();
    for candidate in &candidates {
        if daily_dates.len() >= DAILY_RETENTION {
            break;
        }
        if daily_dates.insert(candidate.created_at.date_naive()) {
            keep.insert(candidate.path.clone());
        }
    }
    let oldest_daily = daily_dates.iter().min().copied();
    let mut weekly_periods = HashSet::new();
    for candidate in &candidates {
        if weekly_periods.len() >= WEEKLY_RETENTION {
            break;
        }
        if oldest_daily.is_some_and(|date| candidate.created_at.date_naive() >= date) {
            continue;
        }
        let iso_week = candidate.created_at.iso_week();
        if weekly_periods.insert((iso_week.year(), iso_week.week())) {
            keep.insert(candidate.path.clone());
        }
    }

    let delete = candidates
        .into_iter()
        .filter(|candidate| !keep.contains(&candidate.path))
        .map(|candidate| candidate.path)
        .collect();
    (delete, keep)
}

fn prune_backups(
    destination: &Path,
    records: Vec<OwnedBackupRecord>,
    protected: Option<&Path>,
) -> Result<PruneResult, AppError> {
    let candidates: Vec<_> = records
        .into_iter()
        .filter_map(|record| candidate_from_record(destination, record))
        .collect();
    let (delete, keep) = retention_paths_to_delete(candidates.clone());
    let mut removed = 0;
    let mut errors = Vec::new();
    let mut failed_deletions = HashSet::new();
    for path in delete {
        if protected.is_some_and(|protected| protected == path) {
            continue;
        }
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(error) => {
                failed_deletions.insert(path);
                errors.push(error.to_string());
            }
        }
    }
    if removed > 0 {
        if let Err(error) = sync_directory(destination) {
            errors.push(error.pt);
        }
    }
    let retained = candidates
        .into_iter()
        .filter(|candidate| {
            keep.contains(&candidate.path)
                || failed_deletions.contains(&candidate.path)
                || protected.is_some_and(|protected| protected == candidate.path)
        })
        .map(|candidate| candidate.record)
        .collect();
    Ok(PruneResult {
        removed,
        warning: (!errors.is_empty()).then(|| {
            format!(
                "Não foi possível remover {} backup(s) antigo(s): {}",
                errors.len(),
                errors.join("; ")
            )
        }),
        retained,
    })
}

fn record_failure(database_path: &Path, now: DateTime<Utc>, error: &AppError) {
    let _ = mutate_document(database_path, |document| {
        document.state.last_attempt_at = Some(now.to_rfc3339());
        document.state.last_error = Some(error.pt.clone());
        if document.settings.enabled {
            document.state.next_backup_at =
                Some((now + ChronoDuration::hours(INITIAL_CHECK_DELAY_HOURS)).to_rfc3339());
        }
        Ok(())
    });
}

fn run_with_paths(
    database_path: &Path,
    attachments_path: &Path,
    force: bool,
    now: DateTime<Utc>,
    app: Option<&AppHandle>,
) -> Result<AutomaticBackupRunResult, AppError> {
    let initial = load_document(database_path)?;
    if !initial.settings.enabled {
        if force {
            return Err(AppError::new(
                "Automatic backup is disabled.",
                "O backup automático está desativado.",
            ));
        }
        return Ok(AutomaticBackupRunResult {
            created: false,
            skipped_unchanged: false,
            pruned_count: 0,
            backup: None,
        });
    }
    if !force && !is_due(&initial, now) {
        return Ok(AutomaticBackupRunResult {
            created: false,
            skipped_unchanged: false,
            pruned_count: 0,
            backup: None,
        });
    }
    emit_progress(app, 1, "preparing", "Iniciando o backup automático.");
    let destination = initial
        .settings
        .destination
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| {
            AppError::new(
                "Configure an automatic backup destination first.",
                "Configure primeiro uma pasta para o backup automático.",
            )
        })?;
    let destination_id = initial.state.destination_id.as_deref().ok_or_else(|| {
        automatic_backup_error("Automatic backup destination identity is missing.")
    })?;
    ensure_destination_marker(&destination, Some(destination_id))?;
    cleanup_stale_destination_files(&destination);

    emit_progress(app, 5, "preparing", "Preparando o backup automático.");
    let storage_guard = crate::database::exclusive_storage_guard()?;
    emit_progress(app, 15, "checking", "Verificando alterações nos dados.");
    let fingerprint = source_fingerprint(database_path, attachments_path)?;
    let mut invalid_previous_backup = None;
    let mut owned_backups = collect_owned_backups(&initial, &destination);
    let latest_backup = initial
        .state
        .last_backup_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_default();
    let latest_backup_exists = owned_backups
        .iter()
        .any(|record| destination.join(&record.file_name) == latest_backup)
        && fs::symlink_metadata(&latest_backup)
            .is_ok_and(|metadata| metadata.file_type().is_file());
    let (storage_guard, fingerprint) = if latest_backup_exists
        && initial.state.source_fingerprint.as_deref() == Some(&fingerprint)
    {
        drop(storage_guard);
        let latest_backup = latest_backup.as_path();
        emit_progress(app, 70, "validating", "Validando o backup mais recente.");
        let digest = file_digest(latest_backup).ok();
        let digest_matches = digest.as_deref() == initial.state.last_backup_digest.as_deref()
            && fs::metadata(latest_backup)
                .ok()
                .map(|metadata| metadata.len())
                == initial.state.last_backup_size_bytes;
        let requires_full_validation = full_validation_is_due(&initial, now);
        let mut backup_is_valid = digest_matches
            && (!requires_full_validation
                || backup_service::validate_backup_contents_with_passphrase(latest_backup, None)
                    .is_ok());
        if backup_is_valid {
            invalid_previous_backup = verify_next_retained_backup(
                &mut owned_backups,
                &destination,
                latest_backup,
                Utc::now().max(now),
            )?;
            backup_is_valid = invalid_previous_backup.is_none();
        }
        if backup_is_valid {
            let current_guard = crate::database::exclusive_storage_guard()?;
            let current_fingerprint = source_fingerprint(database_path, attachments_path)?;
            if current_fingerprint == fingerprint {
                drop(current_guard);
                let completed_at = Utc::now().max(now);
                mutate_document(database_path, |document| {
                    document.state.last_attempt_at = Some(completed_at.to_rfc3339());
                    document.state.last_verified_at = Some(completed_at.to_rfc3339());
                    if requires_full_validation {
                        document.state.last_full_validation_at = Some(completed_at.to_rfc3339());
                    }
                    document.state.last_error = None;
                    document.state.owned_backups = owned_backups.clone();
                    document.state.next_backup_at = Some(
                        (completed_at
                            + ChronoDuration::hours(document.settings.interval_hours as i64))
                        .to_rfc3339(),
                    );
                    Ok(())
                })?;
                emit_progress(
                    app,
                    100,
                    "unchanged",
                    "Os dados não mudaram; nenhum espaço adicional foi usado.",
                );
                return Ok(AutomaticBackupRunResult {
                    created: false,
                    skipped_unchanged: true,
                    pruned_count: 0,
                    backup: None,
                });
            }
            (current_guard, current_fingerprint)
        } else {
            emit_progress(
                app,
                25,
                "preparing",
                "O backup anterior falhou na validação; criando uma nova cópia.",
            );
            if invalid_previous_backup.is_none() {
                invalid_previous_backup = Some(latest_backup.to_path_buf());
            }
            let current_guard = crate::database::exclusive_storage_guard()?;
            let current_fingerprint = source_fingerprint(database_path, attachments_path)?;
            (current_guard, current_fingerprint)
        }
    } else {
        (storage_guard, fingerprint)
    };

    let source_bytes = source_size(database_path, attachments_path)?;
    let staging_parent = automatic_backup_data_dir(database_path)?;
    ensure_staging_capacity(&staging_parent, source_bytes)?;
    emit_progress(
        app,
        25,
        "snapshot",
        "Criando uma cópia consistente dos dados.",
    );
    let prepared =
        backup_service::prepare_backup_sources(database_path, attachments_path, &staging_parent)?;
    drop(storage_guard);
    ensure_disk_capacity(&destination, source_bytes)?;
    let reserved_destination = reserve_backup_destination(&destination, now)?;
    let backup_path = reserved_destination.path.clone();
    emit_progress(app, 30, "exporting", "Empacotando banco e anexos.");
    let summary =
        backup_service::export_prepared_backup_with_passphrase(&prepared, &backup_path, None)?;
    drop(prepared);

    emit_progress(app, 80, "validating", "Validando a integridade do backup.");
    if let Err(error) = backup_service::validate_backup_contents_with_passphrase(&backup_path, None)
    {
        let _ = fs::remove_file(&backup_path);
        return Err(error);
    }
    let backup_size = fs::metadata(&backup_path)
        .map_err(automatic_backup_error)?
        .len();
    let backup_digest = file_digest(&backup_path)?;
    let completed_at = Utc::now().max(now);
    emit_progress(app, 95, "retention", "Aplicando a retenção de backups.");
    if let Some(invalid_previous_backup) = invalid_previous_backup.as_deref() {
        owned_backups
            .retain(|record| destination.join(&record.file_name) != invalid_previous_backup);
    }
    owned_backups.retain(|record| destination.join(&record.file_name) != backup_path);
    owned_backups.push(OwnedBackupRecord {
        file_name: backup_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| automatic_backup_error("Backup filename is invalid."))?
            .to_string(),
        created_at: completed_at.to_rfc3339(),
        size_bytes: backup_size,
        digest: backup_digest.clone(),
        last_verified_at: Some(completed_at.to_rfc3339()),
    });
    let retained_on_error = owned_backups.clone();
    let prune_result = match prune_backups(&destination, owned_backups, Some(&backup_path)) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("[BACKUP] Retention cleanup failed: {}", error.en);
            PruneResult {
                removed: 0,
                warning: Some(error.pt),
                retained: retained_on_error,
            }
        }
    };
    let pruned_count = prune_result.removed;
    mutate_document(database_path, |document| {
        document.state.last_attempt_at = Some(completed_at.to_rfc3339());
        document.state.last_success_at = Some(completed_at.to_rfc3339());
        document.state.last_verified_at = Some(completed_at.to_rfc3339());
        document.state.last_full_validation_at = Some(completed_at.to_rfc3339());
        document.state.last_error = prune_result.warning.clone();
        document.state.last_backup_path = Some(summary.path.clone());
        document.state.last_backup_size_bytes = Some(backup_size);
        document.state.last_backup_digest = Some(backup_digest);
        document.state.source_fingerprint = Some(fingerprint);
        document.state.owned_backups = prune_result.retained.clone();
        if document.settings.enabled {
            document.state.next_backup_at = Some(
                (completed_at + ChronoDuration::hours(document.settings.interval_hours as i64))
                    .to_rfc3339(),
            );
        }
        Ok(())
    })?;
    if let Some(invalid_previous_backup) = invalid_previous_backup {
        if fs::remove_file(invalid_previous_backup).is_ok() {
            let _ = sync_directory(&destination);
        }
    }
    emit_progress(
        app,
        100,
        "completed",
        "Backup automático concluído e validado.",
    );
    Ok(AutomaticBackupRunResult {
        created: true,
        skipped_unchanged: false,
        pruned_count,
        backup: Some(summary),
    })
}

pub fn run(force: bool, app: Option<&AppHandle>) -> Result<AutomaticBackupRunResult, AppError> {
    let _operation = match OPERATION_LOCK.try_lock() {
        Ok(operation) => operation,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
        Err(std::sync::TryLockError::WouldBlock) => {
            return Err(AppError::new(
                "An automatic backup is already running.",
                "Um backup automático já está em andamento.",
            ));
        }
    };
    let database_path = crate::database::database_path();
    let now = Utc::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_with_paths(
            &database_path,
            &crate::database::attachments_dir(),
            force,
            now,
            app,
        )
    }))
    .unwrap_or_else(|_| {
        Err(automatic_backup_error(
            "The automatic backup task stopped unexpectedly.",
        ))
    });
    if let Err(error) = &result {
        record_failure(&database_path, Utc::now().max(now), error);
        emit_progress(app, 100, "failed", &error.pt);
    }
    result
}

fn cleanup_stale_temporary_files(directory: &Path) -> std::io::Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("[BACKUP] Failed to inspect a stale temporary file: {error}");
                continue;
            }
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let path = entry.path();
        if name.starts_with(".opets-backup-staging-") && path.is_dir() {
            if let Err(error) = fs::remove_dir_all(path) {
                eprintln!("[BACKUP] Failed to remove a stale staging directory: {error}");
            }
        } else if path.is_file()
            && (name.starts_with(".opet-snapshot-")
                || name.starts_with(".opets-backup-archive-")
                || name.starts_with(".opets-backup-encrypted-")
                || name.starts_with(".opet-backup-")
                || name.starts_with(".opets-backup-validation-")
                || (name.starts_with(".automatic-backup-") && name.ends_with(".tmp")))
        {
            if let Err(error) = fs::remove_file(path) {
                eprintln!("[BACKUP] Failed to remove a stale temporary file: {error}");
            }
        }
    }
    Ok(())
}

fn cleanup_stale_destination_files(destination: &Path) {
    let Ok(entries) = fs::read_dir(destination) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let reserved_name = name.starts_with(".opet-backup-")
            || name.starts_with(".opets-backup-archive-")
            || name.starts_with(".opets-backup-encrypted-")
            || name.starts_with(".opets-backup-validation-")
            || name.starts_with(".opets-backup-previous-")
            || (name.starts_with(".opets-auto-") && name.ends_with(".osbkp.lock"));
        let stale_regular_file = fs::symlink_metadata(&path).is_ok_and(|metadata| {
            metadata.file_type().is_file()
                && metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age > Duration::from_secs(24 * 60 * 60))
        });
        if reserved_name && stale_regular_file {
            let _ = fs::remove_file(path);
        }
    }
}

fn scheduler_wait_duration(database_path: &Path) -> Option<Duration> {
    let document = load_document(database_path).ok()?;
    if !document.settings.enabled {
        return None;
    }
    document
        .state
        .next_backup_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .and_then(|next| (next.with_timezone(&Utc) - Utc::now()).to_std().ok())
        .map(|duration| duration.min(Duration::from_secs(60 * 60)))
        .or(Some(Duration::from_secs(60 * 60)))
}

pub fn start_scheduler(app: AppHandle) -> std::io::Result<()> {
    let database_path = crate::database::database_path();
    if let Err(error) = initialize_automatic_backup_data_dir(&database_path) {
        eprintln!("[BACKUP] {}", error.en);
        let _ = INITIALIZATION_ERROR.set((error.en, error.pt));
        return Ok(());
    }
    if let Ok(directory) = automatic_backup_data_dir(&database_path) {
        let _ = cleanup_stale_temporary_files(&directory);
    }
    SCHEDULER_STOP.store(false, Ordering::Release);
    *SCHEDULER_DONE
        .0
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = false;
    let handle = std::thread::Builder::new()
        .name("opets-automatic-backup".to_string())
        .spawn(move || {
            loop {
                if SCHEDULER_STOP.load(Ordering::Acquire) {
                    break;
                }
                if let Err(error) = run(false, Some(&app)) {
                    eprintln!("[BACKUP] {}", error.en);
                }
                let (generation, wake) = &*SCHEDULER_WAKE;
                let observed = generation.lock().unwrap_or_else(|error| error.into_inner());
                let current = *observed;
                match scheduler_wait_duration(&database_path) {
                    Some(wait) => {
                        let _ = wake.wait_timeout_while(observed, wait, |generation| {
                            *generation == current && !SCHEDULER_STOP.load(Ordering::Acquire)
                        });
                    }
                    None => {
                        let slept = wake.wait_while(observed, |generation| {
                            *generation == current && !SCHEDULER_STOP.load(Ordering::Acquire)
                        });
                        drop(slept);
                    }
                }
            }
            let (done, wake) = &*SCHEDULER_DONE;
            *done.lock().unwrap_or_else(|error| error.into_inner()) = true;
            wake.notify_all();
        })?;
    *SCHEDULER_HANDLE
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(handle);
    Ok(())
}

pub fn stop_scheduler() {
    SCHEDULER_STOP.store(true, Ordering::Release);
    notify_scheduler();
    let (done, wake) = &*SCHEDULER_DONE;
    let done = done.lock().unwrap_or_else(|error| error.into_inner());
    let (done, _) = wake
        .wait_timeout_while(done, Duration::from_secs(5), |done| !*done)
        .unwrap_or_else(|error| error.into_inner());
    let handle = SCHEDULER_HANDLE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    if *done {
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::run_migrations;
    use rusqlite::Connection;

    fn create_candidate(
        directory: &Path,
        created_at: DateTime<Utc>,
        collision: usize,
    ) -> BackupCandidate {
        let file_name = format!(
            "opets-auto-{}-{collision}.osbkp",
            created_at.with_timezone(&Local).format("%Y%m%d-%H%M%S")
        );
        let path = directory.join(&file_name);
        fs::write(&path, b"backup").unwrap();
        BackupCandidate {
            path,
            created_at,
            record: OwnedBackupRecord {
                file_name,
                created_at: created_at.to_rfc3339(),
                size_bytes: 6,
                digest: String::new(),
                last_verified_at: None,
            },
        }
    }

    #[test]
    fn due_check_respects_enabled_state_and_next_date() {
        let now = Utc::now();
        let mut document = AutomaticBackupDocument::default();
        document.settings.enabled = true;
        document.settings.destination = Some("/backup".to_string());
        document.state.next_backup_at = Some((now - ChronoDuration::minutes(1)).to_rfc3339());
        assert!(is_due(&document, now));
        document.state.next_backup_at = Some((now + ChronoDuration::minutes(1)).to_rfc3339());
        assert!(!is_due(&document, now));
        document.settings.enabled = false;
        assert!(!is_due(&document, now));

        document.settings.enabled = true;
        document.state.next_backup_at = Some((now + ChronoDuration::days(30)).to_rfc3339());
        assert!(is_due(&document, now));
    }

    #[test]
    fn forced_run_is_rejected_while_scheduled_run_noops_when_disabled() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("database.db");
        let attachments = temp.path().join("database.attachments");
        let mut document = AutomaticBackupDocument::default();
        document.settings.enabled = false;
        document.state.source_id = Some(Uuid::new_v4().to_string());
        write_document(&config_path(&database).unwrap(), &document).unwrap();

        let error = run_with_paths(&database, &attachments, true, Utc::now(), None).unwrap_err();
        assert!(error.pt.contains("desativado"));

        let scheduled = run_with_paths(&database, &attachments, false, Utc::now(), None).unwrap();
        assert!(!scheduled.created);
        assert!(!scheduled.skipped_unchanged);
        assert!(scheduled.backup.is_none());
    }

    #[test]
    fn disabled_scheduler_waits_until_notified_while_enabled_sleeps_at_most_an_hour() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("database.db");

        let mut disabled = AutomaticBackupDocument::default();
        disabled.settings.enabled = false;
        write_document(&config_path(&database).unwrap(), &disabled).unwrap();
        assert_eq!(scheduler_wait_duration(&database), None);

        let mut enabled = AutomaticBackupDocument::default();
        enabled.settings.enabled = true;
        enabled.settings.destination = Some("/backup".to_string());
        enabled.state.next_backup_at = Some((Utc::now() + ChronoDuration::days(30)).to_rfc3339());
        write_document(&config_path(&database).unwrap(), &enabled).unwrap();
        let wait = scheduler_wait_duration(&database).unwrap();
        assert!(wait <= Duration::from_secs(60 * 60));

        enabled.state.next_backup_at = Some((Utc::now() + ChronoDuration::minutes(2)).to_rfc3339());
        write_document(&config_path(&database).unwrap(), &enabled).unwrap();
        let wait = scheduler_wait_duration(&database).unwrap();
        assert!(wait > Duration::from_secs(60) && wait <= Duration::from_secs(60 * 60));
    }

    #[test]
    fn retention_keeps_daily_weekly_and_two_newest_points() {
        let temp = tempfile::tempdir().unwrap();
        let mut candidates = Vec::new();
        for day in 1..=45 {
            let date = Utc::now() - ChronoDuration::days(day);
            candidates.push(create_candidate(temp.path(), date, day as usize));
        }
        let (delete, keep) = retention_paths_to_delete(candidates);
        assert!(keep.len() <= DAILY_RETENTION + WEEKLY_RETENTION + MINIMUM_RETENTION);
        assert!(keep.len() >= DAILY_RETENTION);
        assert_eq!(delete.len() + keep.len(), 45);
    }

    #[test]
    fn retention_ignores_manual_and_other_destination_backups() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp
            .path()
            .parent()
            .unwrap()
            .join(format!("outside-backup-{}.osbkp", Uuid::new_v4()));
        let manual = temp.path().join("manual.osbkp");
        let other = temp.path().join("opets-auto-20250101-120000.osbkp");
        fs::write(&outside, b"must not be removed").unwrap();
        fs::write(&manual, b"manual").unwrap();
        fs::write(&other, b"other installation").unwrap();
        let own = create_candidate(temp.path(), Utc::now(), 2);
        let unsafe_record = OwnedBackupRecord {
            file_name: format!("../{}", outside.file_name().unwrap().to_string_lossy()),
            created_at: Utc::now().to_rfc3339(),
            size_bytes: 19,
            digest: String::new(),
            last_verified_at: None,
        };

        prune_backups(temp.path(), vec![own.record, unsafe_record], None).unwrap();

        assert!(manual.exists());
        assert!(other.exists());
        assert!(outside.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn readable_filename_uses_a_numeric_suffix_only_on_collision() {
        let temp = tempfile::tempdir().unwrap();
        let now = Utc::now();
        let first = reserve_backup_destination(temp.path(), now).unwrap();
        let first_name = first
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        fs::write(&first.path, b"first").unwrap();
        drop(first);

        let second = reserve_backup_destination(temp.path(), now).unwrap();
        let second_name = second
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(first_name.starts_with("opets-auto-"));
        assert!(first_name.ends_with(".osbkp"));
        assert!(second_name.ends_with("-2.osbkp"));
    }

    #[test]
    fn destination_marker_detects_a_replaced_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("backups");
        let destination_id = ensure_destination_marker(&destination, None).unwrap();
        fs::remove_file(marker_path(&destination)).unwrap();

        let error = ensure_destination_marker(&destination, Some(&destination_id)).unwrap_err();

        assert!(error.en.contains("unavailable or was replaced"));
    }

    #[test]
    fn configuration_can_be_replaced_repeatedly_and_recovers_previous_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("database.automatic-backup.json");
        let mut document = AutomaticBackupDocument::default();
        write_document(&path, &document).unwrap();
        document.settings.interval_hours = 48;
        write_document(&path, &document).unwrap();
        assert_eq!(read_document(&path).unwrap().settings.interval_hours, 48);

        fs::write(&path, b"invalid configuration").unwrap();
        assert_eq!(read_document(&path).unwrap().settings.interval_hours, 24);
        assert!(path.exists());
    }

    #[test]
    fn configuration_migration_persists_a_missing_source_identity() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("database.db");
        let path = config_path(&database).unwrap();
        write_document(&path, &AutomaticBackupDocument::default()).unwrap();

        let migrated = load_document(&database).unwrap();

        assert!(migrated.state.source_id.is_some());
        assert_eq!(
            read_document(&path).unwrap().state.source_id,
            migrated.state.source_id
        );
    }

    #[test]
    fn legacy_configuration_moves_out_of_the_database_directory() {
        let temp = tempfile::tempdir().unwrap();
        let database_directory = temp.path().join("project-root");
        let app_data_directory = temp.path().join("app-data").join("automatic-backup");
        fs::create_dir_all(&database_directory).unwrap();
        fs::create_dir_all(&app_data_directory).unwrap();
        let database = database_directory.join("database.db");
        let legacy = legacy_config_path(&database).unwrap();
        let mut document = AutomaticBackupDocument::default();
        document.settings.interval_hours = 48;
        write_document(&legacy, &document).unwrap();

        migrate_legacy_configuration(&database, &app_data_directory).unwrap();

        assert!(!legacy.exists());
        assert_eq!(
            read_document(&app_data_directory.join("database.automatic-backup.json"))
                .unwrap()
                .settings
                .interval_hours,
            48
        );
    }

    #[test]
    fn rejects_destination_inside_attachment_storage() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("database.db");
        let attachments = temp.path().join("database.attachments");
        let destination = attachments.join("backups");
        fs::write(&database, b"database").unwrap();
        fs::create_dir_all(&attachments).unwrap();

        let error = validate_destination_path(&destination, &database, &attachments).unwrap_err();

        assert!(error.en.contains("cannot overlap"));
    }

    #[test]
    fn unchanged_sources_skip_a_second_scheduled_backup() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("database.db");
        let attachments = temp.path().join("database.attachments");
        let destination = temp.path().join("backups");
        fs::create_dir_all(&attachments).unwrap();
        fs::write(
            attachments.join("unmanaged.tmp"),
            b"not referenced by database",
        )
        .unwrap();
        let conn = Connection::open(&database).unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO customers (id, name) VALUES ('customer-1', 'Cliente Backup')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO service_orders (id, customer_id, equipment, description, display_id) VALUES ('order-1', 'customer-1', 'Notebook', 'Teste', 'OS-1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO service_order_attachments (id, service_order_id, file_name, storage_name, mime_type, size_bytes) VALUES ('attachment-1', 'order-1', 'evidencia.png', 'stored-attachment', 'image/png', 8)",
            [],
        )
        .unwrap();
        drop(conn);
        fs::write(attachments.join("stored-attachment"), b"\x89PNG\r\n\x1a\n").unwrap();
        let destination_id = ensure_destination_marker(&destination, None).unwrap();
        let document = AutomaticBackupDocument {
            format_version: CONFIG_FORMAT_VERSION,
            settings: AutomaticBackupSettings {
                enabled: true,
                destination: Some(destination.to_string_lossy().to_string()),
                interval_hours: 24,
            },
            state: AutomaticBackupState {
                source_id: Some(Uuid::new_v4().to_string()),
                destination_id: Some(destination_id),
                next_backup_at: Some(Utc::now().to_rfc3339()),
                ..AutomaticBackupState::default()
            },
        };
        write_document(&config_path(&database).unwrap(), &document).unwrap();
        let first_time = Utc::now();
        let first = run_with_paths(&database, &attachments, false, first_time, None).unwrap();
        assert!(first.created);
        let first_backup = first.backup.unwrap();
        assert_eq!(first_backup.attachment_count, 1);
        let first_path = PathBuf::from(first_backup.path);

        let restore_dir = temp.path().join("restore");
        fs::create_dir_all(&restore_dir).unwrap();
        let restore_database = restore_dir.join("database.db");
        let restore_attachments = restore_dir.join("database.attachments");
        let guard = crate::database::exclusive_storage_guard().unwrap();
        backup_service::restore_backup_with_passphrase(
            &first_path,
            &restore_database,
            &restore_attachments,
            None,
            &guard,
        )
        .unwrap();
        drop(guard);
        let restored_attachment = crate::models::service_order_attachment::ServiceOrderAttachment {
            id: "attachment-1".to_string(),
            service_order_id: "order-1".to_string(),
            file_name: "evidencia.png".to_string(),
            storage_name: "stored-attachment".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: 8,
            created_at: String::new(),
        };
        assert_eq!(
            crate::attachment_service::read_attachment_as_data_url_with_paths(
                &restored_attachment,
                &restore_attachments,
            )
            .unwrap(),
            "data:image/png;base64,iVBORw0KGgo="
        );

        let second = run_with_paths(
            &database,
            &attachments,
            true,
            first_time + ChronoDuration::hours(24),
            None,
        )
        .unwrap();
        assert!(second.skipped_unchanged);
        assert_eq!(
            fs::read_dir(&destination)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .path()
                    .extension()
                    .is_some_and(|value| value == "osbkp"))
                .count(),
            1
        );

        fs::write(&first_path, b"corrupted backup").unwrap();
        let replacement = run_with_paths(
            &database,
            &attachments,
            true,
            first_time + ChronoDuration::hours(48),
            None,
        )
        .unwrap();
        assert!(replacement.created);
        assert!(!first_path.exists());

        let connection = Connection::open(&database).unwrap();
        connection
            .execute(
                "UPDATE settings SET company_name = 'Dados alterados' WHERE id = 1",
                [],
            )
            .unwrap();
        drop(connection);
        let changed = run_with_paths(
            &database,
            &attachments,
            true,
            first_time + ChronoDuration::hours(72),
            None,
        )
        .unwrap();
        assert!(changed.created);
        assert_eq!(
            fs::read_dir(&destination)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .path()
                    .extension()
                    .is_some_and(|value| value == "osbkp"))
                .count(),
            2
        );

        let state = read_document(&config_path(&database).unwrap()).unwrap();
        let latest = PathBuf::from(state.state.last_backup_path.unwrap());
        let retained = state
            .state
            .owned_backups
            .iter()
            .map(|record| destination.join(&record.file_name))
            .find(|path| path != &latest)
            .unwrap();
        fs::write(&retained, b"corrupted retained backup").unwrap();
        let recovered = run_with_paths(
            &database,
            &attachments,
            true,
            first_time + ChronoDuration::hours(96),
            None,
        )
        .unwrap();
        assert!(recovered.created);
        assert!(!retained.exists());
    }
}
