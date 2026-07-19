use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, Result};
use std::env;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tauri::Manager;
use uuid::Uuid;

// Static connection pool for simple desktop usage
static DB_PATH: OnceCell<PathBuf> = OnceCell::new();

// Initialize the database connection
pub fn init_db(app: &tauri::App) -> Result<()> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error)))
    })?;
    let resolved_database_path = get_database_path(&app_data_dir)?;
    DB_PATH.set(resolved_database_path).map_err(|_| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(
            "Database path was already initialized.",
        )))
    })?;

    if let Some(parent) = database_path().parent() {
        ensure_private_dir(parent).map_err(io_error)?;
    }

    // Open the connection once to run migrations with foreign keys enabled.
    let conn = get_db()?;
    run_migrations(&conn)?;
    secure_private_file(&database_path()).map_err(io_error)?;

    drop(conn);
    if should_seed_demo_data() {
        crate::seeds::initialize_seed_data().map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
        })?;
    } else if cfg!(debug_assertions) {
        println!("[SEED] Demo seed data skipped by SKIP_DB_SEED.");
    } else {
        println!("[SEED] Demo seed data skipped in production.");
    }

    Ok(())
}

fn io_error(error: io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

pub(crate) fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub(crate) fn secure_private_file(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
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

// Get database path from environment or fallback
fn get_database_path(app_data_dir: &Path) -> Result<PathBuf> {
    let configured_path = env::var("DATABASE_PATH")
        .ok()
        .or_else(|| env::var("DB_PATH").ok())
        .map(PathBuf::from);
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

// Run full migrations: schema + core defaults
pub(crate) fn run_migrations(conn: &Connection) -> Result<()> {
    run_schema_migrations(conn)?;
    ensure_core_defaults(conn)?;
    Ok(())
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
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            closed_at TEXT,
            display_id TEXT NOT NULL DEFAULT '',
            discount_percent REAL NOT NULL DEFAULT 0.0,
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
            quantity INTEGER NOT NULL,
            unit_cost REAL NOT NULL,
            unit_price REAL NOT NULL,
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
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (inventory_item_id) REFERENCES inventory_items (id)
        );

        -- Index for faster snapshot queries by date
        CREATE INDEX IF NOT EXISTS idx_financial_snapshots_date ON financial_snapshots(snapshot_date);

        -- Index for inventory movements lookup
        CREATE INDEX IF NOT EXISTS idx_inventory_movements_item ON inventory_movements(inventory_item_id);
        CREATE INDEX IF NOT EXISTS idx_service_order_events_order ON service_order_events(service_order_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_service_order_attachments_order ON service_order_attachments(service_order_id, created_at DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_service_orders_display_id ON service_orders(display_id) WHERE display_id <> '';
        ",
    )?;

    // Migration: add columns to service_orders if missing
    for migration in &[
        "ALTER TABLE service_orders ADD COLUMN deleted_at TEXT;",
        "ALTER TABLE service_orders ADD COLUMN display_id TEXT NOT NULL DEFAULT '';",
        "ALTER TABLE service_orders ADD COLUMN discount_percent REAL NOT NULL DEFAULT 0.0;",
    ] {
        if let Err(e) = conn.execute_batch(migration) {
            let err_msg = e.to_string();
            if !err_msg.contains("duplicate column") {
                eprintln!(
                    "[MIGRATION WARNING] Could not run migration '{}': {}",
                    migration.trim(),
                    err_msg
                );
            }
        }
    }

    // Migration: add reason to legacy inventory movement records.
    if let Err(error) = conn.execute_batch(
        "ALTER TABLE inventory_movements ADD COLUMN reason TEXT NOT NULL DEFAULT '';",
    ) {
        if !error.to_string().contains("duplicate column") {
            eprintln!("[MIGRATION WARNING] Could not add inventory movement reason: {error}");
        }
    }

    // Additive inventory migrations preserve existing catalog and audit data.
    for migration in &[
        "ALTER TABLE inventory_items ADD COLUMN average_cost REAL NOT NULL DEFAULT 0.0;",
        "ALTER TABLE inventory_items ADD COLUMN supplier_name TEXT;",
        "ALTER TABLE inventory_movements ADD COLUMN unit_cost REAL;",
    ] {
        if let Err(error) = conn.execute_batch(migration) {
            if !error.to_string().contains("duplicate column") {
                eprintln!(
                    "[MIGRATION WARNING] Could not run inventory migration '{}': {error}",
                    migration.trim()
                );
            }
        }
    }

    // Migration: add columns to users if missing from intermediate schema
    for migration in &[
        "ALTER TABLE users ADD COLUMN phone TEXT DEFAULT '';",
        "ALTER TABLE users ADD COLUMN cpf TEXT DEFAULT '';",
        "ALTER TABLE users ADD COLUMN join_date TEXT DEFAULT '';",
    ] {
        if let Err(e) = conn.execute_batch(migration) {
            let err_msg = e.to_string();
            if !err_msg.contains("duplicate column") {
                eprintln!(
                    "[MIGRATION WARNING] Could not run migration '{}': {}",
                    migration.trim(),
                    err_msg
                );
            }
        }
    }

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
            )
            .map_err(|e| eprintln!("[MIGRATION ERROR] Failed to migrate users table: {}", e))
            .ok();
            eprintln!("[MIGRATION] Users table migrated successfully.");
        }
    }

    Ok(())
}

pub(crate) fn ensure_core_defaults(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE inventory_items SET average_cost = cost_price WHERE average_cost = 0.0 AND cost_price > 0.0",
        [],
    )?;

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

// Get database connection - returns a new connection using the stored path
pub fn get_db() -> Result<Connection> {
    let conn = Connection::open(database_path())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

pub fn database_path() -> PathBuf {
    DB_PATH
        .get()
        .cloned()
        .expect("Database path must be initialized before use")
}

pub fn attachments_dir() -> PathBuf {
    let mut path = database_path();
    path.set_extension("attachments");
    path
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
                    'idx_financial_snapshots_date', 'idx_inventory_movements_item'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(table_count, 14);
        assert_eq!(index_count, 2);
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

        let item: (f64, Option<String>) = conn
            .query_row(
                "SELECT average_cost, supplier_name FROM inventory_items WHERE id = 'part-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let has_unit_cost: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('inventory_movements') WHERE name = 'unit_cost'", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(item.0, 42.5);
        assert!(item.1.is_none());
        assert_eq!(has_unit_cost, 1);
    }
}
