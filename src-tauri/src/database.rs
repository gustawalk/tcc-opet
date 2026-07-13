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
    
    // Initialize seed data if database is empty
    drop(conn); // Close the connection before seeding
    if let Err(e) = crate::seeds::initialize_seed_data() {
        eprintln!("[SEED ERROR] Seed initialization failed: {}", e);
    }
    
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
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (inventory_item_id) REFERENCES inventory_items (id)
        );

        -- Index for faster snapshot queries by date
        CREATE INDEX IF NOT EXISTS idx_financial_snapshots_date ON financial_snapshots(snapshot_date);

        -- Index for inventory movements lookup
        CREATE INDEX IF NOT EXISTS idx_inventory_movements_item ON inventory_movements(inventory_item_id);
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
                eprintln!("[MIGRATION WARNING] Could not run migration '{}': {}", migration.trim(), err_msg);
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
                eprintln!("[MIGRATION WARNING] Could not run migration '{}': {}", migration.trim(), err_msg);
            }
        }
    }

    // Migration: migrate users table from old schema (password, role) to new schema (phone, cpf, join_date)
    {
        let has_password_col: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'password'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map(|count| count > 0)
            .unwrap_or(false);

        if has_password_col {
            eprintln!("[MIGRATION] Migrating users table to new schema...");
            conn.execute_batch(
                "CREATE TABLE users_new (
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
                ALTER TABLE users_new RENAME TO users;"
            ).map_err(|e| eprintln!("[MIGRATION ERROR] Failed to migrate users table: {}", e)).ok();
            eprintln!("[MIGRATION] Users table migrated successfully.");
        }
    }

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
