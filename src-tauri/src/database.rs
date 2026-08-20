use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::ops::{Deref, DerefMut};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tauri::Manager;
use uuid::Uuid;

// Static connection pool for simple desktop usage
static DB_PATH: OnceCell<PathBuf> = OnceCell::new();
static APP_DATA_DIR: OnceCell<PathBuf> = OnceCell::new();
static STORAGE_INSTANCE_LOCK: OnceCell<File> = OnceCell::new();
static STORAGE_OPERATION_LOCK: LazyLock<RwLock<()>> = LazyLock::new(|| RwLock::new(()));
// Whether the user opted into sharing the storage on a LAN/network folder. Set
// from the persisted storage config during app initialization; defaults to false.
static LAN_SHARED_MODE: OnceCell<bool> = OnceCell::new();
static LAN_IS_HOST: OnceCell<bool> = OnceCell::new();

// Single shared SQLCipher connection, reused across requests. Reopening the
// connection on every call was the dominant fixed cost (~2.5ms/request: key
// derivation, page cache warmup). Closed before a restore replaces the file.
static DB_CONNECTION: LazyLock<Mutex<Option<Connection>>> = LazyLock::new(|| Mutex::new(None));

pub(crate) type ExclusiveStorageGuard = RwLockWriteGuard<'static, ()>;

const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";
const STORAGE_FORMAT_VERSION: u8 = 1;
const LAN_MANIFEST_MAGIC: &[u8] = b"OPETLAN1";

pub struct DatabaseConnection {
    connection: MutexGuard<'static, Option<Connection>>,
    _guard: RwLockReadGuard<'static, ()>,
}

impl Deref for DatabaseConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.connection
            .as_ref()
            .expect("database connection must be initialized")
    }
}

impl DerefMut for DatabaseConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.connection
            .as_mut()
            .expect("database connection must be initialized")
    }
}

/// Closes the shared connection before an exclusive storage operation replaces
/// the database file. The caller must hold the exclusive storage guard.
pub(crate) fn close_shared_connection(
    database_path: &Path,
    _storage_guard: &ExclusiveStorageGuard,
) -> Result<()> {
    if DB_PATH
        .get()
        .is_none_or(|active_path| active_path != database_path)
    {
        return Ok(());
    }
    let mut shared = DB_CONNECTION
        .lock()
        .map_err(|_| database_error("Database connection lock is unavailable."))?;
    let Some(connection) = shared.take() else {
        return Ok(());
    };
    if let Err((connection, error)) = connection.close() {
        *shared = Some(connection);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn shared_connection_is_open() -> bool {
    DB_CONNECTION
        .lock()
        .map(|connection| connection.is_some())
        .unwrap_or(false)
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageMetadata {
    format_version: u8,
    key_version: u8,
    authentication: String,
}

/// Persisted storage preferences: the database folder chosen by the user and the
/// LAN shared-mode toggle. Stored outside the database (in the application data
/// directory) so the database file itself can live on a network share. Applied on
/// the next application start.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfig {
    #[serde(default)]
    pub database_path: Option<PathBuf>,
    #[serde(default)]
    pub lan_shared: bool,
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LanManifest {
    format_version: u8,
    host_device_id: String,
    app_version: String,
    generation: u64,
    authentication: String,
}

fn lan_manifest_path(database_path: &Path) -> PathBuf {
    database_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".opets-lan.json")
}
fn lan_manifest_auth(host_device_id: &str, app_version: &str, generation: u64) -> String {
    crate::encryption::metadata_authentication(&format!(
        "lan:1:{host_device_id}:{app_version}:{generation}"
    ))
}
fn read_lan_manifest(database_path: &Path) -> Result<Option<LanManifest>> {
    let path = lan_manifest_path(database_path);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(io_error)?;
    if !bytes.starts_with(LAN_MANIFEST_MAGIC) || bytes.len() <= LAN_MANIFEST_MAGIC.len() + 24 {
        return Err(database_error("Manifesto LAN inválido ou corrompido."));
    }
    let cipher = XChaCha20Poly1305::new(
        (&crate::encryption::derive_key("com.walk.tcc-opet/lan-manifest/v1")).into(),
    );
    let payload = cipher
        .decrypt(
            XNonce::from_slice(&bytes[LAN_MANIFEST_MAGIC.len()..LAN_MANIFEST_MAGIC.len() + 24]),
            &bytes[LAN_MANIFEST_MAGIC.len() + 24..],
        )
        .map_err(|_| database_error("Manifesto LAN não pôde ser autenticado."))?;
    let manifest: LanManifest = serde_json::from_slice(&payload).map_err(database_error)?;
    if manifest.format_version != 1
        || manifest.app_version != env!("CARGO_PKG_VERSION")
        || manifest.authentication
            != lan_manifest_auth(
                &manifest.host_device_id,
                &manifest.app_version,
                manifest.generation,
            )
    {
        return Err(database_error("Manifesto LAN inválido ou incompatível. Atualize todos os computadores para a mesma versão."));
    }
    Ok(Some(manifest))
}
fn write_lan_manifest(database_path: &Path, host_device_id: String, generation: u64) -> Result<()> {
    let manifest = LanManifest {
        format_version: 1,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        authentication: lan_manifest_auth(&host_device_id, env!("CARGO_PKG_VERSION"), generation),
        host_device_id,
        generation,
    };
    let path = lan_manifest_path(database_path);
    let temp = path.with_extension("json.tmp");
    let cipher = XChaCha20Poly1305::new(
        (&crate::encryption::derive_key("com.walk.tcc-opet/lan-manifest/v1")).into(),
    );
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            serde_json::to_vec(&manifest)
                .map_err(database_error)?
                .as_ref(),
        )
        .map_err(|_| database_error("Não foi possível cifrar o manifesto LAN."))?;
    let mut bytes = LAN_MANIFEST_MAGIC.to_vec();
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&ciphertext);
    fs::write(&temp, bytes).map_err(io_error)?;
    fs::rename(temp, path).map_err(io_error)
}

pub fn storage_config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("app_config.json")
}

pub fn load_storage_config(app_data_dir: &Path) -> StorageConfig {
    let path = storage_config_path(app_data_dir);
    fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_storage_config(app_data_dir: &Path, config: &StorageConfig) -> Result<()> {
    fs::create_dir_all(app_data_dir).map_err(io_error)?;
    let path = storage_config_path(app_data_dir);
    let temporary = path.with_extension("app_config.json.tmp");
    if let Err(error) = fs::write(
        &temporary,
        serde_json::to_vec_pretty(config).map_err(database_error)?,
    ) {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(error));
    }
    fs::rename(&temporary, &path).map_err(io_error)?;
    Ok(())
}

pub(crate) fn lan_shared_mode() -> bool {
    LAN_SHARED_MODE.get().copied().unwrap_or(false)
}
pub(crate) fn lan_is_host() -> bool {
    LAN_IS_HOST.get().copied().unwrap_or(false)
}
pub(crate) fn advance_lan_generation() -> Result<()> {
    if !lan_shared_mode() || !lan_is_host() {
        return Ok(());
    }
    let config = load_storage_config(&app_data_dir());
    write_lan_manifest(
        &database_path(),
        config.device_id.unwrap_or_default(),
        read_lan_manifest(&database_path())?
            .map(|m| m.generation + 1)
            .unwrap_or(1),
    )
}
pub(crate) fn lan_generation() -> Result<Option<u64>> {
    read_lan_manifest(&database_path()).map(|manifest| manifest.map(|value| value.generation))
}

fn set_lan_shared_mode(lan_shared: bool) -> Result<()> {
    LAN_SHARED_MODE
        .set(lan_shared)
        .map_err(|_| database_error("Shared storage mode was already initialized."))
}

#[cfg(test)]
pub(crate) fn set_lan_shared_mode_for_tests(lan_shared: bool) {
    // Mirror the real startup order (config is loaded before init), so stress
    // tests can make every connection in the process behave like a LAN client.
    // Idempotent: several ignored tests in the same process can call it.
    if lan_shared_mode() == lan_shared {
        return;
    }
    if let Err(error) = set_lan_shared_mode(lan_shared) {
        panic!("lan_shared_mode already initialized in this test process: {error}");
    }
}

