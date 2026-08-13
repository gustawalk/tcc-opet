use crate::database::{get_db, open_encrypted_database};
use crate::page::Page;
use crate::repositories::checklist_repo::ChecklistRepository;
use crate::repositories::customer_repo::CustomerRepository;
use crate::repositories::financial_report_repo::FinancialReportRepository;
use crate::repositories::inventory_repo::InventoryRepository;
use crate::repositories::service_order_repo::ServiceOrderRepository;
use crate::repositories::user_repo::UserRepository;
use crate::test_helpers::setup_global_backend;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::hint::black_box;
use std::time::{Duration, Instant};

const RECORD_COUNT: usize = 10_000;
const PAGE_SIZE: u32 = 20;
const WARMUPS: usize = 3;
const ITERATIONS: usize = 15;
const CONNECTION_ITERATIONS: usize = 50;

#[derive(Clone, Copy)]
struct Stats {
    median: Duration,
    p95: Duration,
    min: Duration,
    max: Duration,
}

struct Comparison {
    name: &'static str,
    baseline: Stats,
    optimized: Stats,
    baseline_bytes: usize,
    optimized_bytes: usize,
}

fn measure<T>(warmups: usize, iterations: usize, mut operation: impl FnMut() -> T) -> Stats {
    for _ in 0..warmups {
        black_box(operation());
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let result = operation();
        let elapsed = start.elapsed();
        black_box(&result);
        samples.push(elapsed);
    }
    samples.sort_unstable();
    let p95_index = ((samples.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(samples.len() - 1);
    Stats {
        median: samples[samples.len() / 2],
        p95: samples[p95_index],
        min: samples[0],
        max: samples[samples.len() - 1],
    }
}

fn json<T: Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}

fn print_comparison(comparison: &Comparison) {
    let speedup = comparison.baseline.median.as_secs_f64()
        / comparison.optimized.median.as_secs_f64().max(f64::EPSILON);
    let payload_reduction = if comparison.baseline_bytes == 0 {
        0.0
    } else {
        (1.0 - comparison.optimized_bytes as f64 / comparison.baseline_bytes as f64) * 100.0
    };
    println!(
        "| {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.2}x | {} | {} | {:.1}% |",
        comparison.name,
        micros(comparison.baseline.median),
        micros(comparison.baseline.p95),
        micros(comparison.optimized.median),
        micros(comparison.optimized.p95),
        speedup,
        comparison.baseline_bytes,
        comparison.optimized_bytes,
        payload_reduction,
    );
}

fn seed_benchmark_data(conn: &mut Connection) {
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    let tx = conn.transaction().unwrap();

    {
        let mut customer = tx
            .prepare(
                "INSERT INTO customers (id, name, phone, email, address, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .unwrap();
        let mut user = tx
            .prepare(
                "INSERT INTO users (id, name, email, phone, cpf, join_date, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, '2024-01-01', ?6)",
            )
            .unwrap();
        let mut inventory = tx
            .prepare(
                "INSERT INTO inventory_items (
                    id, name, description, type, min_quantity, current_quantity,
                    cost_price_cents, average_cost_cents, sale_price_cents,
                    supplier_name, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9, ?10)",
            )
            .unwrap();
        let mut template = tx
            .prepare("INSERT INTO checklist_templates (id, title, created_at) VALUES (?1, ?2, ?3)")
            .unwrap();
        let mut template_item = tx
            .prepare("INSERT INTO template_items (id, template_id, label) VALUES (?1, ?2, ?3)")
            .unwrap();

        for index in 0..RECORD_COUNT {
            let timestamp = format!(
                "2024-{:02}-{:02}T12:00:00+00:00",
                index % 12 + 1,
                index % 28 + 1
            );
            customer
                .execute(params![
                    format!("customer-{index:05}"),
                    format!("Cliente {index:05}"),
                    format!("419{:08}", index),
                    format!("cliente{index:05}@example.com"),
                    format!("Rua de Benchmark, {index}"),
                    timestamp,
                ])
                .unwrap();
            user.execute(params![
                format!("user-{index:05}"),
                format!("Funcionário {index:05}"),
                format!("funcionario{index:05}@example.com"),
                format!("419{:08}", RECORD_COUNT + index),
                format!("{:011}", index),
                timestamp,
            ])
            .unwrap();

            let item_type = if index % 2 == 0 { "part" } else { "service" };
            inventory
                .execute(params![
                    format!("inventory-{index:05}"),
                    format!("Item {index:05}"),
                    format!("Descrição detalhada do item de benchmark {index:05}"),
                    item_type,
                    if item_type == "part" { 5 } else { 0 },
                    if item_type == "part" {
                        (index % 20) as i64
                    } else {
                        0
                    },
                    1_000 + index as i64,
                    2_000 + index as i64,
                    format!("Fornecedor {:03}", index % 100),
                    timestamp,
                ])
                .unwrap();

            let template_id = format!("template-{index:05}");
            template
                .execute(params![
                    template_id,
                    format!("Modelo de checklist {index:05}"),
                    timestamp,
                ])
                .unwrap();
            for item_index in 0..3 {
                template_item
                    .execute(params![
                        format!("template-item-{index:05}-{item_index}"),
                        template_id,
                        format!("Verificação {item_index}"),
                    ])
                    .unwrap();
            }
        }
    }

    {
        let mut order = tx
            .prepare(
                "INSERT INTO service_orders (
                    id, customer_id, customer_name, user_id, equipment, imei,
                    description, status, total_price_cents, created_at, created_date,
                    closed_at, finalized_date, display_id, discount_basis_points
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, date(?10),
                    ?11, ?12, ?13, ?14
                 )",
            )
            .unwrap();
        let mut part = tx
            .prepare(
                "INSERT INTO service_order_parts (
                    id, service_order_id, inventory_item_id, inventory_item_name,
                    item_type, quantity, unit_cost_cents, unit_price_cents, stock_restored
                 ) VALUES (?1, ?2, ?3, ?4, 'part', ?5, ?6, ?7, 0)",
            )
            .unwrap();

        for index in 0..RECORD_COUNT {
            let month = index % 12 + 1;
            let day = index % 28 + 1;
            let created_at = format!("2024-{month:02}-{day:02}T12:00:00+00:00");
            let finalized = index % 2 == 0;
            let status = if finalized {
                "Finalizada"
            } else {
                "Em Manutenção"
            };
            let closed_at = finalized.then(|| format!("2024-{month:02}-{day:02}T18:00:00+00:00"));
            let finalized_date = finalized.then(|| format!("2024-{month:02}-{day:02}"));
            let order_id = format!("order-{index:05}");
            order
                .execute(params![
                    order_id,
                    format!("customer-{index:05}"),
                    format!("Cliente {index:05}"),
                    format!("user-{index:05}"),
                    format!("Equipamento {index:05}"),
                    format!("{:015}", index),
                    format!("Descrição da ordem de serviço {index:05}"),
                    status,
                    30_000 + index as i64,
                    created_at,
                    closed_at,
                    finalized_date,
                    format!("OS-{index:06}"),
                    (index % 1_500) as i64,
                ])
                .unwrap();
            for part_index in 0..3 {
                let inventory_index = ((index + part_index * 2) % RECORD_COUNT) & !1;
                part.execute(params![
                    format!("order-part-{index:05}-{part_index}"),
                    order_id,
                    format!("inventory-{inventory_index:05}"),
                    format!("Item {inventory_index:05}"),
                    part_index + 1,
                    1_000 + inventory_index as i64,
                    4_000 + inventory_index as i64,
                ])
                .unwrap();
            }
        }
    }

    tx.commit().unwrap();
    conn.execute_batch("ANALYZE;").unwrap();
}

