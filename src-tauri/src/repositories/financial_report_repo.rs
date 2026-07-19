use crate::database::get_db;
use chrono::{Datelike, Local};
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialBreakdown {
    pub label: String,
    pub revenue: f64,
    pub cost: f64,
    pub profit: f64,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialMonth {
    pub month: String,
    pub revenue: f64,
    pub profit: f64,
    pub order_count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReport {
    pub start_date: String,
    pub end_date: String,
    pub total_revenue: f64,
    pub total_cost: f64,
    pub net_profit: f64,
    pub average_ticket: f64,
    pub finalized_orders: i32,
    pub by_technician: Vec<FinancialBreakdown>,
    pub by_item_type: Vec<FinancialBreakdown>,
    pub by_month: Vec<FinancialMonth>,
}

pub struct FinancialReportRepository;

impl FinancialReportRepository {
    pub fn get_report(start_date: Option<&str>, end_date: Option<&str>) -> Result<FinancialReport> {
        let conn = get_db()?;
        Self::get_report_with_conn(&conn, start_date, end_date)
    }

    pub(crate) fn get_report_with_conn(
        conn: &Connection,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<FinancialReport> {
        let (default_start, default_end) = default_period();
        let start = start_date.unwrap_or(&default_start);
        let end = end_date.unwrap_or(&default_end);
        let finalized_at = "COALESCE(so.closed_at, so.created_at)";
        let period_filter = format!(
            "so.status = 'Finalizada' AND so.deleted_at IS NULL AND date({finalized_at}, 'localtime') BETWEEN date(?1) AND date(?2)"
        );

        let summary_sql = format!(
            "SELECT
                COALESCE(SUM(so.total_price * (1.0 - COALESCE(so.discount_percent, 0.0) / 100.0)), 0.0),
                COALESCE(SUM(COALESCE(costs.total_cost, 0.0)), 0.0),
                COUNT(*)
             FROM service_orders so
             LEFT JOIN (
                 SELECT service_order_id, SUM(quantity * unit_cost) AS total_cost
                 FROM service_order_parts GROUP BY service_order_id
             ) costs ON costs.service_order_id = so.id
             WHERE {period_filter}"
        );
        let (total_revenue, total_cost, finalized_orders): (f64, f64, i32) =
            conn.query_row(&summary_sql, params![start, end], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;

        let by_technician = query_breakdown(
            conn,
            &format!(
                "SELECT COALESCE(u.name, 'Não atribuído'),
                        COALESCE(SUM(so.total_price * (1.0 - COALESCE(so.discount_percent, 0.0) / 100.0)), 0.0),
                        COALESCE(SUM(COALESCE(costs.total_cost, 0.0)), 0.0),
                        COUNT(*)
                 FROM service_orders so
                 LEFT JOIN users u ON so.user_id = u.id
                 LEFT JOIN (
                    SELECT service_order_id, SUM(quantity * unit_cost) AS total_cost
                    FROM service_order_parts GROUP BY service_order_id
                 ) costs ON costs.service_order_id = so.id
                 WHERE {period_filter}
                 GROUP BY so.user_id
                 ORDER BY 2 DESC"
            ),
            start,
            end,
        )?;

        let by_item_type = query_breakdown(
            conn,
            &format!(
                "SELECT CASE ii.type WHEN 'part' THEN 'Peças' ELSE 'Serviços' END,
                        COALESCE(SUM(sop.quantity * sop.unit_price), 0.0),
                        COALESCE(SUM(sop.quantity * sop.unit_cost), 0.0),
                        COALESCE(SUM(sop.quantity), 0)
                 FROM service_order_parts sop
                 JOIN service_orders so ON sop.service_order_id = so.id
                 JOIN inventory_items ii ON sop.inventory_item_id = ii.id
                  WHERE {period_filter}
                 GROUP BY ii.type
                 ORDER BY 2 DESC"
            ),
            start,
            end,
        )?;

        let month_sql = format!(
            "SELECT strftime('%Y-%m', {finalized_at}, 'localtime'),
                    COALESCE(SUM(so.total_price * (1.0 - COALESCE(so.discount_percent, 0.0) / 100.0)), 0.0),
                    COALESCE(SUM(so.total_price * (1.0 - COALESCE(so.discount_percent, 0.0) / 100.0)) - SUM(COALESCE(costs.total_cost, 0.0)), 0.0),
                    COUNT(*)
             FROM service_orders so
             LEFT JOIN (
                 SELECT service_order_id, SUM(quantity * unit_cost) AS total_cost
                 FROM service_order_parts GROUP BY service_order_id
             ) costs ON costs.service_order_id = so.id
             WHERE {period_filter}
             GROUP BY 1 ORDER BY 1"
        );
        let mut month_stmt = conn.prepare(&month_sql)?;
        let by_month = month_stmt
            .query_map(params![start, end], |row| {
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
            net_profit: total_revenue - total_cost,
            average_ticket: if finalized_orders > 0 {
                total_revenue / finalized_orders as f64
            } else {
                0.0
            },
            finalized_orders,
            by_technician,
            by_item_type,
            by_month,
        })
    }
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
) -> Result<Vec<FinancialBreakdown>> {
    let mut stmt = conn.prepare(sql)?;
    let breakdown = stmt
        .query_map(params![start, end], |row| {
            let revenue: f64 = row.get(1)?;
            let cost: f64 = row.get(2)?;
            Ok(FinancialBreakdown {
                label: row.get(0)?,
                revenue,
                cost,
                profit: revenue - cost,
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
            50.0,
            100.0,
        );
        InventoryRepository::create_with_conn(&conn, &item).unwrap();
        let mut order = ServiceOrder::new(customer.id, "iPhone".to_string(), "Tela".to_string());
        order.user_id = Some(user.id);
        order.discount_percent = 10.0;
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &item.id, 2,
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
        assert_eq!(report.total_revenue, 180.0);
        assert_eq!(report.total_cost, 100.0);
        assert_eq!(report.net_profit, 80.0);
        assert_eq!(report.by_technician[0].label, "Técnica");
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
}