// Initialize the database connection
pub fn init_db(app: &tauri::App) -> Result<()> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error)))
    })?;
    let storage_config = load_storage_config(&app_data_dir);
    let resolved_database_path = get_database_path(&app_data_dir, &storage_config)?;
    let manifest = read_lan_manifest(&resolved_database_path)?;
    let lan_shared = storage_config.lan_shared || manifest.is_some();
    set_lan_shared_mode(lan_shared)?;
    let device_id = storage_config.device_id.as_deref().unwrap_or_default();
    LAN_IS_HOST
        .set(
            manifest
                .as_ref()
                .is_some_and(|m| m.host_device_id == device_id)
                || (storage_config.lan_shared && manifest.is_none()),
        )
        .map_err(|_| database_error("LAN role already initialized."))?;
    if let Some(parent) = resolved_database_path.parent() {
        ensure_private_dir(parent).map_err(io_error)?;
    }
    if !lan_shared {
        acquire_storage_instance_lock(&resolved_database_path)?;
    }
    initialize_storage_at(&resolved_database_path, should_seed_demo_data())?;
    if storage_config.lan_shared && manifest.is_none() {
        write_lan_manifest(&resolved_database_path, device_id.to_string(), 0)?;
    }
    APP_DATA_DIR.set(app_data_dir).map_err(|_| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(
            "Application data path was already initialized.",
        )))
    })?;
    DB_PATH.set(resolved_database_path).map_err(|_| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(
            "Database path was already initialized.",
        )))
    })
}

fn storage_instance_lock_path(database_path: &Path) -> PathBuf {
    let mut path = database_path.to_path_buf();
    path.set_extension("lock");
    path
}

fn open_storage_instance_lock(database_path: &Path) -> Result<File> {
    let path = storage_instance_lock_path(database_path);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(io_error)?;
    fs2::FileExt::try_lock_exclusive(&file).map_err(|error| {
        database_error(format!(
            "Another application instance is already using this storage: {error}"
        ))
    })?;
    secure_private_file(&path).map_err(io_error)?;
    Ok(file)
}

fn acquire_storage_instance_lock(database_path: &Path) -> Result<()> {
    let file = open_storage_instance_lock(database_path)?;
    STORAGE_INSTANCE_LOCK
        .set(file)
        .map_err(|_| database_error("Application storage instance lock was already initialized."))
}

pub(crate) fn initialize_storage_at(database_path: &Path, seed_demo_data: bool) -> Result<()> {
    let attachments_path = attachments_dir_for(database_path);
    if let Some(parent) = database_path.parent() {
        ensure_private_dir(parent).map_err(io_error)?;
    }
    write_or_validate_storage_metadata_at(database_path)?;

    let recovery_backup = if is_plaintext_database(database_path).map_err(io_error)? {
        Some(create_pre_encryption_recovery_backup(
            database_path,
            &attachments_path,
        )?)
    } else {
        None
    };
    let legacy_database = if recovery_backup.is_some() {
        Some(migrate_plaintext_database(database_path)?)
    } else {
        None
    };

    // Diagnose lock-broken transports (e.g. Linux CIFS + SMB2/3) BEFORE any
    // connection activates WAL, so the readable error wins over a 30s SQLITE_BUSY.
    probe_network_share_locking(database_path)?;

    // Open the connection once to run migrations with foreign keys enabled.
    let conn = open_encrypted_database(database_path)?;
    run_migrations(&conn)?;
    crate::attachment_service::recover_staged_attachment_deletions(&conn, &attachments_path)
        .map_err(database_error)?;
    crate::attachment_service::migrate_legacy_attachments(&conn, &attachments_path)
        .map_err(database_error)?;
    secure_private_file(database_path).map_err(io_error)?;

    if seed_demo_data {
        crate::seeds::initialize_seed_data_with_conn(&conn).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
        })?;
    } else if cfg!(debug_assertions) {
        println!("[SEED] Demo seed data skipped by SKIP_DB_SEED.");
    } else {
        println!("[SEED] Demo seed data skipped in production.");
    }
    drop(conn);

    if let Some(path) = legacy_database {
        let _ = fs::remove_file(path);
    }
    if let Some(path) = recovery_backup {
        println!(
            "[MIGRATION] Encrypted pre-migration recovery backup created at {}.",
            path.display()
        );
    }

    Ok(())
}

fn io_error(error: io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn database_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error.to_string())))
}

fn create_pre_encryption_recovery_backup(
    database_path: &Path,
    attachments_path: &Path,
) -> Result<PathBuf> {
    let parent = database_path
        .parent()
        .ok_or_else(|| database_error("Database path has no parent."))?;
    let destination = parent.join(format!(
        "opets-pre-encryption-v0.1.0-{}.osbkp",
        Uuid::new_v4()
    ));
    crate::backup_service::export_backup_with_passphrase(
        database_path,
        attachments_path,
        &destination,
        None,
    )
    .map_err(database_error)?;
    Ok(destination)
}

pub(crate) fn is_plaintext_database(path: &Path) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let bytes = fs::read(path)?;
    Ok(bytes.starts_with(SQLITE_HEADER))
}

fn storage_metadata_path_for(database_path: &Path) -> PathBuf {
    let mut path = database_path.to_path_buf();
    path.set_extension("encryption.json");
    path
}

fn metadata_payload(format_version: u8, key_version: u8) -> String {
    format!("{format_version}:{key_version}")
}

fn write_or_validate_storage_metadata_at(database_path: &Path) -> Result<()> {
    let path = storage_metadata_path_for(database_path);
    let expected_authentication = crate::encryption::metadata_authentication(&metadata_payload(
        STORAGE_FORMAT_VERSION,
        crate::encryption::ACTIVE_KEY_VERSION,
    ));
    if path.exists() {
        let metadata: StorageMetadata =
            serde_json::from_slice(&fs::read(&path).map_err(io_error)?).map_err(database_error)?;
        if metadata.format_version != STORAGE_FORMAT_VERSION
            || metadata.key_version != crate::encryption::ACTIVE_KEY_VERSION
            || metadata.authentication != expected_authentication
        {
            return Err(database_error(
                "Unsupported or invalid encrypted storage metadata.",
            ));
        }
        return Ok(());
    }

    let metadata = StorageMetadata {
        format_version: STORAGE_FORMAT_VERSION,
        key_version: crate::encryption::ACTIVE_KEY_VERSION,
        authentication: expected_authentication,
    };
    fs::write(
        &path,
        serde_json::to_vec(&metadata).map_err(database_error)?,
    )
    .map_err(io_error)?;
    secure_private_file(&path).map_err(io_error)
}

fn database_key_hex() -> String {
    crate::encryption::database_key()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn open_encrypted_database(path: &Path) -> Result<Connection> {
    open_encrypted_database_with_mode(path, lan_shared_mode())
}

/// SQLite WAL is the product default and works on every transport that provides
/// byte-range locking (local disks, NFS with lockd, SMB1+unix, Windows/macOS SMB
/// clients). A lock-broken transport (Linux CIFS + SMB2/3) is detected at startup
/// by `probe_network_share_locking`, which fails fast with a readable
/// message instead of a cryptic `database is locked`.
#[cfg(target_os = "linux")]
fn filesystem_is_network_share(path: &Path) -> bool {
    let mut probe = path.to_path_buf();
    loop {
        if probe.exists() {
            break;
        }
        if !probe.pop() {
            break;
        }
    }
    let canonical = fs::canonicalize(&probe).unwrap_or(probe);
    let target = canonical.to_string_lossy().to_string();
    let Ok(mounts) = fs::read_to_string("/proc/self/mounts") else {
        return false;
    };
    let mut best = "";
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let _device = fields.next();
        let Some(mount_point) = fields.next() else {
            continue;
        };
        let Some(fs_type) = fields.next() else {
            continue;
        };
        if !(fs_type.starts_with("nfs")
            || fs_type.starts_with("cifs")
            || fs_type.starts_with("smb"))
        {
            continue;
        }
        if target.starts_with(mount_point) && mount_point.len() > best.len() {
            best = mount_point;
        }
    }
    !best.is_empty()
}