fn old_financial_summary(conn: &Connection) -> (i64, i64, i64, i64) {
    let mut orders = conn
        .prepare(
            "SELECT so.id, so.total_price_cents, so.discount_basis_points
             FROM service_orders so
             WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL
               AND date(COALESCE(so.closed_at, so.created_at), 'localtime')
                   BETWEEN date('2024-01-01') AND date('2024-12-31')",
        )
        .unwrap();
    let rows = orders
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .unwrap();
    let mut revenue = 0_i64;
    let mut cost = 0_i64;
    let mut count = 0_i64;
    let mut discounts = 0_i64;
    for row in rows {
        let (order_id, gross, basis_points) = row.unwrap();
        let discounted = (gross * (10_000 - basis_points) + 5_000) / 10_000;
        revenue += discounted;
        discounts += gross - discounted;
        count += 1;
        let mut parts = conn
            .prepare(
                "SELECT quantity, unit_cost_cents FROM service_order_parts WHERE service_order_id = ?1",
            )
            .unwrap();
        let part_rows = parts
            .query_map([order_id], |part| {
                Ok((part.get::<_, i64>(0)?, part.get::<_, i64>(1)?))
            })
            .unwrap();
        for part in part_rows {
            let (quantity, unit_cost) = part.unwrap();
            cost += quantity * unit_cost;
        }
    }
    (revenue, cost, count, discounts)
}

