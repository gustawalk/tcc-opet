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
use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tauri::Manager;
use uuid::Uuid;

// Static connection pool for simple desktop usage
static DB_PATH: OnceCell<PathBuf> = OnceCell::new();
static STORAGE_INSTANCE_LOCK: OnceCell<File> = OnceCell::new();
static STORAGE_OPERATION_LOCK: LazyLock<RwLock<()>> = LazyLock::new(|| RwLock::new(()));
const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";
const STORAGE_FORMAT_VERSION: u8 = 1;

pub struct DatabaseConnection {
    connection: Connection,
    _guard: RwLockReadGuard<'static, ()>,
}

impl Deref for DatabaseConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl DerefMut for DatabaseConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageMetadata {
    format_version: u8,
    key_version: u8,
    authentication: String,
}

// Initialize the database connection
pub fn init_db(app: &tauri::App) -> Result<()> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error)))
    })?;
    let resolved_database_path = get_database_path(&app_data_dir)?;
    if let Some(parent) = resolved_database_path.parent() {
        ensure_private_dir(parent).map_err(io_error)?;
    }
    acquire_storage_instance_lock(&resolved_database_path)?;
    initialize_storage_at(&resolved_database_path, should_seed_demo_data())?;
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
    let conn = Connection::open(path)?;
    let key = database_key_hex();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{key}'\"; PRAGMA cipher_memory_security = ON; PRAGMA foreign_keys = ON;"
    ))?;
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
    let transaction = conn.unchecked_transaction()?;
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
        add_column_if_missing(&transaction, table, column, definition)?;
    }
    transaction.execute_batch(
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
    make_legacy_part_prices_optional(&transaction)?;
    validate_integer_money(&transaction)?;
    transaction.commit()
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
    let connection = open_encrypted_database(&database_path())?;
    Ok(DatabaseConnection {
        connection,
        _guard: guard,
    })
}

pub(crate) fn exclusive_storage_guard() -> Result<RwLockWriteGuard<'static, ()>> {
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