#[cfg(not(target_os = "linux"))]
fn filesystem_is_network_share(_path: &Path) -> bool {
    false
}

fn probe_tables_work(conn: &Connection) -> std::result::Result<(), rusqlite::Error> {
    // A zero-concurrency write must succeed on any share SQLite can host. On
    // lock-broken transports (e.g. Linux CIFS + SMB2/3) even a plain CREATE
    // fails instantly with SQLITE_BUSY; fail fast here with a clear message.
    conn.execute_batch("CREATE TABLE IF NOT EXISTS __opets_share_lock_probe__ (id INTEGER);")?;
    conn.execute_batch("DROP TABLE IF EXISTS __opets_share_lock_probe__;")
}

/// Runs once at startup on LAN network shares, before any connection enables
/// WAL. This is where a lock-broken transport is diagnosed: with zero
/// concurrency and no busy timeout a CREATE + DROP must succeed; on Linux CIFS +
/// SMB2/3 it fails instantly with SQLITE_BUSY. The probe uses its own short-lived
/// connection so it never interferes with (or is masked by) concurrent WAL opens.
fn probe_network_share_locking(database_path: &Path) -> Result<()> {
    if !lan_shared_mode() || !filesystem_is_network_share(database_path) {
        return Ok(());
    }
    let conn = Connection::open(database_path)?;
    let key = database_key_hex();
    let pragmas = format!(
        "PRAGMA key = \"x'{key}'\"; PRAGMA cipher_memory_security = ON; PRAGMA busy_timeout = 0;"
    );
    let probe_result = conn
        .execute_batch(pragmas.as_str())
        .and_then(|()| probe_tables_work(&conn));
    drop(conn);
    if probe_result.is_ok() {
        return Ok(());
    }
    Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
        io::Error::other(
            "Esta pasta compartilhada não suporta o bloqueio de arquivos que o SQLite exige \
             (Linux + CIFS/SMB2/3 não oferece locks de byte-range). Use um compartilhamento com \
             locking POSIX (ex.: NFS, ou SMB1+unix extensions), clientes Windows/macOS, ou a \
             abordagem de servidor embutido.",
        ),
    )))
}

pub(crate) fn open_encrypted_database_with_mode(
    path: &Path,
    lan_shared: bool,
) -> Result<Connection> {
    let conn = Connection::open(path)?;
    let key = database_key_hex();
    let mut pragmas = format!(
        "PRAGMA key = \"x'{key}'\"; PRAGMA cipher_memory_security = ON; PRAGMA foreign_keys = ON;"
    );
    if lan_shared {
        pragmas.push_str("PRAGMA busy_timeout = 30000; PRAGMA synchronous = NORMAL;");
        conn.execute_batch(&pragmas)?;
        // journal_mode returns a row, so it must go through query_row.
        conn.query_row("PRAGMA journal_mode = WAL", [], |row| {
            row.get::<_, String>(0)
        })?;
    } else {
        conn.execute_batch(&pragmas)?;
    }
    conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(conn)
}

fn quote_sql(value: &Path) -> String {
    value.to_string_lossy().replace('\'', "''")
}

pub(crate) fn migrate_plaintext_database(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| database_error("Database path has no parent."))?;
    let staging_path = parent.join(format!(".opets-encrypted-{}.db", Uuid::new_v4()));
    let recovery_path = parent.join(format!(".opets-plaintext-recovery-{}.db", Uuid::new_v4()));
    let source = Connection::open(path)?;
    let key = database_key_hex();
    let export_result = source.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS encrypted KEY \"x'{}'\"; SELECT sqlcipher_export('encrypted'); DETACH DATABASE encrypted;",
        quote_sql(&staging_path),
        key,
    ));
    drop(source);
    if let Err(error) = export_result {
        let _ = fs::remove_file(&staging_path);
        return Err(error);
    }
    let encrypted = open_encrypted_database(&staging_path)?;
    encrypted.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))?;
    drop(encrypted);

    fs::rename(path, &recovery_path).map_err(io_error)?;
    if let Err(error) = fs::rename(&staging_path, path) {
        let _ = fs::rename(&recovery_path, path);
        return Err(io_error(error));
    }
    Ok(recovery_path)
}

pub(crate) fn ensure_private_dir(path: &Path) -> io::Result<()> {
    ensure_private_dir_with_mode(path, lan_shared_mode())
}

