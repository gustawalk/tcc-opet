//! Multi-user stress validation for the LAN shared database.
//!
//! These tests are `#[ignore]`d because they deliberately hammer the storage with
//! high concurrency. They answer two questions about the shared-folder mode:
//!   1. At which concurrency level do writes start failing (gracefully) with
//!      BUSY/LOCKED instead of completing, and
//!   2. whether any failure mode ever corrupts the database.
//!
//! Run the whole suite with:
//!   cargo test --lib -- --ignored storage_concurrency --nocapture
//!
//! To also drive the real shared database (while the desktop clients are
//! open), point the live test at it (a safe `VACUUM INTO` backup is taken first):
//!   STRESS_DB_PATH=/tmp/opets-lan/database.db \
//!     cargo test --lib -- --ignored live_shared_database --nocapture

use crate::database::{
    initialize_storage_at, open_encrypted_database_with_mode, set_lan_shared_mode_for_tests,
};
use rusqlite::{params, Connection, Error as SqlError, Transaction};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

const WORKERS: &[usize] = &[2, 4, 8, 16, 32, 64];
const DEFAULT_TOTAL_WRITES: usize = 2400;
const READERS: usize = 8;
const READER_PASSES: usize = 40;
const BUSY_TIMEOUT_MS: i64 = 30_000;

/// Total writer commits per matrix stage. Network filesystems (rollback journal
/// plus FULL synchronous) commit far slower than local WAL, so `STRESS_WRITES`
/// shrinks the volume for validation runs (e.g. `300`) without changing behaviour.
fn total_writes() -> usize {
    std::env::var("STRESS_WRITES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &usize| *value > 0)
        .unwrap_or(DEFAULT_TOTAL_WRITES)
}

fn worker_conn(path: &std::path::Path) -> Connection {
    worker_conn_with_busy_timeout(path, BUSY_TIMEOUT_MS)
}

fn worker_conn_with_busy_timeout(path: &std::path::Path, busy_timeout_ms: i64) -> Connection {
    let conn = open_encrypted_database_with_mode(path, true)
        .expect("failed to open LAN-mode connection for stress test");
    conn.execute_batch(&format!("PRAGMA busy_timeout = {busy_timeout_ms};"))
        .expect("failed to set busy timeout for stress connection");
    conn
}

fn enable_lan_mode_for_stress() {
    // Mirrors the production startup: the LAN config is loaded before init, so
    // seeding here must open its connections with the LAN pragmas too (WAL +
    // busy_timeout=30000), otherwise first-start migrations on a slow share
    // surface instant SQLITE_BUSY instead of waiting.
    set_lan_shared_mode_for_tests(true);
}

/// A batch of work that mirrors what one person does in the app: create a
/// customer, a part, restock it and open a service order with the part. The
/// whole batch is one IMMEDIATE transaction, like the app's command handlers.
fn writer_batch(conn: &Connection, worker: u64, batch: u64) -> Result<(), SqlError> {
    let transaction = Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;

    let now = chrono::Utc::now().to_rfc3339();
    let customer_id = Uuid::new_v4().to_string();
    let customer_name = format!("Cliente Carga {worker}-{batch}");
    let item_id = Uuid::new_v4().to_string();
    let item_name = format!("Peça Carga {worker}-{batch}");
    let order_id = Uuid::new_v4().to_string();

    transaction.execute(
        "INSERT INTO customers (id, name, phone, email, address, created_at, updated_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL)",
        params![
            customer_id,
            customer_name,
            "41955556666",
            format!("carga{worker}.{batch}@test.com"),
            "Rua Carga",
            now
        ],
    )?;

    let cost_cents: i64 = 1_000_000;
    let sale_cents: i64 = 2_500_000;
    transaction.execute(
        "INSERT INTO inventory_items (
            id, name, description, type, min_quantity, current_quantity,
            cost_price, average_cost, sale_price,
            cost_price_cents, average_cost_cents, sale_price_cents,
            supplier_name, created_at, updated_at, deleted_at)
         VALUES (?1, ?2, '', 'part', 1, 1, ?3, ?3, ?4, ?5, ?5, ?6, NULL, ?7, NULL, NULL)",
        params![
            item_id,
            item_name,
            10_000.0_f64,
            25_000.0_f64,
            cost_cents,
            sale_cents,
            now
        ],
    )?;

    transaction.execute(
        "INSERT INTO inventory_movements (
            id, inventory_item_id, type, quantity, reference_os_id, reason, unit_cost_cents, created_at)
         VALUES (?1, ?2, 'entrada', 1, NULL, 'manual_restock', ?3, ?4)",
        params![Uuid::new_v4().to_string(), item_id, cost_cents, now],
    )?;

    let next_number: i64 = transaction.query_row(
        "INSERT INTO service_order_sequences (name, value)
         VALUES ('service_order', 1)
         ON CONFLICT(name) DO UPDATE SET value = service_order_sequences.value + 1
         RETURNING value",
        [],
        |row| row.get(0),
    )?;
    let display_id = format!("OS-{:06}", next_number);

    transaction.execute(
        "INSERT INTO service_orders (
            id, customer_id, customer_name, user_id, equipment, imei, description, status,
            total_price_cents, created_at, updated_at, closed_at, display_id,
            discount_basis_points, created_date, finalized_date)
         VALUES (?1, ?2, ?3, NULL, ?4, NULL, ?5, 'Orçamento', ?6, ?7, ?7, NULL, ?8, 0,
                 date(?7, 'localtime'), NULL)",
        params![
            order_id,
            customer_id,
            customer_name,
            format!("Equipamento Carga {worker}-{batch}"),
            "OS de estresse de escrita",
            sale_cents,
            now,
            display_id
        ],
    )?;

    transaction.execute(
        "INSERT INTO service_order_events (id, service_order_id, event_type, details, created_at)
         VALUES (?1, ?2, 'created', ?3, ?4)",
        params![
            Uuid::new_v4().to_string(),
            order_id,
            serde_json::json!({ "status": "Orçamento" }).to_string(),
            now
        ],
    )?;

    transaction.execute(
        "INSERT INTO service_order_parts (
            id, service_order_id, inventory_item_id, inventory_item_name, item_type,
            quantity, unit_cost_cents, unit_price_cents, stock_restored)
         VALUES (?1, ?2, ?3, ?4, 'part', 1, ?5, ?6, 0)",
        params![
            Uuid::new_v4().to_string(),
            order_id,
            item_id,
            item_name,
            cost_cents,
            sale_cents
        ],
    )?;

    transaction.commit()
}

