use crate::backup_service::{self, BackupSummary};
use crate::error::AppError;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
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
    last_error: Option<String>,
    last_backup_path: Option<String>,
    last_backup_size_bytes: Option<u64>,
    last_backup_digest: Option<String>,
    source_fingerprint: Option<String>,
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

#[derive(Debug)]
struct BackupCandidate {
    path: PathBuf,
    created_at: DateTime<Utc>,
}

struct PruneResult {
    removed: usize,
    warning: Option<String>,
}

fn automatic_backup_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        format!("Automatic backup failed: {error}"),
        format!("O backup automático falhou: {error}"),
    )
}

fn config_path(database_path: &Path) -> Result<PathBuf, AppError> {
    let parent = database_path
        .parent()
        .ok_or_else(|| automatic_backup_error("Database path has no parent directory."))?;
    Ok(parent.join("database.automatic-backup.json"))
}

fn read_document(path: &Path) -> Result<AutomaticBackupDocument, AppError> {
    let previous = previous_config_path(path);
    if !path.exists() && previous.exists() {
        fs::rename(&previous, path).map_err(automatic_backup_error)?;
    }
    if !path.exists() {
        return Ok(AutomaticBackupDocument::default());
    }
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
        if had_previous {
            let _ = fs::remove_file(previous);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn load_document(database_path: &Path) -> Result<AutomaticBackupDocument, AppError> {
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| automatic_backup_error("Configuration lock is unavailable."))?;
    let path = config_path(database_path)?;
    let mut document = read_document(&path)?;
    if document.state.source_id.is_none() {
        document.state.source_id = Some(Uuid::new_v4().to_string());
        write_document(&path, &document)?;
    }
    Ok(document)
}

fn save_document(database_path: &Path, document: &AutomaticBackupDocument) -> Result<(), AppError> {
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| automatic_backup_error("Configuration lock is unavailable."))?;
    write_document(&config_path(database_path)?, document)
}

fn mutate_document<T>(
    database_path: &Path,
    mutation: impl FnOnce(&mut AutomaticBackupDocument) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| automatic_backup_error("Configuration lock is unavailable."))?;
    let path = config_path(database_path)?;
    let mut document = read_document(&path)?;
    let result = mutation(&mut document)?;
    write_document(&path, &document)?;
    Ok(result)
}

fn status_from_document(document: AutomaticBackupDocument) -> AutomaticBackupStatus {
    let runtime = RUNTIME_STATUS
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default();
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
        fs::rename(&temporary, &path).map_err(automatic_backup_error)?;
        Ok(())
    })();
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
        .map_err(|_| automatic_backup_error("Automatic backup operation lock is unavailable."))?;
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
    if let Some(destination) = destination.as_deref() {
        validate_destination_path(
            destination,
            &database_path,
            &crate::database::attachments_dir(),
        )?;
    }
    let previous = load_document(&database_path)?;
    let normalized_destination = destination
        .as_ref()
        .map(|value| value.to_string_lossy().to_string());
    let destination_changed = previous.settings.destination != normalized_destination;
    let interval_changed = previous.settings.interval_hours != settings.interval_hours;
    let destination_id = match destination.as_deref() {
        Some(path) => ensure_destination_marker(
            path,
            if destination_changed {
                None
            } else {
                previous.state.destination_id.as_deref()
            },
        )
        .map(Some)?,
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
        document.state.last_backup_path = None;
        document.state.last_backup_size_bytes = None;
        document.state.last_backup_digest = None;
    }
    if document.settings.enabled {
        if enabling
            || destination_changed
            || interval_changed
            || document.state.next_backup_at.is_none()
        {
            document.state.next_backup_at = Some(now.to_rfc3339());
        }
    } else {
        document.state.next_backup_at = None;
    }
    save_document(&database_path, &document)?;
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
    if let Ok(mut status) = RUNTIME_STATUS.lock() {
        status.running = percent < 100;
        status.progress_percent = percent;
        status.phase = (percent < 100).then(|| phase.to_string());
    }
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
    parse_timestamp(document.state.last_verified_at.as_deref()).is_none_or(|last_verified| {
        last_verified <= now - ChronoDuration::days(FULL_VALIDATION_INTERVAL_DAYS)
            || last_verified > now + ChronoDuration::hours(1)
    })
}