pub(crate) fn ensure_private_dir_with_mode(path: &Path, lan_shared: bool) -> io::Result<()> {
    fs::create_dir_all(path)?;
    if !lan_shared {
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub(crate) fn secure_private_file(path: &Path) -> io::Result<()> {
    secure_private_file_with_mode(path, lan_shared_mode())
}

pub(crate) fn secure_private_file_with_mode(path: &Path, lan_shared: bool) -> io::Result<()> {
    if lan_shared {
        return Ok(());
    }
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn should_seed_demo_data() -> bool {
    should_seed_demo_data_for(
        cfg!(debug_assertions),
        is_skip_db_seed_enabled(std::env::var("SKIP_DB_SEED").ok().as_deref()),
    )
}

fn should_seed_demo_data_for(is_debug_build: bool, skip_db_seed: bool) -> bool {
    is_debug_build && !skip_db_seed
}

fn is_skip_db_seed_enabled(value: Option<&str>) -> bool {
    matches!(
        value.map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "1" | "true")
    )
}

// Get database path from the persisted storage config (the user choice takes
// precedence over the environment), environment, or fallback to the application
// data directory.
fn get_database_path(app_data_dir: &Path, storage_config: &StorageConfig) -> Result<PathBuf> {
    let configured_path = storage_config.database_path.clone().or_else(|| {
        env::var("DATABASE_PATH")
            .ok()
            .or_else(|| env::var("DB_PATH").ok())
            .map(PathBuf::from)
    });
    Ok(resolve_database_path(configured_path, app_data_dir))
}

fn resolve_database_path(configured_path: Option<PathBuf>, app_data_dir: &Path) -> PathBuf {
    match configured_path {
        Some(path) if path.is_absolute() => path,
        Some(path) => env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path),
        None => app_data_dir.join("database.db"),
    }
}

// Run full migrations: schema + core defaults. The whole migration run happens
// inside a single queued ("IMMEDIATE") write transaction so that two processes
// bootstrapping the same storage for the first time on a shared folder cannot
// race on schema-changing statements (see `add_column_if_missing`). With WAL and
// the LAN busy timeout the second process simply waits for the first to commit.
pub(crate) fn run_migrations(conn: &Connection) -> Result<()> {
    let transaction =
        rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;
    run_schema_migrations(&transaction)?;
    ensure_core_defaults(&transaction)?;
    transaction.commit()
}

pub(crate) fn run_schema_migrations(conn: &Connection) -> Result<()> {
    // Create tables if they don't exist
    conn.execute_batch(
        "
        -- Settings table (singleton)
        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            company_name TEXT NOT NULL DEFAULT 'Minha Empresa',
            cnpj TEXT DEFAULT '',
            logo_path TEXT DEFAULT '',
            address TEXT DEFAULT ''
        );

        -- Customers table
        CREATE TABLE IF NOT EXISTS customers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            phone TEXT DEFAULT '',
            email TEXT DEFAULT '',
            address TEXT DEFAULT '',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            deleted_at TEXT
        );

        -- Users table
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            phone TEXT DEFAULT '',
            cpf TEXT DEFAULT '',
            join_date TEXT DEFAULT '',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            deleted_at TEXT
        );

        -- Inventory items table
        CREATE TABLE IF NOT EXISTS inventory_items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT DEFAULT '',
            type TEXT NOT NULL CHECK (type IN ('part', 'service')),
            min_quantity INTEGER NOT NULL DEFAULT 0,
            current_quantity INTEGER NOT NULL DEFAULT 0,
            cost_price REAL NOT NULL DEFAULT 0.0,
            average_cost REAL NOT NULL DEFAULT 0.0,
            sale_price REAL NOT NULL DEFAULT 0.0,
            cost_price_cents INTEGER NOT NULL DEFAULT 0 CHECK (cost_price_cents >= 0),
            average_cost_cents INTEGER NOT NULL DEFAULT 0 CHECK (average_cost_cents >= 0),
            sale_price_cents INTEGER NOT NULL DEFAULT 0 CHECK (sale_price_cents >= 0),
            supplier_name TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            deleted_at TEXT
        );

        -- Service orders table
        CREATE TABLE IF NOT EXISTS service_orders (
            id TEXT PRIMARY KEY,
            customer_id TEXT NOT NULL,
            customer_name TEXT,
            user_id TEXT, -- Technician ID
            equipment TEXT NOT NULL,
            imei TEXT,
            description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Orçamento' CHECK (status IN ('Orçamento', 'Em Manutenção', 'Aguardando Peça', 'Finalizada', 'Cancelada')),
            total_price REAL DEFAULT 0.0,
            total_price_cents INTEGER NOT NULL DEFAULT 0 CHECK (total_price_cents >= 0),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            closed_at TEXT,
            display_id TEXT NOT NULL DEFAULT '',
            discount_percent REAL NOT NULL DEFAULT 0.0,
            discount_basis_points INTEGER NOT NULL DEFAULT 0 CHECK (discount_basis_points BETWEEN 0 AND 10000),
            deleted_at TEXT,
            FOREIGN KEY (customer_id) REFERENCES customers (id),
            FOREIGN KEY (user_id) REFERENCES users (id)
        );

        -- Checklist templates table
        CREATE TABLE IF NOT EXISTS checklist_templates (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        -- Checklist items belonging to a template (The Blueprint)
        CREATE TABLE IF NOT EXISTS template_items (
            id TEXT PRIMARY KEY,
            template_id TEXT NOT NULL,
            label TEXT NOT NULL,
            FOREIGN KEY (template_id) REFERENCES checklist_templates (id) ON DELETE CASCADE
        );

        -- Checklist items actually used in a Service Order (The Instance)
        CREATE TABLE IF NOT EXISTS service_order_checklists (
            id TEXT PRIMARY KEY,
            service_order_id TEXT NOT NULL,
            label TEXT NOT NULL,
            checked BOOLEAN NOT NULL DEFAULT 0,
            FOREIGN KEY (service_order_id) REFERENCES service_orders (id) ON DELETE CASCADE
        );

        -- Service order parts table (junction for tracking parts used in orders)
        CREATE TABLE IF NOT EXISTS service_order_parts (
            id TEXT PRIMARY KEY,
            service_order_id TEXT NOT NULL,
            inventory_item_id TEXT NOT NULL,
            inventory_item_name TEXT NOT NULL DEFAULT '',
            item_type TEXT NOT NULL DEFAULT '',
            quantity INTEGER NOT NULL,
            unit_cost REAL NOT NULL DEFAULT 0.0,
            unit_price REAL NOT NULL DEFAULT 0.0,
            unit_cost_cents INTEGER NOT NULL DEFAULT 0 CHECK (unit_cost_cents >= 0),
            unit_price_cents INTEGER NOT NULL DEFAULT 0 CHECK (unit_price_cents >= 0),
            stock_restored BOOLEAN NOT NULL DEFAULT 0,
            FOREIGN KEY (service_order_id) REFERENCES service_orders (id) ON DELETE CASCADE,
            FOREIGN KEY (inventory_item_id) REFERENCES inventory_items (id)
        );

        -- Monotonic sequence used to generate collision-free OS display IDs.
        CREATE TABLE IF NOT EXISTS service_order_sequences (
            name TEXT PRIMARY KEY,
            value INTEGER NOT NULL DEFAULT 0
        );

        -- Immutable operational timeline for service orders.
        CREATE TABLE IF NOT EXISTS service_order_events (
            id TEXT PRIMARY KEY,
            service_order_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            details TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (service_order_id) REFERENCES service_orders (id) ON DELETE CASCADE
        );

        -- Metadata for files managed by the application storage directory.
        CREATE TABLE IF NOT EXISTS service_order_attachments (
            id TEXT PRIMARY KEY,
            service_order_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            storage_name TEXT NOT NULL UNIQUE,
            mime_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (service_order_id) REFERENCES service_orders (id) ON DELETE CASCADE
        );

        -- Financial snapshots table for trend calculations
        CREATE TABLE IF NOT EXISTS financial_snapshots (
            id TEXT PRIMARY KEY,
            snapshot_date DATE NOT NULL UNIQUE,
            total_revenue REAL NOT NULL DEFAULT 0.0,
            total_cost REAL NOT NULL DEFAULT 0.0,
            net_profit REAL NOT NULL DEFAULT 0.0,
            parts_in_use_cost REAL NOT NULL DEFAULT 0.0,
            total_revenue_cents INTEGER NOT NULL DEFAULT 0 CHECK (total_revenue_cents >= 0),
            total_cost_cents INTEGER NOT NULL DEFAULT 0 CHECK (total_cost_cents >= 0),
            estimated_gross_profit_cents INTEGER NOT NULL DEFAULT 0,
            parts_in_use_cost_cents INTEGER NOT NULL DEFAULT 0 CHECK (parts_in_use_cost_cents >= 0),
            active_orders_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        -- Inventory movements table (audit trail)
        CREATE TABLE IF NOT EXISTS inventory_movements (
            id TEXT PRIMARY KEY,
            inventory_item_id TEXT NOT NULL,
            type TEXT NOT NULL CHECK (type IN ('entrada', 'saida')),
            quantity INTEGER NOT NULL,
            reference_os_id TEXT,
            reason TEXT NOT NULL DEFAULT '',
            unit_cost REAL,
            unit_cost_cents INTEGER CHECK (unit_cost_cents IS NULL OR unit_cost_cents >= 0),
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (inventory_item_id) REFERENCES inventory_items (id)
        );

        -- Index for faster snapshot queries by date
        CREATE INDEX IF NOT EXISTS idx_financial_snapshots_date ON financial_snapshots(snapshot_date);

        -- Index for inventory movements lookup
        CREATE INDEX IF NOT EXISTS idx_inventory_movements_item ON inventory_movements(inventory_item_id);
        CREATE INDEX IF NOT EXISTS idx_service_order_events_order ON service_order_events(service_order_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_service_order_attachments_order ON service_order_attachments(service_order_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_template_items_template ON template_items(template_id);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_service_orders_display_id ON service_orders(display_id) WHERE display_id <> '';
        ",
    )?;

    // Migration: add columns to service_orders if missing
    add_column_if_missing(conn, "service_orders", "deleted_at", "TEXT")?;
    add_column_if_missing(
        conn,
        "service_orders",
        "display_id",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "service_orders",
        "discount_percent",
        "REAL NOT NULL DEFAULT 0.0",
    )?;

    // Migration: add reason to legacy inventory movement records.
    add_column_if_missing(
        conn,
        "inventory_movements",
        "reason",
        "TEXT NOT NULL DEFAULT ''",
    )?;

    // Additive inventory migrations preserve existing catalog and audit data.
    add_column_if_missing(
        conn,
        "inventory_items",
        "average_cost",
        "REAL NOT NULL DEFAULT 0.0",
    )?;
    add_column_if_missing(conn, "inventory_items", "supplier_name", "TEXT")?;
    add_column_if_missing(conn, "inventory_movements", "unit_cost", "REAL")?;

    // Preserve the catalog identity used by an OS even when the item is later renamed or retyped.
    add_column_if_missing(
        conn,
        "service_order_parts",
        "inventory_item_name",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "service_order_parts",
        "item_type",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "service_order_parts",
        "stock_restored",
        "BOOLEAN NOT NULL DEFAULT 0",
    )?;
    conn.execute_batch(
        "UPDATE service_order_parts
         SET inventory_item_name = (SELECT name FROM inventory_items WHERE id = inventory_item_id),
             item_type = (SELECT type FROM inventory_items WHERE id = inventory_item_id)
         WHERE inventory_item_name = '' OR item_type = '';",
    )?;

    // Migration: add columns to users if missing from intermediate schema
    add_column_if_missing(conn, "users", "phone", "TEXT DEFAULT ''")?;
    add_column_if_missing(conn, "users", "cpf", "TEXT DEFAULT ''")?;
    add_column_if_missing(conn, "users", "join_date", "TEXT DEFAULT ''")?;

    // Migration: migrate users table from old schema (role) to new schema (phone, cpf, join_date)
    {
        let has_role_col: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'role'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);

        if has_role_col {
            eprintln!("[MIGRATION] Migrating users table to new schema...");
            conn.execute_batch(
                "PRAGMA foreign_keys = OFF;
                DROP TABLE IF EXISTS users_new;
                CREATE TABLE users_new (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    email TEXT NOT NULL UNIQUE,
                    phone TEXT DEFAULT '',
                    cpf TEXT DEFAULT '',
                    join_date TEXT DEFAULT '',
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    updated_at TEXT,
                    deleted_at TEXT
                );
                INSERT INTO users_new (id, name, email, created_at, updated_at, deleted_at)
                    SELECT id, name, email, created_at, updated_at, deleted_at FROM users;
                DROP TABLE users;
                ALTER TABLE users_new RENAME TO users;
                PRAGMA foreign_keys = ON;",
            )?;
            eprintln!("[MIGRATION] Users table migrated successfully.");
        }
    }

    // Performance: denormalized local-date columns so the financial/dashboard date
    // filters are sargable. The old filters wrapped the column in
    // date(COALESCE(closed_at, created_at), 'localtime'), which blocks index use and
    // forces full table scans on every aggregation query. Values are maintained on
    // write (create/transition/seeds); this migration backfills existing rows.
    add_column_if_missing(conn, "service_orders", "created_date", "TEXT")?;
    add_column_if_missing(conn, "service_orders", "finalized_date", "TEXT")?;
    conn.execute(
        "UPDATE service_orders
         SET created_date = COALESCE(created_date, date(created_at, 'localtime')),
             finalized_date = CASE
                 WHEN status = 'Finalizada' THEN COALESCE(finalized_date, date(COALESCE(closed_at, created_at), 'localtime'))
             END",
        [],
    )?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_service_orders_finalized
             ON service_orders(status, deleted_at, finalized_date);
         CREATE INDEX IF NOT EXISTS idx_service_orders_created
             ON service_orders(deleted_at, created_date);
         CREATE INDEX IF NOT EXISTS idx_service_orders_customer_history
             ON service_orders(customer_id, deleted_at, created_date);
         CREATE INDEX IF NOT EXISTS idx_service_order_parts_order
             ON service_order_parts(service_order_id);",
    )?;

    migrate_integer_money(conn)?;
    Ok(())
}

