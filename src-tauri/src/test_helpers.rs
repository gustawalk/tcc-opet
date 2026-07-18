use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