fn source_size(database_path: &Path, attachments_path: &Path) -> Result<u64, AppError> {
    let mut size = fs::metadata(database_path)
        .map_err(automatic_backup_error)?
        .len();
    if attachments_path.exists() {
        for entry in fs::read_dir(attachments_path).map_err(automatic_backup_error)? {
            let metadata = entry.map_err(automatic_backup_error)?.metadata();
            if let Ok(metadata) = metadata {
                if metadata.is_file() {
                    size = size.saturating_add(metadata.len());
                }
            }
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

fn ensure_staging_capacity(database_path: &Path, source_bytes: u64) -> Result<(), AppError> {
    let parent = database_path
        .parent()
        .ok_or_else(|| automatic_backup_error("Database path has no parent directory."))?;
    let required = source_bytes.saturating_add(DISK_SAFETY_MARGIN_BYTES);
    let available = fs2::available_space(parent).map_err(automatic_backup_error)?;
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

fn destination_prefix(destination_id: &str) -> String {
    format!("opets-auto-{destination_id}-")
}

fn backup_destination(destination: &Path, destination_id: &str, now: DateTime<Utc>) -> PathBuf {
    destination.join(format!(
        "{}{}-{}.osbkp",
        destination_prefix(destination_id),
        now.format("%Y%m%d-%H%M%S"),
        Uuid::new_v4()
    ))
}

fn parse_candidate(path: PathBuf, destination_id: &str) -> Option<BackupCandidate> {
    let name = path.file_name()?.to_str()?;
    let timestamp = name
        .strip_prefix(&destination_prefix(destination_id))?
        .get(..15)?;
    let created_at = NaiveDateTime::parse_from_str(timestamp, "%Y%m%d-%H%M%S")
        .ok()?
        .and_utc();
    let metadata = fs::symlink_metadata(&path).ok()?;
    (metadata.file_type().is_file() && name.ends_with(".osbkp"))
        .then_some(BackupCandidate { path, created_at })
}

fn retention_paths_to_delete(
    mut candidates: Vec<BackupCandidate>,
) -> (Vec<PathBuf>, HashSet<PathBuf>) {
    candidates.sort_by(|left, right| right.created_at.cmp(&left.created_at));
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
    source_id: &str,
    protected: Option<&Path>,
) -> Result<PruneResult, AppError> {
    let candidates = fs::read_dir(destination)
        .map_err(automatic_backup_error)?
        .filter_map(Result::ok)
        .filter_map(|entry| parse_candidate(entry.path(), source_id))
        .collect();
    let (delete, _) = retention_paths_to_delete(candidates);
    let mut removed = 0;
    let mut errors = Vec::new();
    for path in delete {
        if protected.is_some_and(|protected| protected == path) {
            continue;
        }
        match fs::remove_file(path) {
            Ok(()) => removed += 1,
            Err(error) => errors.push(error.to_string()),
        }
    }
    Ok(PruneResult {
        removed,
        warning: (!errors.is_empty()).then(|| {
            format!(
                "Não foi possível remover {} backup(s) antigo(s): {}",
                errors.len(),
                errors.join("; ")
            )
        }),
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
    if !force && !is_due(&initial, now) {
        return Ok(AutomaticBackupRunResult {
            created: false,
            skipped_unchanged: false,
            pruned_count: 0,
            backup: None,
        });
    }
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
    let source_id =
        initial.state.source_id.as_deref().ok_or_else(|| {
            automatic_backup_error("Automatic backup source identity is missing.")
        })?;
    ensure_destination_marker(&destination, Some(destination_id))?;

    emit_progress(app, 5, "preparing", "Preparando o backup automático.");
    let storage_guard = crate::database::exclusive_storage_guard()?;
    emit_progress(app, 15, "checking", "Verificando alterações nos dados.");
    let fingerprint = source_fingerprint(database_path, attachments_path)?;
    let mut invalid_previous_backup = None;
    let latest_backup_exists = initial
        .state
        .last_backup_path
        .as_deref()
        .map(Path::new)
        .and_then(|path| fs::symlink_metadata(path).ok())
        .is_some_and(|metadata| metadata.file_type().is_file());
    let (storage_guard, fingerprint) = if latest_backup_exists
        && initial.state.source_fingerprint.as_deref() == Some(&fingerprint)
    {
        drop(storage_guard);
        let latest_backup = Path::new(
            initial
                .state
                .last_backup_path
                .as_deref()
                .unwrap_or_default(),
        );
        emit_progress(app, 70, "validating", "Validando o backup mais recente.");
        let digest = file_digest(latest_backup).ok();
        let digest_matches = digest.as_deref() == initial.state.last_backup_digest.as_deref()
            && fs::metadata(latest_backup)
                .ok()
                .map(|metadata| metadata.len())
                == initial.state.last_backup_size_bytes;
        let requires_full_validation = full_validation_is_due(&initial, now);
        let backup_is_valid = digest_matches
            && (!requires_full_validation
                || backup_service::validate_backup_contents_with_passphrase(latest_backup, None)
                    .is_ok());
        if backup_is_valid {
            let current_guard = crate::database::exclusive_storage_guard()?;
            let current_fingerprint = source_fingerprint(database_path, attachments_path)?;
            if current_fingerprint == fingerprint {
                drop(current_guard);
                let (pruned_count, retention_warning) =
                    match prune_backups(&destination, source_id, Some(latest_backup)) {
                        Ok(result) => (result.removed, result.warning),
                        Err(error) => (0, Some(error.pt)),
                    };
                mutate_document(database_path, |document| {
                    document.state.last_attempt_at = Some(now.to_rfc3339());
                    if requires_full_validation {
                        document.state.last_verified_at = Some(now.to_rfc3339());
                    }
                    document.state.last_error = retention_warning;
                    document.state.next_backup_at = Some(
                        (now + ChronoDuration::hours(document.settings.interval_hours as i64))
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
                    pruned_count,
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
            invalid_previous_backup = Some(latest_backup.to_path_buf());
            let current_guard = crate::database::exclusive_storage_guard()?;
            let current_fingerprint = source_fingerprint(database_path, attachments_path)?;
            (current_guard, current_fingerprint)
        }
    } else {
        (storage_guard, fingerprint)
    };

    let source_bytes = source_size(database_path, attachments_path)?;
    ensure_disk_capacity(&destination, source_bytes)?;
    ensure_staging_capacity(database_path, source_bytes)?;
    emit_progress(
        app,
        25,
        "snapshot",
        "Criando uma cópia consistente dos dados.",
    );
    let prepared = backup_service::prepare_backup_sources(database_path, attachments_path)?;
    drop(storage_guard);
    let backup_path = backup_destination(&destination, source_id, now);
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
    emit_progress(app, 95, "retention", "Aplicando a retenção de backups.");
    let (pruned_count, retention_warning) =
        match prune_backups(&destination, source_id, Some(&backup_path)) {
            Ok(result) => (result.removed, result.warning),
            Err(error) => {
                eprintln!("[BACKUP] Retention cleanup failed: {}", error.en);
                (0, Some(error.pt))
            }
        };
    mutate_document(database_path, |document| {
        document.state.last_attempt_at = Some(now.to_rfc3339());
        document.state.last_success_at = Some(now.to_rfc3339());
        document.state.last_verified_at = Some(now.to_rfc3339());
        document.state.last_error = retention_warning;
        document.state.last_backup_path = Some(summary.path.clone());
        document.state.last_backup_size_bytes = Some(backup_size);
        document.state.last_backup_digest = Some(backup_digest);
        document.state.source_fingerprint = Some(fingerprint);
        if document.settings.enabled {
            document.state.next_backup_at = Some(
                (now + ChronoDuration::hours(document.settings.interval_hours as i64)).to_rfc3339(),
            );
        }
        Ok(())
    })?;
    if let Some(invalid_previous_backup) = invalid_previous_backup {
        let _ = fs::remove_file(invalid_previous_backup);
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
    let _operation = OPERATION_LOCK.try_lock().map_err(|_| {
        AppError::new(
            "An automatic backup is already running.",
            "Um backup automático já está em andamento.",
        )
    })?;
    let database_path = crate::database::database_path();
    let now = Utc::now();
    let result = run_with_paths(
        &database_path,
        &crate::database::attachments_dir(),
        force,
        now,
        app,
    );
    if let Err(error) = &result {
        record_failure(&database_path, now, error);
        emit_progress(app, 100, "failed", &error.pt);
    }
    result
}

fn cleanup_stale_temporary_files(database_path: &Path) -> std::io::Result<()> {
    let Some(parent) = database_path.parent() else {
        return Ok(());
    };
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let path = entry.path();
        if name.starts_with(".opets-backup-staging-") && path.is_dir() {
            fs::remove_dir_all(path)?;
        } else if path.is_file()
            && (name.starts_with(".opet-snapshot-")
                || name.starts_with(".opets-backup-archive-")
                || name.starts_with(".opets-backup-encrypted-")
                || name.starts_with(".opet-backup-")
                || name.starts_with(".opets-backup-validation-")
                || (name.starts_with(".automatic-backup-") && name.ends_with(".tmp")))
        {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub fn start_scheduler(app: AppHandle) -> std::io::Result<()> {
    cleanup_stale_temporary_files(&crate::database::database_path())?;
    std::thread::Builder::new()
        .name("opets-automatic-backup".to_string())
        .spawn(move || loop {
            if let Err(error) = run(false, Some(&app)) {
                eprintln!("[BACKUP] {}", error.en);
            }
            std::thread::sleep(Duration::from_secs(60 * 60));
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::run_migrations;
    use rusqlite::Connection;

    fn create_candidate(directory: &Path, destination_id: &str, timestamp: &str) -> PathBuf {
        let path = directory.join(format!(
            "{}{}-{}.osbkp",
            destination_prefix(destination_id),
            timestamp,
            Uuid::new_v4()
        ));
        fs::write(&path, b"backup").unwrap();
        path
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
    fn retention_keeps_daily_weekly_and_two_newest_points() {
        let temp = tempfile::tempdir().unwrap();
        let destination_id = "12345678-aaaa-bbbb-cccc-dddddddddddd";
        let mut candidates = Vec::new();
        for day in 1..=45 {
            let date = Utc::now() - ChronoDuration::days(day);
            let path = create_candidate(
                temp.path(),
                destination_id,
                &date.format("%Y%m%d-%H%M%S").to_string(),
            );
            candidates.push(parse_candidate(path, destination_id).unwrap());
        }
        let (delete, keep) = retention_paths_to_delete(candidates);
        assert!(keep.len() <= DAILY_RETENTION + WEEKLY_RETENTION + MINIMUM_RETENTION);
        assert!(keep.len() >= DAILY_RETENTION);
        assert_eq!(delete.len() + keep.len(), 45);
    }

    #[test]
    fn retention_ignores_manual_and_other_destination_backups() {
        let temp = tempfile::tempdir().unwrap();
        let own_id = "12345678-aaaa-bbbb-cccc-dddddddddddd";
        let other_id = "87654321-aaaa-bbbb-cccc-dddddddddddd";
        let manual = temp.path().join("manual.osbkp");
        fs::write(&manual, b"manual").unwrap();
        create_candidate(temp.path(), other_id, "20250101-120000");
        create_candidate(temp.path(), own_id, "20250101-120000");

        prune_backups(temp.path(), own_id, None).unwrap();

        assert!(manual.exists());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
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

        fs::rename(&path, previous_config_path(&path)).unwrap();
        assert_eq!(read_document(&path).unwrap().settings.interval_hours, 48);
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
        drop(conn);
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
        assert_eq!(first_backup.attachment_count, 0);
        let first_path = PathBuf::from(first_backup.path);

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
    }
}