pub(crate) fn ensure_core_defaults(conn: &Connection) -> Result<()> {
    // Insert default settings if not exists
    conn.execute(
        "INSERT OR IGNORE INTO settings (id, company_name) VALUES (1, 'Minha Empresa')",
        [],
    )?;

    // Insert initial financial snapshot for today if not exists
    conn.execute(
        "INSERT OR IGNORE INTO financial_snapshots (id, snapshot_date) VALUES (?, date('now'))",
        params![Uuid::new_v4().to_string()],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO service_order_sequences (name, value)
         SELECT 'service_order', COALESCE(MAX(CAST(SUBSTR(display_id, 4) AS INTEGER)), 0)
         FROM service_orders",
        [],
    )?;

    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let exists: bool = conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1)"),
        [column],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {definition}"
        ))?;
    }
    Ok(())
}

fn migrate_integer_money(conn: &Connection) -> Result<()> {
    for (table, column, definition) in [
        ("inventory_items", "cost_price_cents", "INTEGER CHECK (cost_price_cents IS NULL OR cost_price_cents >= 0)"),
        ("inventory_items", "average_cost_cents", "INTEGER CHECK (average_cost_cents IS NULL OR average_cost_cents >= 0)"),
        ("inventory_items", "sale_price_cents", "INTEGER CHECK (sale_price_cents IS NULL OR sale_price_cents >= 0)"),
        ("service_orders", "total_price_cents", "INTEGER CHECK (total_price_cents IS NULL OR total_price_cents >= 0)"),
        ("service_orders", "discount_basis_points", "INTEGER CHECK (discount_basis_points IS NULL OR discount_basis_points BETWEEN 0 AND 10000)"),
        ("service_order_parts", "unit_cost_cents", "INTEGER CHECK (unit_cost_cents IS NULL OR unit_cost_cents >= 0)"),
        ("service_order_parts", "unit_price_cents", "INTEGER CHECK (unit_price_cents IS NULL OR unit_price_cents >= 0)"),
        ("inventory_movements", "unit_cost_cents", "INTEGER CHECK (unit_cost_cents IS NULL OR unit_cost_cents >= 0)"),
        ("financial_snapshots", "total_revenue_cents", "INTEGER CHECK (total_revenue_cents IS NULL OR total_revenue_cents >= 0)"),
        ("financial_snapshots", "total_cost_cents", "INTEGER CHECK (total_cost_cents IS NULL OR total_cost_cents >= 0)"),
        (
            "financial_snapshots",
            "estimated_gross_profit_cents",
            "INTEGER",
        ),
        ("financial_snapshots", "parts_in_use_cost_cents", "INTEGER CHECK (parts_in_use_cost_cents IS NULL OR parts_in_use_cost_cents >= 0)"),
    ] {
        add_column_if_missing(conn, table, column, definition)?;
    }
    conn.execute_batch(
        "UPDATE inventory_items SET cost_price_cents = ROUND(cost_price * 100)
         WHERE cost_price_cents IS NULL;
         UPDATE inventory_items SET average_cost_cents = ROUND(
             CASE WHEN average_cost > 0 THEN average_cost ELSE cost_price END * 100
         )
         WHERE average_cost_cents IS NULL;
         UPDATE inventory_items SET sale_price_cents = ROUND(sale_price * 100)
         WHERE sale_price_cents IS NULL;
         UPDATE service_orders SET total_price_cents = ROUND(COALESCE(total_price, 0) * 100)
         WHERE total_price_cents IS NULL;
         UPDATE service_orders SET discount_basis_points = ROUND(COALESCE(discount_percent, 0) * 100)
         WHERE discount_basis_points IS NULL;
         UPDATE service_order_parts SET unit_cost_cents = ROUND(unit_cost * 100)
         WHERE unit_cost_cents IS NULL;
         UPDATE service_order_parts SET unit_price_cents = ROUND(unit_price * 100)
         WHERE unit_price_cents IS NULL;
         UPDATE inventory_movements SET unit_cost_cents = ROUND(unit_cost * 100)
         WHERE unit_cost_cents IS NULL AND unit_cost IS NOT NULL;
         UPDATE financial_snapshots SET total_revenue_cents = ROUND(total_revenue * 100)
         WHERE total_revenue_cents IS NULL;
         UPDATE financial_snapshots SET total_cost_cents = ROUND(total_cost * 100)
         WHERE total_cost_cents IS NULL;
         UPDATE financial_snapshots SET estimated_gross_profit_cents = ROUND(net_profit * 100)
         WHERE estimated_gross_profit_cents IS NULL;
         UPDATE financial_snapshots SET parts_in_use_cost_cents = ROUND(parts_in_use_cost * 100)
         WHERE parts_in_use_cost_cents IS NULL;",
    )?;
    make_legacy_part_prices_optional(conn)?;
    validate_integer_money(conn)?;
    Ok(())
}