/// Dashboard-like read workload: browsing users never block in WAL mode, but the
/// test still records any read errors that show up under heavy write contention.
fn reader_pass(conn: &Connection) -> Result<(), SqlError> {
    conn.query_row(
        "SELECT COUNT(*) FROM customers WHERE deleted_at IS NULL",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    conn.query_row(
        "SELECT COALESCE(SUM(total_price_cents), 0) FROM service_orders WHERE deleted_at IS NULL",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    conn.query_row("SELECT COUNT(*) FROM service_order_parts", [], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(())
}

fn classify(error: &SqlError) -> &'static str {
    if let SqlError::SqliteFailure(err, _) = error {
        if err.code == rusqlite::ErrorCode::DatabaseBusy {
            return "busy";
        }
        if err.code == rusqlite::ErrorCode::DatabaseLocked {
            return "locked";
        }
        return "other";
    }
    "other"
}

#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    orders: i64,
    seq: i64,
    distinct_display_ids: i64,
}

fn snapshot(path: &std::path::Path) -> Counts {
    let conn = worker_conn(path);
    let read = |query: &str| conn.query_row(query, [], |row| row.get::<_, i64>(0));
    Counts {
        orders: read("SELECT COUNT(*) FROM service_orders").unwrap_or(-1),
        seq: read("SELECT value FROM service_order_sequences WHERE name = 'service_order'")
            .unwrap_or(-1),
        distinct_display_ids: read("SELECT COUNT(DISTINCT display_id) FROM service_orders")
            .unwrap_or(-1),
    }
}

fn invariant_holds(before: &Counts, after: &Counts) -> bool {
    // The demo seed creates orders with a constant display_id (it never touches
    // the OS sequence), so only the deltas are meaningful. Every order created
    // during the storm must consume exactly one atomic sequence increment, and
    // every display id minted during the storm must be unique. Each of the N
    // new orders therefore shows up as +1 order, +1 sequence value and +1
    // distinct display id.
    let added_orders = match after.orders.checked_sub(before.orders) {
        Some(added) if added >= 0 => added,
        _ => return false,
    };
    added_orders == after.seq.saturating_sub(before.seq)
        && added_orders
            == after
                .distinct_display_ids
                .saturating_sub(before.distinct_display_ids)
}

fn integrity(path: &std::path::Path) -> String {
    let conn = worker_conn(path);
    conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap_or_else(|error| format!("error: {error}"))
}

fn run_writer_storm(
    path: &std::path::Path,
    workers: usize,
    busy_timeout_ms: i64,
) -> (usize, usize, usize, usize, usize, Duration) {
    let wraps_per_worker = total_writes() / workers;
    let start_line = std::sync::Arc::new(std::sync::Barrier::new(workers + READERS));
    let mut handles = Vec::with_capacity(workers);
    for worker in 0..workers {
        let owned_path = path.to_path_buf();
        let start_line = start_line.clone();
        handles.push(std::thread::spawn(move || {
            let conn = worker_conn_with_busy_timeout(&owned_path, busy_timeout_ms);
            start_line.wait();
            let (mut busy, mut locked, mut other) = (0usize, 0usize, 0usize);
            let mut count = 0usize;
            for batch in 0..wraps_per_worker {
                count += 1;
                if let Err(error) = writer_batch(&conn, worker as u64, batch as u64) {
                    match classify(&error) {
                        "busy" => busy += 1,
                        "locked" => locked += 1,
                        _ => other += 1,
                    }
                }
            }
            (count, busy, locked, other)
        }));
    }

    let mut reader_handles = Vec::with_capacity(READERS);
    for _ in 0..READERS {
        let owned_path = path.to_path_buf();
        let start_line = start_line.clone();
        reader_handles.push(std::thread::spawn(move || {
            let conn = worker_conn_with_busy_timeout(&owned_path, busy_timeout_ms);
            start_line.wait();
            let mut errors = 0usize;
            for _ in 0..READER_PASSES {
                if reader_pass(&conn).is_err() {
                    errors += 1;
                }
            }
            errors
        }));
    }

    let start = Instant::now();
    let (mut attempts, mut busy, mut locked, mut other) = (0usize, 0usize, 0usize, 0usize);
    for handle in handles {
        let (count, b, l, o) = handle.join().expect("writer thread panicked");
        attempts += count;
        busy += b;
        locked += l;
        other += o;
    }
    let mut reader_errors = 0usize;
    for handle in reader_handles {
        reader_errors += handle.join().expect("reader thread panicked");
    }
    let elapsed = start.elapsed();
    (attempts, busy, locked, other, reader_errors, elapsed)
}

#[test]
#[ignore]
fn write_storm_scale_and_integrity() {
    // By default runs on a local temp dir; set STRESS_DB_PATH to point the
    // matrix at an explicit database (e.g. a CIFS/SMB share) — the file is
    // recreated fresh so the run is reproducible.
    let _local_root;
    let database = match std::env::var("STRESS_DB_PATH") {
        Ok(path) => {
            let path = PathBuf::from(path);
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(path.with_extension("db-wal"));
            let _ = std::fs::remove_file(path.with_extension("db-shm"));
            path
        }
        Err(_) => {
            _local_root = tempfile::tempdir().expect("failed to create stress temp dir");
            _local_root.path().join("database.db")
        }
    };
    enable_lan_mode_for_stress();
    initialize_storage_at(&database, true).expect("failed to seed stress database");

    let baseline = snapshot(&database);
    assert!(
        baseline.orders > 0,
        "seed must create at least one service order"
    );

    println!(
        "{:>6} | {:>5} | {:>5} | {:>4} | {:>4} | {:>4} | {:>4} | {:>6} | {:>12} | {:>5}",
        "workers", "writes", "busy", "lock", "other", "rdErr", "rows", "wall_s", "integrity", "inv"
    );
    for &workers in WORKERS {
        let before = snapshot(&database);
        let (attempts, busy, locked, other, reader_errors, elapsed) =
            run_writer_storm(&database, workers, BUSY_TIMEOUT_MS);
        let after = snapshot(&database);
        let integrity_result = integrity(&database);
        let inv = invariant_holds(&before, &after);
        let added = after.orders - before.orders;
        println!(
            "{workers:>6} | {attempts:>5} | {busy:>5} | {locked:>4} | {other:>4} | {reader_errors:>4} | {added:>4} | {:.2}   | {integrity_result:>12} | {inv:>5}",
            elapsed.as_secs_f64()
        );
        assert_eq!(
            integrity_result, "ok",
            "database corrupted at {workers} workers"
        );
        assert_eq!(
            other, 0,
            "unexpected sqlite errors at {workers} workers: {other}"
        );
        assert!(
            inv,
            "sequence invariant broken at {workers} workers: +{added} orders",
        );
        assert!(
            reader_errors == 0,
            "readers failed at {workers} workers: {reader_errors}"
        );
    }
}

#[test]
#[ignore]
fn commit_backlog_exceeding_busy_timeout_fails_cleanly_without_corruption() {
    // Also honors STRESS_DB_PATH so the degraded path can be exercised on a
    // real CIFS/SMB share; defaults to a local temp dir.
    let _local_root;
    let database = match std::env::var("STRESS_DB_PATH") {
        Ok(path) => {
            let path = PathBuf::from(path);
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(path.with_extension("db-wal"));
            let _ = std::fs::remove_file(path.with_extension("db-shm"));
            path
        }
        Err(_) => {
            _local_root = tempfile::tempdir().expect("failed to create stress temp dir");
            _local_root.path().join("database.db")
        }
    };
    enable_lan_mode_for_stress();
    initialize_storage_at(&database, true).expect("failed to seed stress database");

    const WRAPPERS: usize = 64;
    const WRAPPS_PER_WRAPPER: usize = 60;
    // A tiny busy timeout (the app ships 30s) makes the commit queue saturate
    // deterministically under a simultaneous write burst, proving the degraded
    // path: writers surface BUSY errors, the data keeps its integrity, and no
    // "other" error ever appears.
    const SHORT_BUSY_TIMEOUT_MS: i64 = 400;

    let start_line = std::sync::Arc::new(std::sync::Barrier::new(WRAPPERS));
    let mut handles = Vec::with_capacity(WRAPPERS);
    for worker in 0..WRAPPERS {
        let owned_path = database.clone();
        let start_line = start_line.clone();
        handles.push(std::thread::spawn(move || {
            let conn = worker_conn_with_busy_timeout(&owned_path, SHORT_BUSY_TIMEOUT_MS);
            start_line.wait();
            let mut busy = 0usize;
            let mut locked = 0usize;
            let mut other = 0usize;
            for batch in 0..WRAPPS_PER_WRAPPER {
                if let Err(error) = writer_batch(&conn, worker as u64, batch as u64) {
                    match classify(&error) {
                        "busy" => busy += 1,
                        "locked" => locked += 1,
                        _ => other += 1,
                    }
                }
            }
            (busy, locked, other)
        }));
    }

    let before = snapshot(&database);
    let start = Instant::now();
    let (mut busy, mut locked, mut other) = (0usize, 0usize, 0usize);
    for handle in handles {
        let (b, l, o) = handle.join().expect("writer thread panicked");
        busy += b;
        locked += l;
        other += o;
    }
    let elapsed = start.elapsed();
    let after = snapshot(&database);
    let integrity_result = integrity(&database);

    println!(
        "short-busy queue: busy={busy} locked={locked} other={other} wall_s={:.2} integrity={integrity_result} added_orders={}",
        elapsed.as_secs_f64(),
        after.orders - before.orders
    );
    assert_eq!(
        integrity_result, "ok",
        "database corrupted under degraded path"
    );
    assert_eq!(
        other, 0,
        "unexpected sqlite errors under degraded path: {other}"
    );
    assert!(
        busy + locked >= 1,
        "the commit backlog should exceed the short busy timeout at least once"
    );
    assert!(invariant_holds(&before, &after));
}

#[test]
#[ignore]
fn live_shared_database_under_additional_writers() {
    let Ok(path) = std::env::var("STRESS_DB_PATH") else {
        println!(
            "skipped: set STRESS_DB_PATH to point at the live shared database, e.g. /tmp/opets-lan/database.db"
        );
        return;
    };
    let database = PathBuf::from(path);
    assert!(
        database.is_file(),
        "STRESS_DB_PATH must be an existing file"
    );

    // Safe restore point before touching the clients' live storage.
    let backup = database.with_extension("db.stress-backup.db");
    match worker_conn(&database).execute_batch(&format!(
        "PRAGMA busy_timeout = 60000; VACUUM INTO '{}'",
        backup.to_string_lossy().replace('\'', "''")
    )) {
        Ok(()) => println!("backup created at {}", backup.display()),
        Err(error) => println!("warning: VACUUM INTO backup failed: {error} (continuing)"),
    }

    let before = snapshot(&database);
    println!("live database before: {before:?}");
    let (attempts, busy, locked, other, reader_errors, elapsed) =
        run_writer_storm(&database, 16, BUSY_TIMEOUT_MS);
    let after = snapshot(&database);
    let integrity_result = integrity(&database);
    println!(
        "live: attempts={attempts} busy={busy} locked={locked} other={other} reader_errors={reader_errors} wall_s={:.2} integrity={integrity_result} added_orders={}",
        elapsed.as_secs_f64(),
        after.orders - before.orders
    );
    assert_eq!(integrity_result, "ok");
    assert_eq!(other, 0);
    assert!(invariant_holds(&before, &after));
}
