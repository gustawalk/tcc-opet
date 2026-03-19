use dotenv::dotenv;
use rusqlite::{Connection, Result, params};
use std::env;
use std::path::PathBuf;
use uuid::Uuid;
use once_cell::sync::Lazy;

// Static connection pool for simple desktop usage
static DB_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let _ = dotenv();
    get_database_path().expect("Failed to determine database path")
});

// Initialize the database connection
pub fn init_db() -> Result<()> {
    // Open the connection once to run migrations
    let conn = Connection::open(&*DB_PATH)?;
    run_migrations(&conn)?;
    Ok(())
}

// Get database path from environment or fallback
fn get_database_path() -> Result<PathBuf> {
    // Check for DATABASE_PATH environment variable first
    if let Ok(db_path) = env::var("DATABASE_PATH") {
        let path = PathBuf::from(db_path);
        if !path.is_absolute() {
            let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Ok(current_dir.join(path))
        } else {
            Ok(path)
        }
    } else {
        if let Ok(db_path) = env::var("DB_PATH") {
            let path = PathBuf::from(db_path);
            if !path.is_absolute() {
                let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                Ok(current_dir.join(path))
            } else {
                Ok(path)
            }
        } else {
            // Default path in the current directory for dev
            let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push("database.db");
            Ok(path)
        }
    }
}

// Run database migrations
fn run_migrations(conn: &Connection) -> Result<()> {
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
            role TEXT NOT NULL CHECK (role IN ('admin', 'tech')),
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
            sale_price REAL NOT NULL DEFAULT 0.0,
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
            signature_path TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT,
            closed_at TEXT,
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

        -- Index for faster snapshot queries by date
        CREATE INDEX IF NOT EXISTS idx_financial_snapshots_date ON financial_snapshots(snapshot_date);
        ",
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

    Ok(())
}

// Get database connection - returns a new connection using the stored path
pub fn get_db() -> Result<Connection> {
    Connection::open(&*DB_PATH)
}