fn validate_integer_money(conn: &Connection) -> Result<()> {
    let invalid: bool = conn.query_row(
        "SELECT
            EXISTS(SELECT 1 FROM inventory_items WHERE cost_price_cents IS NULL OR cost_price_cents < 0 OR average_cost_cents IS NULL OR average_cost_cents < 0 OR sale_price_cents IS NULL OR sale_price_cents < 0)
            OR EXISTS(SELECT 1 FROM service_orders WHERE total_price_cents IS NULL OR total_price_cents < 0 OR discount_basis_points IS NULL OR discount_basis_points NOT BETWEEN 0 AND 10000)
            OR EXISTS(SELECT 1 FROM service_order_parts WHERE unit_cost_cents IS NULL OR unit_cost_cents < 0 OR unit_price_cents IS NULL OR unit_price_cents < 0)
            OR EXISTS(SELECT 1 FROM inventory_movements WHERE unit_cost_cents < 0)
            OR EXISTS(SELECT 1 FROM financial_snapshots WHERE total_revenue_cents IS NULL OR total_revenue_cents < 0 OR total_cost_cents IS NULL OR total_cost_cents < 0 OR estimated_gross_profit_cents IS NULL OR parts_in_use_cost_cents IS NULL OR parts_in_use_cost_cents < 0)",
        [],
        |row| row.get(0),
    )?;
    if invalid {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn make_legacy_part_prices_optional(conn: &Connection) -> Result<()> {
    let unit_cost_not_null: i64 = conn.query_row(
        "SELECT \"notnull\" FROM pragma_table_info('service_order_parts') WHERE name = 'unit_cost'",
        [],
        |row| row.get(0),
    )?;
    if unit_cost_not_null == 0 {
        return Ok(());
    }

    conn.execute_batch(
        "ALTER TABLE service_order_parts RENAME TO service_order_parts_legacy_money;
         CREATE TABLE service_order_parts (
             id TEXT PRIMARY KEY,
             service_order_id TEXT NOT NULL,
             inventory_item_id TEXT NOT NULL,
             inventory_item_name TEXT NOT NULL DEFAULT '',
             item_type TEXT NOT NULL DEFAULT '',
             quantity INTEGER NOT NULL,
             unit_cost REAL,
             unit_price REAL,
             unit_cost_cents INTEGER NOT NULL DEFAULT 0 CHECK (unit_cost_cents >= 0),
             unit_price_cents INTEGER NOT NULL DEFAULT 0 CHECK (unit_price_cents >= 0),
             stock_restored BOOLEAN NOT NULL DEFAULT 0,
             FOREIGN KEY (service_order_id) REFERENCES service_orders (id) ON DELETE CASCADE,
             FOREIGN KEY (inventory_item_id) REFERENCES inventory_items (id)
         );
         INSERT INTO service_order_parts (
             id, service_order_id, inventory_item_id, inventory_item_name, item_type, quantity,
             unit_cost, unit_price, unit_cost_cents, unit_price_cents, stock_restored
         )
         SELECT id, service_order_id, inventory_item_id, inventory_item_name, item_type, quantity,
                unit_cost, unit_price, unit_cost_cents, unit_price_cents, stock_restored
         FROM service_order_parts_legacy_money;
         DROP TABLE service_order_parts_legacy_money;",
    )
}

// Get database connection - returns a new connection using the stored path
pub fn get_db() -> Result<DatabaseConnection> {
    let guard = STORAGE_OPERATION_LOCK
        .read()
        .map_err(|_| database_error("Storage operation lock is unavailable."))?;
    let mut connection = DB_CONNECTION
        .lock()
        .map_err(|_| database_error("Database connection lock is unavailable."))?;
    if connection.is_none() {
        *connection = Some(open_encrypted_database(&database_path())?);
    }
    Ok(DatabaseConnection {
        connection,
        _guard: guard,
    })
}

pub(crate) fn exclusive_storage_guard() -> Result<ExclusiveStorageGuard> {
    STORAGE_OPERATION_LOCK
        .write()
        .map_err(|_| database_error("Storage operation lock is unavailable."))
}

pub fn database_path() -> PathBuf {
    DB_PATH
        .get()
        .cloned()
        .expect("Database path must be initialized before use")
}

pub(crate) fn app_data_dir() -> PathBuf {
    APP_DATA_DIR
        .get()
        .cloned()
        .expect("Application data path must be initialized before use")
}

pub fn attachments_dir() -> PathBuf {
    attachments_dir_for(&database_path())
}

pub(crate) fn attachments_dir_for(database_path: &Path) -> PathBuf {
    let mut path = database_path.to_path_buf();
    path.set_extension("attachments");
    path
}

#[cfg(test)]
pub(crate) fn initialize_test_database(path: &Path) -> Result<()> {
    initialize_storage_at(path, false)?;
    if let Some(app_data_dir) = path.parent() {
        // The single global test storage also acts as the application data
        // directory, so storage-config commands can be exercised in tests.
        if APP_DATA_DIR.get().is_none() {
            APP_DATA_DIR.set(app_data_dir.to_path_buf()).map_err(|_| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(
                    "Application data path was already initialized.",
                )))
            })?;
        }
    }
    match DB_PATH.get() {
        Some(initialized) if initialized == path => Ok(()),
        Some(_) => Err(database_error(
            "Test database path was already initialized with a different path.",
        )),
        None => DB_PATH
            .set(path.to_path_buf())
            .map_err(|_| database_error("Test database path was already initialized.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{setup_db, setup_legacy_users_db};

    #[test]
    fn default_database_path_uses_the_application_data_directory() {
        let app_data_dir = PathBuf::from("/tmp/opets-data");

        assert_eq!(
            resolve_database_path(None, &app_data_dir),
            app_data_dir.join("database.db")
        );
    }

    #[test]
    fn configured_absolute_database_path_overrides_application_data_directory() {
        let app_data_dir = PathBuf::from("/tmp/opets-data");
        let configured_path = PathBuf::from("/tmp/custom/database.db");

        assert_eq!(
            resolve_database_path(Some(configured_path.clone()), &app_data_dir),
            configured_path
        );
    }

    #[test]
    fn user_storage_config_precedes_every_other_resolution_source() {
        let app_data_dir = PathBuf::from("/tmp/opets-data");
        let shared_path = PathBuf::from("/mnt/lan-share/database.db");
        let config = StorageConfig {
            database_path: Some(shared_path.clone()),
            lan_shared: true,
            device_id: None,
        };

        // The environment (DATABASE_PATH/DB_PATH) may or may not be set in the
        // test process; the user choice must win in either case.
        let resolved = get_database_path(&app_data_dir, &config).unwrap();
        assert_eq!(resolved, shared_path);
    }

    #[test]
    fn default_storage_config_resolves_to_the_application_data_directory() {
        let app_data_dir = PathBuf::from("/tmp/opets-data");
        let config = StorageConfig::default();

        let resolved = get_database_path(&app_data_dir, &config).unwrap();
        // Only valid when no DATABASE_PATH/DB_PATH env var is present.
        let env_override = env::var("DATABASE_PATH")
            .ok()
            .or_else(|| env::var("DB_PATH").ok())
            .is_some();
        if !env_override {
            assert_eq!(resolved, app_data_dir.join("database.db"));
        }
    }

    #[test]
    fn storage_config_round_trips_through_the_application_data_directory() {
        let app_data_dir = std::env::temp_dir().join(format!("opets-config-{}", Uuid::new_v4()));
        let config = StorageConfig {
            database_path: Some(PathBuf::from("/mnt/lan-share/database.db")),
            lan_shared: true,
            device_id: None,
        };

        save_storage_config(&app_data_dir, &config).unwrap();
        let loaded = load_storage_config(&app_data_dir);

        assert_eq!(loaded.database_path, config.database_path);
        assert!(loaded.lan_shared);
        let _ = fs::remove_dir_all(app_data_dir);
    }

    #[test]
    fn missing_storage_config_defaults_to_single_user_storage() {
        let app_data_dir =
            std::env::temp_dir().join(format!("opets-config-missing-{}", Uuid::new_v4()));

        let loaded = load_storage_config(&app_data_dir);

        assert_eq!(loaded.database_path, None);
        assert!(!loaded.lan_shared);
        let _ = fs::remove_dir_all(app_data_dir);
    }

    #[test]
    fn shared_mode_is_off_by_default() {
        // The flag is only flipped by init_db; tests never initialize it.
        assert!(!lan_shared_mode());
    }

    #[test]
    fn lan_mode_enables_wal_busy_timeout_and_normal_synchronous() {
        let directory = std::env::temp_dir().join(format!("opets-lan-wal-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();

        let shared_database = directory.join("shared.db");
        let shared = open_encrypted_database_with_mode(&shared_database, true).unwrap();
        let journal_mode: String = shared
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        let busy_timeout: i64 = shared
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        let synchronous: i64 = shared
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        drop(shared);
        assert_eq!(journal_mode, "wal");
        assert_eq!(busy_timeout, 30000);
        assert_eq!(synchronous, 1); // NORMAL

        let single_user_database = directory.join("single.db");
        let single_user = open_encrypted_database_with_mode(&single_user_database, false).unwrap();
        let single_journal_mode: String = single_user
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        drop(single_user);
        assert_eq!(single_journal_mode, "delete");

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn lan_mode_keeps_wal_journal_and_normal_sync() {
        let directory = std::env::temp_dir().join(format!("opets-lan-wal-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let database = directory.join("database.db");

        let lan = open_encrypted_database_with_mode(&database, true).unwrap();
        let journal: String = lan
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        let synchronous: i64 = lan
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        drop(lan);
        assert_eq!(journal, "wal");
        assert_eq!(synchronous, 1, "WAL pairs with synchronous=NORMAL");

        let local = open_encrypted_database_with_mode(&database, false).unwrap();
        let local_journal: String = local
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        drop(local);
        assert_eq!(local_journal, "wal");

        let _ = fs::remove_dir_all(directory);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn local_disks_are_not_network_shares() {
        let directory = std::env::temp_dir();
        assert!(
            !filesystem_is_network_share(&directory.join("database.db")),
            "a temp-dir database must not be treated as a network share"
        );
    }

    #[cfg(unix)]
    #[test]
    fn lan_mode_keeps_shared_storage_permissions_open() {
        let directory = std::env::temp_dir().join(format!("opets-lan-perms-{}", Uuid::new_v4()));
        let file = directory.join("database.db");

        ensure_private_dir_with_mode(&directory, true).unwrap();
        fs::write(&file, b"database").unwrap();

        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        secure_private_file_with_mode(&file, true).unwrap();
        let shared_permissions = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(shared_permissions, 0o644);

        ensure_private_dir_with_mode(&directory, false).unwrap();
        secure_private_file_with_mode(&file, false).unwrap();
        let private_permissions = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(private_permissions, 0o600);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn concurrent_migrations_on_the_same_database_are_serialized_and_idempotent() {
        let directory = std::env::temp_dir().join(format!("opets-migrations-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let database = directory.join("database.db");

        let first = Connection::open(&database).unwrap();
        let second = Connection::open(&database).unwrap();
        first.execute_batch("PRAGMA busy_timeout = 15000;").unwrap();
        second
            .execute_batch("PRAGMA busy_timeout = 15000;")
            .unwrap();

        let first_thread = std::thread::spawn(move || run_migrations(&first));
        let second_thread = std::thread::spawn(move || run_migrations(&second));

        first_thread.join().unwrap().unwrap();
        second_thread.join().unwrap().unwrap();

        let conn = Connection::open(&database).unwrap();
        let phone_column: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'phone'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(phone_column, 1);
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        drop(conn);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn storage_instance_lock_rejects_a_second_owner() {
        let temp_dir = tempfile::tempdir().unwrap();
        let database_path = temp_dir.path().join("database.db");
        let first = open_storage_instance_lock(&database_path).unwrap();

        assert!(open_storage_instance_lock(&database_path).is_err());
        drop(first);
        assert!(open_storage_instance_lock(&database_path).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn private_storage_permissions_are_restricted_to_the_current_user() {
        let temp_dir = std::env::temp_dir().join(format!("opets-private-{}", Uuid::new_v4()));
        let database_file = temp_dir.join("database.db");

        ensure_private_dir(&temp_dir).unwrap();
        fs::write(&database_file, b"database").unwrap();
        secure_private_file(&database_file).unwrap();

        assert_eq!(
            fs::metadata(&temp_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&database_file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn schema_migrations_run_without_inserting_data() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        run_schema_migrations(&conn).unwrap();

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let settings_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();

        assert!(table_count >= 14);
        assert_eq!(settings_count, 0);
    }

    #[test]
    fn encrypted_database_cannot_be_read_without_the_application_key() {
        let path = std::env::temp_dir().join(format!("opets-encrypted-{}.db", Uuid::new_v4()));
        let conn = open_encrypted_database(&path).unwrap();
        run_migrations(&conn).unwrap();
        drop(conn);

        assert!(Connection::open(&path)
            .and_then(
                |conn| conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row
                    .get::<_, i64>(0))
            )
            .is_err());
        assert!(open_encrypted_database(&path).is_ok());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn plaintext_database_is_migrated_to_encrypted_storage() {
        let path = std::env::temp_dir().join(format!("opets-legacy-{}.db", Uuid::new_v4()));
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "UPDATE settings SET company_name = 'Dados legados' WHERE id = 1",
            [],
        )
        .unwrap();
        drop(conn);

        let recovery = migrate_plaintext_database(&path).unwrap();
        assert!(Connection::open(&path)
            .and_then(
                |conn| conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row
                    .get::<_, i64>(0))
            )
            .is_err());
        let encrypted = open_encrypted_database(&path).unwrap();
        let company_name: String = encrypted
            .query_row(
                "SELECT company_name FROM settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(company_name, "Dados legados");

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(recovery);
    }

    #[test]
    fn creates_encrypted_recovery_backup_before_plaintext_migration() {
        let directory = std::env::temp_dir().join(format!("opets-recovery-{}", Uuid::new_v4()));
        let database = directory.join("database.db");
        let attachments = directory.join("database.attachments");
        fs::create_dir_all(&attachments).unwrap();
        let conn = Connection::open(&database).unwrap();
        run_migrations(&conn).unwrap();
        drop(conn);

        let backup = create_pre_encryption_recovery_backup(&database, &attachments).unwrap();
        assert!(backup.exists());
        assert!(
            !crate::backup_service::inspect_backup(&backup)
                .unwrap()
                .requires_passphrase
        );

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn migrations_create_core_tables_and_indexes() {
        let conn = setup_db();

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'settings', 'customers', 'users', 'inventory_items', 'service_orders',
                    'checklist_templates', 'template_items', 'service_order_checklists',
                    'service_order_parts', 'financial_snapshots', 'inventory_movements',
                    'service_order_sequences', 'service_order_events', 'service_order_attachments'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name IN (
                    'idx_financial_snapshots_date', 'idx_inventory_movements_item',
                    'idx_service_orders_customer_history'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(table_count, 14);
        assert_eq!(index_count, 3);
    }

    #[test]
    fn migrations_insert_default_settings_row() {
        let conn = setup_db();

        let company_name: String = conn
            .query_row(
                "SELECT company_name FROM settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(company_name, "Minha Empresa");
    }

    #[test]
    fn migrations_insert_initial_financial_snapshot() {
        let conn = setup_db();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM financial_snapshots", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn demo_seeds_only_run_in_debug_without_skip_flag() {
        assert!(should_seed_demo_data_for(true, false));
        assert!(!should_seed_demo_data_for(true, true));
        assert!(!should_seed_demo_data_for(false, false));
        assert!(!should_seed_demo_data_for(false, true));
    }

    #[test]
    fn skip_db_seed_accepts_true_and_one() {
        assert!(is_skip_db_seed_enabled(Some("true")));
        assert!(is_skip_db_seed_enabled(Some(" 1 ")));
        assert!(!is_skip_db_seed_enabled(Some("false")));
        assert!(!is_skip_db_seed_enabled(None));
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = setup_db();

        run_migrations(&conn).unwrap();

        let settings_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM settings WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        let snapshot_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM financial_snapshots", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(settings_count, 1);
        assert_eq!(snapshot_count, 1);
    }

    #[test]
    fn migrations_upgrade_legacy_users_schema() {
        let conn = setup_legacy_users_db();

        conn.execute(
            "INSERT INTO users (id, name, email, role, created_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params!["user-1", "Maria", "maria@example.com", "admin"],
        )
        .unwrap();

        run_migrations(&conn).unwrap();

        let has_role: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'role'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let migrated_row: (String, String, String) = conn
            .query_row(
                "SELECT name, phone, cpf FROM users WHERE id = 'user-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(has_role, 0);
        assert_eq!(migrated_row.0, "Maria");
        assert_eq!(migrated_row.1, "");
        assert_eq!(migrated_row.2, "");
    }

    #[test]
    fn migrations_upgrade_legacy_inventory_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE inventory_items (id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT DEFAULT '', type TEXT NOT NULL, min_quantity INTEGER NOT NULL DEFAULT 0, current_quantity INTEGER NOT NULL DEFAULT 0, cost_price REAL NOT NULL DEFAULT 0.0, sale_price REAL NOT NULL DEFAULT 0.0, created_at TEXT, updated_at TEXT, deleted_at TEXT);
             CREATE TABLE inventory_movements (id TEXT PRIMARY KEY, inventory_item_id TEXT NOT NULL, type TEXT NOT NULL, quantity INTEGER NOT NULL, reference_os_id TEXT, reason TEXT NOT NULL DEFAULT '', created_at TEXT);"
        ).unwrap();
        conn.execute("INSERT INTO inventory_items (id, name, type, cost_price) VALUES ('part-1', 'Tela', 'part', 42.5)", []).unwrap();

        run_migrations(&conn).unwrap();

        let item: (i64, Option<String>) = conn
            .query_row(
                "SELECT average_cost_cents, supplier_name FROM inventory_items WHERE id = 'part-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let has_unit_cost: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('inventory_movements') WHERE name = 'unit_cost'", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(item.0, 4_250);
        assert!(item.1.is_none());
        assert_eq!(has_unit_cost, 1);
    }

    #[test]
    fn money_migration_backfills_legacy_decimals_and_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE inventory_items (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT DEFAULT '', type TEXT NOT NULL,
                min_quantity INTEGER NOT NULL DEFAULT 0, current_quantity INTEGER NOT NULL DEFAULT 0,
                cost_price REAL NOT NULL DEFAULT 0.0, average_cost REAL NOT NULL DEFAULT 0.0,
                sale_price REAL NOT NULL DEFAULT 0.0, cost_price_cents INTEGER,
                average_cost_cents INTEGER, sale_price_cents INTEGER,
                created_at TEXT, updated_at TEXT, deleted_at TEXT
             );
             CREATE TABLE service_orders (
                id TEXT PRIMARY KEY, customer_id TEXT NOT NULL, customer_name TEXT, user_id TEXT,
                equipment TEXT NOT NULL, imei TEXT, description TEXT NOT NULL, status TEXT NOT NULL,
                total_price REAL DEFAULT 0.0, created_at TEXT NOT NULL, updated_at TEXT, closed_at TEXT,
                display_id TEXT NOT NULL DEFAULT '', discount_percent REAL NOT NULL DEFAULT 0.0,
                total_price_cents INTEGER, discount_basis_points INTEGER, deleted_at TEXT
             );
             CREATE TABLE service_order_parts (
                id TEXT PRIMARY KEY, service_order_id TEXT NOT NULL, inventory_item_id TEXT NOT NULL,
                quantity INTEGER NOT NULL, unit_cost REAL NOT NULL, unit_price REAL NOT NULL,
                unit_cost_cents INTEGER, unit_price_cents INTEGER
             );
             CREATE TABLE inventory_movements (
                id TEXT PRIMARY KEY, inventory_item_id TEXT NOT NULL, type TEXT NOT NULL,
                quantity INTEGER NOT NULL, reference_os_id TEXT, reason TEXT NOT NULL DEFAULT '',
                unit_cost REAL, created_at TEXT
             );
             CREATE TABLE financial_snapshots (
                id TEXT PRIMARY KEY, snapshot_date DATE NOT NULL UNIQUE,
                total_revenue REAL NOT NULL DEFAULT 0.0, total_cost REAL NOT NULL DEFAULT 0.0,
                net_profit REAL NOT NULL DEFAULT 0.0, parts_in_use_cost REAL NOT NULL DEFAULT 0.0,
                total_revenue_cents INTEGER, total_cost_cents INTEGER,
                estimated_gross_profit_cents INTEGER, parts_in_use_cost_cents INTEGER,
                active_orders_count INTEGER NOT NULL DEFAULT 0, created_at TEXT
             );
             INSERT INTO inventory_items (id, name, type, cost_price, average_cost, sale_price, cost_price_cents, average_cost_cents, sale_price_cents)
                VALUES ('part-1', 'Tela', 'part', 42.567, 12.34, 99.999, 777, NULL, 10000);
             INSERT INTO inventory_items (id, name, type, cost_price, average_cost, sale_price, cost_price_cents, average_cost_cents, sale_price_cents)
                VALUES ('part-2', 'Cabo', 'part', 10.0, 0.0, 20.0, 1000, 0, 2000);
             INSERT INTO service_orders (id, customer_id, equipment, description, status, total_price, created_at, discount_percent, total_price_cents, discount_basis_points)
                VALUES ('order-1', 'customer-1', 'Celular', 'Reparo', 'Finalizada', 123.456, CURRENT_TIMESTAMP, 1.5, 888, NULL);
             INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, quantity, unit_cost, unit_price, unit_cost_cents, unit_price_cents)
                VALUES ('line-1', 'order-1', 'part-1', 2, 42.567, 7.89, 666, NULL);
             INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, unit_cost)
                VALUES ('movement-1', 'part-1', 'entrada', 2, 40.555);
             INSERT INTO financial_snapshots (
                 id, snapshot_date, total_revenue, total_cost, net_profit, parts_in_use_cost,
                 total_revenue_cents, total_cost_cents, estimated_gross_profit_cents, parts_in_use_cost_cents
              ) VALUES ('snapshot-1', '2020-01-01', 123.456, 4.44, 38.322, 2.22, 555, NULL, 333, NULL);",
        )
        .unwrap();

        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        let item: (i64, i64, i64) = conn
            .query_row(
                "SELECT cost_price_cents, average_cost_cents, sale_price_cents FROM inventory_items WHERE id = 'part-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let order: (i64, i64) = conn
            .query_row(
                "SELECT total_price_cents, discount_basis_points FROM service_orders WHERE id = 'order-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let migrated_zero_average: i64 = conn
            .query_row(
                "SELECT average_cost_cents FROM inventory_items WHERE id = 'part-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let line: (i64, i64) = conn
            .query_row(
                "SELECT unit_cost_cents, unit_price_cents FROM service_order_parts WHERE id = 'line-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let movement: i64 = conn
            .query_row(
                "SELECT unit_cost_cents FROM inventory_movements WHERE id = 'movement-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let snapshot: (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT total_revenue_cents, total_cost_cents, estimated_gross_profit_cents, parts_in_use_cost_cents
                 FROM financial_snapshots WHERE id = 'snapshot-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(item, (777, 1_234, 10_000));
        assert_eq!(migrated_zero_average, 0);
        assert_eq!(order, (888, 150));
        assert_eq!(line, (666, 789));
        assert_eq!(movement, 4_056);
        assert_eq!(snapshot, (555, 444, 333, 222));
    }

    #[test]
    fn fresh_schema_rejects_invalid_integer_money() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let customer = crate::models::customer::Customer::new(
            "Cliente".to_string(),
            "41".to_string(),
            "cliente@example.com".to_string(),
            "Rua".to_string(),
        );
        crate::repositories::customer_repo::CustomerRepository::create_with_conn(&conn, &customer)
            .unwrap();
        assert!(conn
            .execute(
                "INSERT INTO service_orders (id, customer_id, equipment, description, discount_basis_points)
                 VALUES ('invalid-order', ?1, 'Celular', 'Reparo', 10001)",
                [customer.id],
            )
            .is_err());
    }

    #[test]
    fn migration_rejects_out_of_range_existing_money() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute_batch(
            "PRAGMA ignore_check_constraints = ON;
             UPDATE financial_snapshots SET total_cost_cents = -1;",
        )
        .unwrap();

        assert!(migrate_integer_money(&conn).is_err());
    }
}
