use crate::database::get_db;
use crate::money::{apply_discount, checked_div_round_nearest};
use chrono::{Datelike, Local};
use rusqlite::{params, Connection, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialBreakdown {
    pub label: String,
    pub revenue: i64,
    pub cost: i64,
    pub profit: i64,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialMonth {
    pub month: String,
    pub revenue: i64,
    pub profit: i64,
    pub order_count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReport {
    pub start_date: String,
    pub end_date: String,
    pub total_revenue: i64,
    pub total_cost: i64,
    pub estimated_gross_profit: i64,
    pub average_ticket: i64,
    pub finalized_orders: i32,
    pub new_customers: i32,
    pub new_orders: i32,
    pub completion_rate: f64,
    pub cancelled_orders: i32,
    pub cancellation_rate: f64,
    pub average_turnaround_hours: f64,
    pub returning_customers: i32,
    pub total_discounts: i64,
    pub ranking_metric: String,
    pub ranking_limit: i32,
    pub by_technician: Vec<FinancialBreakdown>,
    pub by_item_type: Vec<FinancialBreakdown>,
    pub top_items: Vec<FinancialBreakdown>,
    pub by_month: Vec<FinancialMonth>,
}

pub struct FinancialReportRepository;

impl FinancialReportRepository {
    pub fn get_report_filtered(
        start_date: Option<&str>,
        end_date: Option<&str>,
        technician_id: Option<&str>,
        ranking_metric: Option<&str>,
        ranking_limit: Option<i32>,
    ) -> Result<FinancialReport> {
        let conn = get_db()?;
        Self::get_report_with_conn_filtered(
            &conn,
            start_date,
            end_date,
            technician_id,
            ranking_metric,
            ranking_limit,
        )
    }

    #[cfg(test)]
    pub(crate) fn get_report_with_conn(
        conn: &Connection,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<FinancialReport> {
        Self::get_report_with_conn_filtered(conn, start_date, end_date, None, None, None)
    }

    pub(crate) fn get_report_with_conn_filtered(
        conn: &Connection,
        start_date: Option<&str>,
        end_date: Option<&str>,
        technician_id: Option<&str>,
        ranking_metric: Option<&str>,
        ranking_limit: Option<i32>,
    ) -> Result<FinancialReport> {
        let (default_start, default_end) = default_period();
        let start = start_date.unwrap_or(&default_start);
        let end = end_date.unwrap_or(&default_end);
        let ranking_metric = match ranking_metric {
            Some("quantity") => "quantity",
            _ => "revenue",
        };
        let ranking_limit = ranking_limit.unwrap_or(5).clamp(5, 20);
        let finalized_at = "COALESCE(so.closed_at, so.created_at)";
        let period_filter = format!(
            "so.status = 'Finalizada' AND so.deleted_at IS NULL AND date({finalized_at}, 'localtime') BETWEEN date(?1) AND date(?2) AND (?3 IS NULL OR so.user_id = ?3)"
        );

        let (total_revenue, total_cost, finalized_orders, total_discounts) =
            query_summary(conn, start, end, technician_id)?;

        let created_in_period = "so.deleted_at IS NULL AND date(so.created_at, 'localtime') BETWEEN date(?1) AND date(?2) AND (?3 IS NULL OR so.user_id = ?3)";
        let operational_sql = format!(
            "SELECT
                (SELECT COUNT(DISTINCT c.id) FROM customers c JOIN service_orders so ON so.customer_id = c.id WHERE c.deleted_at IS NULL AND date(c.created_at, 'localtime') BETWEEN date(?1) AND date(?2) AND {created_in_period}),
                (SELECT COUNT(*) FROM service_orders so WHERE {created_in_period}),
                (SELECT COUNT(*) FROM service_orders so WHERE {created_in_period} AND so.status = 'Finalizada'),
                (SELECT COUNT(*) FROM service_orders so WHERE {created_in_period} AND so.status = 'Cancelada'),
                (SELECT COALESCE(AVG((julianday(COALESCE(so.closed_at, so.created_at)) - julianday(so.created_at)) * 24.0), 0.0) FROM service_orders so WHERE {period_filter}),
                (SELECT COUNT(DISTINCT so.customer_id) FROM service_orders so WHERE {created_in_period} AND EXISTS (
                    SELECT 1 FROM service_orders previous WHERE previous.customer_id = so.customer_id AND previous.deleted_at IS NULL AND date(previous.created_at, 'localtime') < date(?1)
                ))"
        );
        let (
            new_customers,
            new_orders,
            completed_created_orders,
            cancelled_orders,
            average_turnaround_hours,
            returning_customers,
        ): (i32, i32, i32, i32, f64, i32) = conn.query_row(
            &operational_sql,
            params![start, end, technician_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?;
        let completion_rate = if new_orders > 0 {
            completed_created_orders as f64 / new_orders as f64 * 100.0
        } else {
            0.0
        };
        let cancellation_rate = if new_orders > 0 {
            cancelled_orders as f64 / new_orders as f64 * 100.0
        } else {
            0.0
        };

        let by_technician = query_breakdown(
            conn,
            &format!(
                "SELECT COALESCE(u.name, 'Não atribuído'),
                        COALESCE(SUM((so.total_price_cents / 10000) * (10000 - so.discount_basis_points) + ((so.total_price_cents % 10000) * (10000 - so.discount_basis_points) + 5000) / 10000), 0),
                        COALESCE(SUM(COALESCE(costs.total_cost, 0)), 0),
                        COUNT(*)
                 FROM service_orders so
                 LEFT JOIN users u ON so.user_id = u.id
                 LEFT JOIN (
                    SELECT service_order_id, SUM(quantity * unit_cost_cents) AS total_cost
                    FROM service_order_parts GROUP BY service_order_id
                 ) costs ON costs.service_order_id = so.id
                 WHERE {period_filter}
                 GROUP BY so.user_id
                 ORDER BY 2 DESC"
            ),
            start,
            end,
            technician_id,
        )?;

        let (by_item_type, top_items) = query_allocated_line_breakdowns(
            conn,
            start,
            end,
            technician_id,
            ranking_metric,
            ranking_limit,
        )?;

        let month_sql = format!(
            "SELECT strftime('%Y-%m', {finalized_at}, 'localtime'),
                    COALESCE(SUM((so.total_price_cents / 10000) * (10000 - so.discount_basis_points) + ((so.total_price_cents % 10000) * (10000 - so.discount_basis_points) + 5000) / 10000), 0),
                    COALESCE(SUM((so.total_price_cents / 10000) * (10000 - so.discount_basis_points) + ((so.total_price_cents % 10000) * (10000 - so.discount_basis_points) + 5000) / 10000) - SUM(COALESCE(costs.total_cost, 0)), 0),
                    COUNT(*)
             FROM service_orders so
             LEFT JOIN (
                  SELECT service_order_id, SUM(quantity * unit_cost_cents) AS total_cost
                 FROM service_order_parts GROUP BY service_order_id
             ) costs ON costs.service_order_id = so.id
             WHERE {period_filter}
             GROUP BY 1 ORDER BY 1"
        );
        let mut month_stmt = conn.prepare(&month_sql)?;
        let by_month = month_stmt
            .query_map(params![start, end, technician_id], |row| {
                Ok(FinancialMonth {
                    month: row.get(0)?,
                    revenue: row.get(1)?,
                    profit: row.get(2)?,
                    order_count: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FinancialReport {
            start_date: start.to_string(),
            end_date: end.to_string(),
            total_revenue,
            total_cost,
            estimated_gross_profit: total_revenue
                .checked_sub(total_cost)
                .ok_or(rusqlite::Error::InvalidQuery)?,
            average_ticket: if finalized_orders > 0 {
                checked_div_round_nearest(total_revenue, i64::from(finalized_orders))
                    .ok_or(rusqlite::Error::InvalidQuery)?
            } else {
                0
            },
            finalized_orders,
            new_customers,
            new_orders,
            completion_rate,
            cancelled_orders,
            cancellation_rate,
            average_turnaround_hours,
            returning_customers,
            total_discounts,
            ranking_metric: ranking_metric.to_string(),
            ranking_limit,
            by_technician,
            by_item_type,
            top_items,
            by_month,
        })
    }
}

#[derive(Debug)]
struct ReportLine {
    id: String,
    order_id: String,
    name: String,
    item_type: String,
    quantity: i64,
    gross: i64,
    cost: i64,
    order_gross: i64,
    discount_basis_points: i64,
}

#[derive(Default)]
struct LineAggregate {
    revenue: i64,
    cost: i64,
    quantity: i64,
    order_ids: HashSet<String>,
}

fn query_allocated_line_breakdowns(
    conn: &Connection,
    start: &str,
    end: &str,
    technician_id: Option<&str>,
    ranking_metric: &str,
    ranking_limit: i32,
) -> Result<(Vec<FinancialBreakdown>, Vec<FinancialBreakdown>)> {
    let mut stmt = conn.prepare(
        "SELECT sop.id, so.id, sop.inventory_item_name, sop.item_type, sop.quantity,
                sop.quantity * sop.unit_price_cents, sop.quantity * sop.unit_cost_cents,
                so.total_price_cents, so.discount_basis_points
         FROM service_order_parts sop
         JOIN service_orders so ON sop.service_order_id = so.id
         WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL
           AND date(COALESCE(so.closed_at, so.created_at), 'localtime') BETWEEN date(?1) AND date(?2)
           AND (?3 IS NULL OR so.user_id = ?3)
         ORDER BY so.id, sop.id",
    )?;
    let lines = stmt
        .query_map(params![start, end, technician_id], |row| {
            Ok(ReportLine {
                id: row.get(0)?,
                order_id: row.get(1)?,
                name: row.get(2)?,
                item_type: row.get(3)?,
                quantity: row.get(4)?,
                gross: row.get(5)?,
                cost: row.get(6)?,
                order_gross: row.get(7)?,
                discount_basis_points: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut by_type: BTreeMap<String, LineAggregate> = BTreeMap::new();
    let mut by_name: BTreeMap<String, LineAggregate> = BTreeMap::new();
    let mut start_index = 0;
    while start_index < lines.len() {
        let order_id = &lines[start_index].order_id;
        let mut end_index = start_index + 1;
        while end_index < lines.len() && lines[end_index].order_id == *order_id {
            end_index += 1;
        }
        let order_lines = &lines[start_index..end_index];
        let target = apply_discount(
            order_lines[0].order_gross,
            order_lines[0].discount_basis_points,
        )
        .ok_or(rusqlite::Error::InvalidQuery)?;
        let gross_sum = order_lines.iter().try_fold(0_i128, |sum, line| {
            if line.gross < 0 || line.cost < 0 || line.quantity < 0 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            sum.checked_add(i128::from(line.gross))
                .ok_or(rusqlite::Error::InvalidQuery)
        })?;
        if gross_sum == 0 && target != 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let mut allocations = Vec::with_capacity(order_lines.len());
        let mut allocated = 0_i64;
        for line in order_lines {
            let numerator = i128::from(line.gross)
                .checked_mul(i128::from(target))
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let (base, remainder) = if gross_sum == 0 {
                (0, 0)
            } else {
                (numerator / gross_sum, numerator % gross_sum)
            };
            let base = i64::try_from(base).map_err(|_| rusqlite::Error::InvalidQuery)?;
            allocated = allocated
                .checked_add(base)
                .ok_or(rusqlite::Error::InvalidQuery)?;
            allocations.push((line, base, remainder));
        }
        allocations.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| left.0.id.cmp(&right.0.id))
        });
        let remaining = target
            .checked_sub(allocated)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        let remaining = usize::try_from(remaining).map_err(|_| rusqlite::Error::InvalidQuery)?;
        if remaining > allocations.len() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        for (index, (line, base, _)) in allocations.into_iter().enumerate() {
            let revenue = base
                .checked_add(i64::from(index < remaining))
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let type_label = if line.item_type == "part" {
                "Peças"
            } else {
                "Serviços"
            };
            add_line_aggregate(
                by_type.entry(type_label.to_string()).or_default(),
                line,
                revenue,
            )?;
            add_line_aggregate(by_name.entry(line.name.clone()).or_default(), line, revenue)?;
        }
        start_index = end_index;
    }

    let mut by_item_type = by_type
        .into_iter()
        .map(|(label, aggregate)| breakdown_from_aggregate(label, aggregate, true))
        .collect::<Result<Vec<_>>>()?;
    by_item_type.sort_by(|left, right| {
        right
            .revenue
            .cmp(&left.revenue)
            .then_with(|| left.label.cmp(&right.label))
    });

    let mut top_items = by_name
        .into_iter()
        .map(|(label, aggregate)| breakdown_from_aggregate(label, aggregate, false))
        .collect::<Result<Vec<_>>>()?;
    top_items.sort_by(|left, right| {
        let metric_order = if ranking_metric == "quantity" {
            right.count.cmp(&left.count)
        } else {
            right.revenue.cmp(&left.revenue)
        };
        metric_order.then_with(|| left.label.cmp(&right.label))
    });
    top_items.truncate(usize::try_from(ranking_limit).map_err(|_| rusqlite::Error::InvalidQuery)?);
    Ok((by_item_type, top_items))
}

fn add_line_aggregate(
    aggregate: &mut LineAggregate,
    line: &ReportLine,
    revenue: i64,
) -> Result<()> {
    aggregate.revenue = aggregate
        .revenue
        .checked_add(revenue)
        .ok_or(rusqlite::Error::InvalidQuery)?;
    aggregate.cost = aggregate
        .cost
        .checked_add(line.cost)
        .ok_or(rusqlite::Error::InvalidQuery)?;
    aggregate.quantity = aggregate
        .quantity
        .checked_add(line.quantity)
        .ok_or(rusqlite::Error::InvalidQuery)?;
    aggregate.order_ids.insert(line.order_id.clone());
    Ok(())
}

fn breakdown_from_aggregate(
    label: String,
    aggregate: LineAggregate,
    count_orders: bool,
) -> Result<FinancialBreakdown> {
    let count = if count_orders {
        i32::try_from(aggregate.order_ids.len()).map_err(|_| rusqlite::Error::InvalidQuery)?
    } else {
        i32::try_from(aggregate.quantity).map_err(|_| rusqlite::Error::InvalidQuery)?
    };
    Ok(FinancialBreakdown {
        label,
        revenue: aggregate.revenue,
        cost: aggregate.cost,
        profit: aggregate
            .revenue
            .checked_sub(aggregate.cost)
            .ok_or(rusqlite::Error::InvalidQuery)?,
        count,
    })
}

fn query_summary(
    conn: &Connection,
    start: &str,
    end: &str,
    technician_id: Option<&str>,
) -> Result<(i64, i64, i32, i64)> {
    let mut orders = conn.prepare(
        "SELECT so.id, so.total_price_cents, so.discount_basis_points
         FROM service_orders so
         WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL
           AND date(COALESCE(so.closed_at, so.created_at), 'localtime') BETWEEN date(?1) AND date(?2)
           AND (?3 IS NULL OR so.user_id = ?3)",
    )?;
    let rows = orders.query_map(params![start, end, technician_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut revenue = 0_i64;
    let mut cost = 0_i128;
    let mut discounts = 0_i64;
    let mut count = 0_i32;
    for row in rows {
        let (order_id, gross, basis_points) = row?;
        let discounted =
            apply_discount(gross, basis_points).ok_or(rusqlite::Error::InvalidQuery)?;
        revenue = revenue
            .checked_add(discounted)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        discounts = discounts
            .checked_add(
                gross
                    .checked_sub(discounted)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
            )
            .ok_or(rusqlite::Error::InvalidQuery)?;
        count = count.checked_add(1).ok_or(rusqlite::Error::InvalidQuery)?;

        let mut parts = conn.prepare(
            "SELECT quantity, unit_cost_cents FROM service_order_parts WHERE service_order_id = ?1",
        )?;
        let part_rows = parts.query_map([order_id], |part| {
            Ok((part.get::<_, i64>(0)?, part.get::<_, i64>(1)?))
        })?;
        for part in part_rows {
            let (quantity, unit_cost) = part?;
            cost = cost
                .checked_add(i128::from(quantity) * i128::from(unit_cost))
                .ok_or(rusqlite::Error::InvalidQuery)?;
        }
    }
    Ok((
        revenue,
        i64::try_from(cost).map_err(|_| rusqlite::Error::InvalidQuery)?,
        count,
        discounts,
    ))
}

fn default_period() -> (String, String) {
    let today = Local::now().date_naive();
    (
        format!("{:04}-{:02}-01", today.year(), today.month()),
        today.format("%Y-%m-%d").to_string(),
    )
}

fn query_breakdown(
    conn: &Connection,
    sql: &str,
    start: &str,
    end: &str,
    technician_id: Option<&str>,
) -> Result<Vec<FinancialBreakdown>> {
    let mut stmt = conn.prepare(sql)?;
    let breakdown = stmt
        .query_map(params![start, end, technician_id], |row| {
            let revenue: i64 = row.get(1)?;
            let cost: i64 = row.get(2)?;
            Ok(FinancialBreakdown {
                label: row.get(0)?,
                revenue,
                cost,
                profit: revenue
                    .checked_sub(cost)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                count: row.get(3)?,
            })
        })?
        .collect();
    breakdown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::inventory_item::InventoryItem;
    use crate::models::service_order::ServiceOrder;
    use crate::models::user::User;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::inventory_repo::InventoryRepository;
    use crate::repositories::service_order_repo::ServiceOrderRepository;
    use crate::repositories::user_repo::UserRepository;
    use crate::test_helpers::setup_db;

    #[test]
    fn reports_discounted_revenue_cost_and_technician_breakdown() {
        let mut conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let user = User::new("Técnica".to_string(), "tecnica@example.com".to_string());
        UserRepository::create_with_conn(&conn, &user).unwrap();
        let item = InventoryItem::new(
            "Tela".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            1,
            3,
            5_000,
            10_000,
        );
        InventoryRepository::create_with_conn(&conn, &item).unwrap();
        let mut order = ServiceOrder::new(customer.id, "iPhone".to_string(), "Tela".to_string());
        order.user_id = Some(user.id);
        order.discount_basis_points = 1_000;
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &item.id, 2,
        )
        .unwrap();
        conn.execute(
            "UPDATE inventory_items SET name = 'Tela renomeada', type = 'service' WHERE id = ?1",
            params![item.id],
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(&conn, &order.id, "Finalizada", false)
            .unwrap();

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some("2000-01-01"),
            Some("2099-12-31"),
        )
        .unwrap();

        assert_eq!(report.finalized_orders, 1);
        assert_eq!(report.total_revenue, 18_000);
        assert_eq!(report.total_cost, 10_000);
        assert_eq!(report.estimated_gross_profit, 8_000);
        assert_eq!(report.total_discounts, 2_000);
        assert_eq!(report.new_customers, 1);
        assert_eq!(report.new_orders, 1);
        assert_eq!(report.completion_rate, 100.0);
        assert_eq!(report.cancelled_orders, 0);
        assert_eq!(report.cancellation_rate, 0.0);
        assert_eq!(report.returning_customers, 0);
        assert_eq!(report.by_technician[0].label, "Técnica");
        assert_eq!(report.top_items[0].label, "Tela");
        assert_eq!(report.top_items[0].revenue, 18_000);
        assert_eq!(report.top_items[0].count, 2);
        assert_eq!(report.by_item_type[0].label, "Peças");
        assert_eq!(report.by_item_type[0].revenue, 18_000);
        assert_eq!(report.by_item_type[0].count, 1);
    }

    #[test]
    fn includes_orders_finalized_on_the_local_end_date() {
        let conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let mut order = ServiceOrder::new(customer.id, "iPhone".to_string(), "Tela".to_string());
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(&conn, &order.id, "Finalizada", false)
            .unwrap();

        let finalized_at = "2030-01-01T01:00:00+00:00";
        conn.execute(
            "UPDATE service_orders SET closed_at = ?1 WHERE id = ?2",
            params![finalized_at, order.id],
        )
        .unwrap();
        let local_finalized_date: String = conn
            .query_row(
                "SELECT date(?1, 'localtime')",
                params![finalized_at],
                |row| row.get(0),
            )
            .unwrap();

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some(&local_finalized_date),
            Some(&local_finalized_date),
        )
        .unwrap();

        assert_eq!(report.finalized_orders, 1);
    }

    #[test]
    fn category_revenue_applies_discount_and_counts_distinct_orders() {
        let mut conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let first = InventoryItem::new(
            "Tela".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            1,
            2,
            2_000,
            10_000,
        );
        let second = InventoryItem::new(
            "Conector".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            1,
            2,
            1_000,
            5_000,
        );
        InventoryRepository::create_with_conn(&conn, &first).unwrap();
        InventoryRepository::create_with_conn(&conn, &second).unwrap();
        let mut order = ServiceOrder::new(customer.id, "iPhone".to_string(), "Reparo".to_string());
        order.discount_basis_points = 2_000;
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &first.id, 1,
        )
        .unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &second.id, 1,
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(&conn, &order.id, "Finalizada", false)
            .unwrap();

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some("2000-01-01"),
            Some("2099-12-31"),
        )
        .unwrap();

        assert_eq!(report.by_item_type[0].label, "Peças");
        assert_eq!(report.by_item_type[0].revenue, 12_000);
        assert_eq!(report.by_item_type[0].cost, 3_000);
        assert_eq!(report.by_item_type[0].count, 1);
    }

    #[test]
    fn allocated_line_revenue_reconciles_an_odd_cent_order() {
        let mut conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let first = InventoryItem::new(
            "Item A".to_string(),
            String::new(),
            "part".to_string(),
            0,
            1,
            0,
            1,
        );
        let second = InventoryItem::new(
            "Item B".to_string(),
            String::new(),
            "service".to_string(),
            0,
            0,
            0,
            1,
        );
        InventoryRepository::create_with_conn(&conn, &first).unwrap();
        InventoryRepository::create_with_conn(&conn, &second).unwrap();
        let mut order = ServiceOrder::new(customer.id, "Celular".to_string(), "Reparo".to_string());
        order.discount_basis_points = 5_000;
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &first.id, 1,
        )
        .unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &second.id, 1,
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(&conn, &order.id, "Finalizada", false)
            .unwrap();

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some("2000-01-01"),
            Some("2099-12-31"),
        )
        .unwrap();

        assert_eq!(report.total_revenue, 1);
        assert_eq!(
            report
                .by_item_type
                .iter()
                .map(|item| item.revenue)
                .sum::<i64>(),
            1
        );
        assert_eq!(
            report
                .top_items
                .iter()
                .map(|item| item.revenue)
                .sum::<i64>(),
            1
        );
    }

    #[test]
    fn average_ticket_rounds_to_nearest_cent() {
        let conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        for total in [50, 51] {
            let mut order = ServiceOrder::new(
                customer.id.clone(),
                "Celular".to_string(),
                "Reparo".to_string(),
            );
            order.status = "Finalizada".to_string();
            order.total_price = Some(total);
            ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        }

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some("2000-01-01"),
            Some("2099-12-31"),
        )
        .unwrap();

        assert_eq!(report.total_revenue, 101);
        assert_eq!(report.average_ticket, 51);
    }

    #[test]
    fn reports_operational_customer_and_cancellation_metrics() {
        let conn = setup_db();
        let returning_customer = Customer::new(
            "Cliente recorrente".to_string(),
            "41".to_string(),
            "recorrente@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &returning_customer).unwrap();
        conn.execute(
            "UPDATE customers SET created_at = '2024-01-01T12:00:00+00:00' WHERE id = ?1",
            params![returning_customer.id],
        )
        .unwrap();
        let new_customer = Customer::new(
            "Cliente novo".to_string(),
            "42".to_string(),
            "novo@example.com".to_string(),
            "Rua B".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &new_customer).unwrap();
        conn.execute(
            "UPDATE customers SET created_at = '2024-02-10T12:00:00+00:00' WHERE id = ?1",
            params![new_customer.id],
        )
        .unwrap();

        let mut previous_order = ServiceOrder::new(
            returning_customer.id.clone(),
            "Notebook".to_string(),
            "Avaliação inicial".to_string(),
        );
        ServiceOrderRepository::create_with_conn(&conn, &mut previous_order).unwrap();
        conn.execute(
            "UPDATE service_orders SET created_at = '2024-01-15T12:00:00+00:00' WHERE id = ?1",
            params![previous_order.id],
        )
        .unwrap();

        let mut completed_order = ServiceOrder::new(
            returning_customer.id.clone(),
            "Notebook".to_string(),
            "Reparo concluído".to_string(),
        );
        ServiceOrderRepository::create_with_conn(&conn, &mut completed_order).unwrap();
        conn.execute(
            "UPDATE service_orders SET status = 'Finalizada', created_at = '2024-02-10T12:00:00+00:00', closed_at = '2024-02-11T12:00:00+00:00' WHERE id = ?1",
            params![completed_order.id],
        )
        .unwrap();

        let mut cancelled_order = ServiceOrder::new(
            new_customer.id.clone(),
            "Tablet".to_string(),
            "Orçamento cancelado".to_string(),
        );
        ServiceOrderRepository::create_with_conn(&conn, &mut cancelled_order).unwrap();
        conn.execute(
            "UPDATE service_orders SET status = 'Cancelada', created_at = '2024-02-12T12:00:00+00:00', closed_at = '2024-02-12T18:00:00+00:00' WHERE id = ?1",
            params![cancelled_order.id],
        )
        .unwrap();

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some("2024-02-01"),
            Some("2024-02-29"),
        )
        .unwrap();

        assert_eq!(report.new_customers, 1);
        assert_eq!(report.new_orders, 2);
        assert_eq!(report.completion_rate, 50.0);
        assert_eq!(report.cancelled_orders, 1);
        assert_eq!(report.cancellation_rate, 50.0);
        assert_eq!(report.returning_customers, 1);
        assert_eq!(report.average_turnaround_hours, 24.0);
    }

    #[test]
    fn new_orders_are_counted_by_created_at_not_finalization_date() {
        let conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let mut created_in_period = ServiceOrder::new(
            customer.id.clone(),
            "Notebook".to_string(),
            "Reparo".to_string(),
        );
        ServiceOrderRepository::create_with_conn(&conn, &mut created_in_period).unwrap();
        let mut created_before_period =
            ServiceOrder::new(customer.id, "Tablet".to_string(), "Reparo".to_string());
        ServiceOrderRepository::create_with_conn(&conn, &mut created_before_period).unwrap();
        conn.execute(
            "UPDATE service_orders SET created_at = '2024-02-10T12:00:00+00:00', status = 'Finalizada', closed_at = '2025-03-01T12:00:00+00:00' WHERE id = ?1",
            params![created_in_period.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE service_orders SET created_at = '2024-01-31T12:00:00+00:00', status = 'Finalizada', closed_at = '2024-02-10T12:00:00+00:00' WHERE id = ?1",
            params![created_before_period.id],
        )
        .unwrap();

        let report = FinancialReportRepository::get_report_with_conn(
            &conn,
            Some("2024-02-01"),
            Some("2024-02-29"),
        )
        .unwrap();

        assert_eq!(report.new_orders, 1);
    }

    #[test]
    fn filters_by_technician_and_orders_ranking_by_metric() {
        let conn = setup_db();
        let technician = User::new("Técnica A".to_string(), "tecnica-a@example.com".to_string());
        let other_technician =
            User::new("Técnico B".to_string(), "tecnico-b@example.com".to_string());
        UserRepository::create_with_conn(&conn, &technician).unwrap();
        UserRepository::create_with_conn(&conn, &other_technician).unwrap();
        let customer_a = Customer::new(
            "Cliente A".to_string(),
            "41".to_string(),
            "cliente-a@example.com".to_string(),
            "Rua A".to_string(),
        );
        let customer_b = Customer::new(
            "Cliente B".to_string(),
            "42".to_string(),
            "cliente-b@example.com".to_string(),
            "Rua B".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer_a).unwrap();
        CustomerRepository::create_with_conn(&conn, &customer_b).unwrap();
        let quantity_item = InventoryItem::new(
            "Mais unidades".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            0,
            10,
            100,
            1_000,
        );
        let revenue_item = InventoryItem::new(
            "Maior faturamento".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            0,
            10,
            100,
            10_000,
        );
        InventoryRepository::create_with_conn(&conn, &quantity_item).unwrap();
        InventoryRepository::create_with_conn(&conn, &revenue_item).unwrap();

        let mut selected_order =
            ServiceOrder::new(customer_a.id, "iPhone".to_string(), "Reparo".to_string());
        selected_order.user_id = Some(technician.id.clone());
        ServiceOrderRepository::create_with_conn(&conn, &mut selected_order).unwrap();
        let mut other_order =
            ServiceOrder::new(customer_b.id, "Samsung".to_string(), "Reparo".to_string());
        other_order.user_id = Some(other_technician.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut other_order).unwrap();
        conn.execute(
            "UPDATE service_orders SET status = 'Finalizada', closed_at = created_at WHERE id IN (?1, ?2)",
            params![selected_order.id, other_order.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, inventory_item_name, item_type, quantity, unit_cost_cents, unit_price_cents)
             VALUES ('quantity-item', ?1, ?2, 'Mais unidades', 'part', 3, 100, 1000),
                    ('revenue-item', ?1, ?3, 'Maior faturamento', 'part', 1, 100, 10000)",
            params![selected_order.id, quantity_item.id, revenue_item.id],
        )
        .unwrap();

        let by_quantity = FinancialReportRepository::get_report_with_conn_filtered(
            &conn,
            Some("2000-01-01"),
            Some("2099-12-31"),
            Some(&technician.id),
            Some("quantity"),
            Some(2),
        )
        .unwrap();
        let by_revenue = FinancialReportRepository::get_report_with_conn_filtered(
            &conn,
            Some("2000-01-01"),
            Some("2099-12-31"),
            Some(&technician.id),
            Some("revenue"),
            Some(20),
        )
        .unwrap();

        assert_eq!(by_quantity.finalized_orders, 1);
        assert_eq!(by_quantity.new_orders, 1);
        assert_eq!(by_quantity.new_customers, 1);
        assert_eq!(by_quantity.ranking_metric, "quantity");
        assert_eq!(by_quantity.ranking_limit, 5);
        assert_eq!(by_quantity.top_items[0].label, "Mais unidades");
        assert_eq!(by_revenue.ranking_limit, 20);
        assert_eq!(by_revenue.top_items[0].label, "Maior faturamento");
    }
}
