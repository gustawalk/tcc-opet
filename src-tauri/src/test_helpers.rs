use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

static GLOBAL_BACKEND_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct GlobalTestStorage {
    _root: TempDir,
    database_path: PathBuf,
}

static GLOBAL_TEST_STORAGE: LazyLock<GlobalTestStorage> = LazyLock::new(|| {
    let root = tempfile::tempdir().expect("failed to create global backend temp directory");
    let database_path = root.path().join("database.db");
    crate::database::initialize_test_database(&database_path)
        .expect("failed to initialize global encrypted test database");
    GlobalTestStorage {
        _root: root,
        database_path,
    }
});

pub struct GlobalTestBackend {
    _guard: MutexGuard<'static, ()>,
}

impl GlobalTestBackend {
    pub fn database_path(&self) -> &Path {
        &GLOBAL_TEST_STORAGE.database_path
    }

    pub fn attachments_dir(&self) -> PathBuf {
        crate::database::attachments_dir_for(self.database_path())
    }

    pub fn temp_file(&self, name: &str, extension: &str, bytes: &[u8]) -> PathBuf {
        let path = GLOBAL_TEST_STORAGE._root.path().join(format!(
            "{}_{}.{}",
            name,
            uuid::Uuid::new_v4(),
            extension
        ));
        fs::write(&path, bytes).expect("failed to write backend fixture file");
        path
    }

    pub fn temp_path(&self, name: &str) -> PathBuf {
        GLOBAL_TEST_STORAGE
            ._root
            .path()
            .join(format!("{}_{}", name, uuid::Uuid::new_v4()))
    }
}

pub fn setup_global_backend() -> GlobalTestBackend {
    let guard = GLOBAL_BACKEND_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    LazyLock::force(&GLOBAL_TEST_STORAGE);
    crate::database::update_storage_mode_config(&crate::database::StorageModeConfig::default())
        .expect("failed to reset LAN mode configuration");

    let conn = crate::database::get_db().expect("failed to open global test database");
    crate::commands::settings_commands::reset_database_with_conn(&conn)
        .expect("failed to reset global test database");
    drop(conn);

    let attachments_dir = crate::database::attachments_dir();
    if attachments_dir.exists() {
        fs::remove_dir_all(&attachments_dir).expect("failed to clear test attachments");
    }
    if let Ok(mut pending) =
        crate::commands::attachment_commands::PENDING_ATTACHMENT_SELECTIONS.lock()
    {
        pending.clear();
    }

    GlobalTestBackend { _guard: guard }
}

pub struct TestStorage {
    _root: TempDir,
    pub database_path: PathBuf,
}

impl TestStorage {
    pub fn new(seed_demo_data: bool) -> Self {
        let root = tempfile::tempdir().expect("failed to create storage temp directory");
        let database_path = root.path().join("database.db");
        crate::database::initialize_storage_at(&database_path, seed_demo_data)
            .expect("failed to initialize encrypted test storage");
        Self {
            _root: root,
            database_path,
        }
    }

    pub fn open(&self) -> Connection {
        crate::database::open_encrypted_database(&self.database_path)
            .expect("failed to open encrypted test database")
    }
}

pub fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("failed to open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("failed to enable foreign keys");
    crate::database::run_migrations(&conn).expect("failed to run migrations");
    conn
}

pub fn setup_legacy_users_db() -> Connection {
    let conn = Connection::open_in_memory().expect("failed to open in-memory sqlite");
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        CREATE TABLE users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            role TEXT DEFAULT 'admin',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            deleted_at TEXT
        );
        PRAGMA foreign_keys = ON;
        ",
    )
    .expect("failed to create legacy users table");
    conn
}

pub fn create_temp_file(name: &str, extension: &str, bytes: &[u8]) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    path.push(format!("{}_{}.{}", name, nonce, extension));
    fs::write(&path, bytes).expect("failed to write temp file");
    path
}