fn optimized_financial_summary(conn: &Connection) -> (i64, i64, i64, i64) {
    let mut stmt = conn
        .prepare(
            "SELECT so.total_price_cents, so.discount_basis_points,
                    COALESCE(SUM(sop.quantity * sop.unit_cost_cents), 0)
             FROM service_orders so
             LEFT JOIN service_order_parts sop ON sop.service_order_id = so.id
             WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL
               AND so.finalized_date BETWEEN date('2024-01-01') AND date('2024-12-31')
             GROUP BY so.id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .unwrap();
    let mut revenue = 0_i64;
    let mut cost = 0_i64;
    let mut count = 0_i64;
    let mut discounts = 0_i64;
    for row in rows {
        let (gross, basis_points, order_cost) = row.unwrap();
        let discounted = (gross * (10_000 - basis_points) + 5_000) / 10_000;
        revenue += discounted;
        discounts += gross - discounted;
        cost += order_cost;
        count += 1;
    }
    (revenue, cost, count, discounts)
}

fn explain_plan(conn: &Connection, sql: &str) -> Vec<String> {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
    stmt.query_map([], |row| row.get::<_, String>(3))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
#[ignore = "performance benchmark; run explicitly with --release --ignored --nocapture"]
fn performance_benchmarks() {
    let backend = setup_global_backend();
    {
        let mut conn = get_db().unwrap();
        seed_benchmark_data(&mut conn);
    }

    println!("\n# Optimization benchmark");
    println!("records per primary entity: {RECORD_COUNT}");
    println!("page size: {PAGE_SIZE}; warmups: {WARMUPS}; iterations: {ITERATIONS}");

    let baseline_connection = measure(3, CONNECTION_ITERATIONS, || {
        let conn = open_encrypted_database(backend.database_path()).unwrap();
        conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
            .unwrap()
    });
    let optimized_connection = measure(10, CONNECTION_ITERATIONS, || {
        let conn = get_db().unwrap();
        conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
            .unwrap()
    });

    println!("\n## Connection acquisition");
    println!("| path | median us | p95 us | min us | max us |");
    println!("|---|---:|---:|---:|---:|");
    println!(
        "| reopen SQLCipher | {:.1} | {:.1} | {:.1} | {:.1} |",
        micros(baseline_connection.median),
        micros(baseline_connection.p95),
        micros(baseline_connection.min),
        micros(baseline_connection.max),
    );
    println!(
        "| shared get_db | {:.1} | {:.1} | {:.1} | {:.1} |",
        micros(optimized_connection.median),
        micros(optimized_connection.p95),
        micros(optimized_connection.min),
        micros(optimized_connection.max),
    );

    let mut comparisons = Vec::new();

    let baseline_payload = {
        let conn = get_db().unwrap();
        json(&CustomerRepository::get_all_with_conn(&conn).unwrap())
    };
    let optimized_payload = {
        let conn = get_db().unwrap();
        let items = CustomerRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "").unwrap();
        let total = CustomerRepository::count_all_with_conn(&conn, "").unwrap();
        json(&Page { items, total })
    };
    comparisons.push(Comparison {
        name: "customers endpoint",
        baseline: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            json(&CustomerRepository::get_all_with_conn(&conn).unwrap())
        }),
        optimized: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let items = CustomerRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "").unwrap();
            let total = CustomerRepository::count_all_with_conn(&conn, "").unwrap();
            json(&Page { items, total })
        }),
        baseline_bytes: baseline_payload.len(),
        optimized_bytes: optimized_payload.len(),
    });

    let baseline_payload = {
        let conn = get_db().unwrap();
        json(&UserRepository::get_all_with_conn(&conn).unwrap())
    };
    let optimized_payload = {
        let conn = get_db().unwrap();
        let items = UserRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "").unwrap();
        let total = UserRepository::count_all_with_conn(&conn, "").unwrap();
        json(&Page { items, total })
    };
    comparisons.push(Comparison {
        name: "users endpoint",
        baseline: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            json(&UserRepository::get_all_with_conn(&conn).unwrap())
        }),
        optimized: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let items = UserRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "").unwrap();
            let total = UserRepository::count_all_with_conn(&conn, "").unwrap();
            json(&Page { items, total })
        }),
        baseline_bytes: baseline_payload.len(),
        optimized_bytes: optimized_payload.len(),
    });

    let baseline_payload = {
        let conn = get_db().unwrap();
        json(&ChecklistRepository::get_templates_with_conn(&conn).unwrap())
    };
    let optimized_payload = {
        let conn = get_db().unwrap();
        let items = ChecklistRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "").unwrap();
        let total = ChecklistRepository::count_all_with_conn(&conn, "").unwrap();
        json(&Page { items, total })
    };
    comparisons.push(Comparison {
        name: "templates endpoint",
        baseline: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            json(&ChecklistRepository::get_templates_with_conn(&conn).unwrap())
        }),
        optimized: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let items = ChecklistRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "").unwrap();
            let total = ChecklistRepository::count_all_with_conn(&conn, "").unwrap();
            json(&Page { items, total })
        }),
        baseline_bytes: baseline_payload.len(),
        optimized_bytes: optimized_payload.len(),
    });

    let baseline_payload = {
        let conn = get_db().unwrap();
        json(&InventoryRepository::get_all_with_conn(&conn).unwrap())
    };
    let optimized_payload = {
        let conn = get_db().unwrap();
        let parts =
            InventoryRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "", Some("part")).unwrap();
        let parts_total =
            InventoryRepository::count_all_with_conn(&conn, "", Some("part")).unwrap();
        let services =
            InventoryRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "", Some("service"))
                .unwrap();
        let services_total =
            InventoryRepository::count_all_with_conn(&conn, "", Some("service")).unwrap();
        let summary = InventoryRepository::get_summary_with_conn(&conn).unwrap();
        json(&(
            Page {
                items: parts,
                total: parts_total,
            },
            Page {
                items: services,
                total: services_total,
            },
            summary,
        ))
    };
    comparisons.push(Comparison {
        name: "inventory screen",
        baseline: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            json(&InventoryRepository::get_all_with_conn(&conn).unwrap())
        }),
        optimized: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let parts =
                InventoryRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "", Some("part"))
                    .unwrap();
            let parts_total =
                InventoryRepository::count_all_with_conn(&conn, "", Some("part")).unwrap();
            let services =
                InventoryRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "", Some("service"))
                    .unwrap();
            let services_total =
                InventoryRepository::count_all_with_conn(&conn, "", Some("service")).unwrap();
            let summary = InventoryRepository::get_summary_with_conn(&conn).unwrap();
            json(&(
                Page {
                    items: parts,
                    total: parts_total,
                },
                Page {
                    items: services,
                    total: services_total,
                },
                summary,
            ))
        }),
        baseline_bytes: baseline_payload.len(),
        optimized_bytes: optimized_payload.len(),
    });

    let baseline_payload = {
        let conn = get_db().unwrap();
        json(&ServiceOrderRepository::get_all_with_conn(&conn).unwrap())
    };
    let optimized_payload = {
        let conn = get_db().unwrap();
        let items =
            ServiceOrderRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "", None, None, None)
                .unwrap();
        let total =
            ServiceOrderRepository::count_all_with_conn(&conn, "", None, None, None).unwrap();
        json(&Page { items, total })
    };
    comparisons.push(Comparison {
        name: "service orders endpoint",
        baseline: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            json(&ServiceOrderRepository::get_all_with_conn(&conn).unwrap())
        }),
        optimized: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let items = ServiceOrderRepository::get_page_with_conn(
                &conn, PAGE_SIZE, 0, "", None, None, None,
            )
            .unwrap();
            let total =
                ServiceOrderRepository::count_all_with_conn(&conn, "", None, None, None).unwrap();
            json(&Page { items, total })
        }),
        baseline_bytes: baseline_payload.len(),
        optimized_bytes: optimized_payload.len(),
    });

    let baseline_payload = {
        let conn = get_db().unwrap();
        let orders = ServiceOrderRepository::get_all_with_conn(&conn).unwrap();
        let customers = CustomerRepository::get_all_with_conn(&conn).unwrap();
        let users = UserRepository::get_all_with_conn(&conn).unwrap();
        json(&(orders, customers, users))
    };
    let optimized_payload = {
        let conn = get_db().unwrap();
        let items =
            ServiceOrderRepository::get_page_with_conn(&conn, PAGE_SIZE, 0, "", None, None, None)
                .unwrap();
        let total =
            ServiceOrderRepository::count_all_with_conn(&conn, "", None, None, None).unwrap();
        let customers = CustomerRepository::get_all_with_conn(&conn).unwrap();
        let users = UserRepository::get_all_with_conn(&conn).unwrap();
        json(&(Page { items, total }, customers, users))
    };
    comparisons.push(Comparison {
        name: "service orders screen",
        baseline: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let orders = ServiceOrderRepository::get_all_with_conn(&conn).unwrap();
            let customers = CustomerRepository::get_all_with_conn(&conn).unwrap();
            let users = UserRepository::get_all_with_conn(&conn).unwrap();
            json(&(orders, customers, users))
        }),
        optimized: measure(WARMUPS, ITERATIONS, || {
            let conn = get_db().unwrap();
            let items = ServiceOrderRepository::get_page_with_conn(
                &conn, PAGE_SIZE, 0, "", None, None, None,
            )
            .unwrap();
            let total =
                ServiceOrderRepository::count_all_with_conn(&conn, "", None, None, None).unwrap();
            let customers = CustomerRepository::get_all_with_conn(&conn).unwrap();
            let users = UserRepository::get_all_with_conn(&conn).unwrap();
            json(&(Page { items, total }, customers, users))
        }),
        baseline_bytes: baseline_payload.len(),
        optimized_bytes: optimized_payload.len(),
    });

    println!("\n## Listing and screen comparisons");
    println!("| scenario | baseline median us | baseline p95 us | optimized median us | optimized p95 us | speedup | baseline bytes | optimized bytes | payload reduction |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for comparison in &comparisons {
        print_comparison(comparison);
    }

    let old_date_sql = "SELECT COUNT(*) FROM service_orders so
        WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL
          AND date(COALESCE(so.closed_at, so.created_at), 'localtime')
              BETWEEN date('2024-01-01') AND date('2024-12-31')";
    let new_date_sql = "SELECT COUNT(*) FROM service_orders so
        WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL
          AND so.finalized_date BETWEEN date('2024-01-01') AND date('2024-12-31')";
    let old_date_count = {
        let conn = get_db().unwrap();
        conn.query_row(old_date_sql, [], |row| row.get::<_, i64>(0))
            .unwrap()
    };
    let new_date_count = {
        let conn = get_db().unwrap();
        conn.query_row(new_date_sql, [], |row| row.get::<_, i64>(0))
            .unwrap()
    };
    assert_eq!(old_date_count, new_date_count);
    let old_date_stats = measure(WARMUPS, ITERATIONS, || {
        let conn = get_db().unwrap();
        conn.query_row(old_date_sql, [], |row| row.get::<_, i64>(0))
            .unwrap()
    });
    let new_date_stats = measure(WARMUPS, ITERATIONS, || {
        let conn = get_db().unwrap();
        conn.query_row(new_date_sql, [], |row| row.get::<_, i64>(0))
            .unwrap()
    });

    {
        let conn = get_db().unwrap();
        assert_eq!(
            old_financial_summary(&conn),
            optimized_financial_summary(&conn)
        );
    }
    // One N+1 sample already executes 5,000 per-order part queries on this data set.
    // Repeating it like the sub-millisecond cases makes the explicit benchmark
    // impractically long without adding useful signal.
    let old_summary_stats = measure(0, 1, || {
        let conn = get_db().unwrap();
        old_financial_summary(&conn)
    });
    let new_summary_stats = measure(1, 5, || {
        let conn = get_db().unwrap();
        optimized_financial_summary(&conn)
    });
    let full_report_stats = measure(0, 1, || {
        let conn = get_db().unwrap();
        json(
            &FinancialReportRepository::get_report_with_conn_filtered(
                &conn,
                Some("2024-01-01"),
                Some("2024-12-31"),
                None,
                Some("revenue"),
                Some(10),
            )
            .unwrap(),
        )
    });

    println!("\n## Financial hot spots");
    println!("| scenario | baseline median us | baseline p95 us | optimized median us | optimized p95 us | speedup |");
    println!("|---|---:|---:|---:|---:|---:|");
    println!(
        "| date filter | {:.1} | {:.1} | {:.1} | {:.1} | {:.2}x |",
        micros(old_date_stats.median),
        micros(old_date_stats.p95),
        micros(new_date_stats.median),
        micros(new_date_stats.p95),
        old_date_stats.median.as_secs_f64() / new_date_stats.median.as_secs_f64(),
    );
    println!(
        "| summary cost (N+1 vs grouped) | {:.1} | {:.1} | {:.1} | {:.1} | {:.2}x |",
        micros(old_summary_stats.median),
        micros(old_summary_stats.p95),
        micros(new_summary_stats.median),
        micros(new_summary_stats.p95),
        old_summary_stats.median.as_secs_f64() / new_summary_stats.median.as_secs_f64(),
    );
    println!(
        "current full financial report: median {:.1} us; p95 {:.1} us",
        micros(full_report_stats.median),
        micros(full_report_stats.p95),
    );

    let (old_plan, new_plan) = {
        let conn = get_db().unwrap();
        (
            explain_plan(&conn, old_date_sql),
            explain_plan(&conn, new_date_sql),
        )
    };
    println!("\n## Date filter query plans");
    println!("old: {}", old_plan.join(" | "));
    println!("new: {}", new_plan.join(" | "));
}
