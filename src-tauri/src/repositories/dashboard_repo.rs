use crate::database::get_db;
use crate::money::apply_discount;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialSummary {
    #[serde(rename = "totalRevenue")]
    pub total_revenue: i64,
    #[serde(rename = "estimatedGrossProfit")]
    pub estimated_gross_profit: i64,
    #[serde(rename = "partsInUseCost")]
    pub parts_in_use_cost: i64,
    #[serde(rename = "activeOrdersCount")]
    pub active_orders_count: i32,
    #[serde(rename = "revenueTrend")]
    pub revenue_trend: Trend,
    #[serde(rename = "profitTrend")]
    pub profit_trend: Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub value: String,
    #[serde(rename = "isPositive")]
    pub is_positive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentOS {
    pub id: String,
    #[serde(rename = "customerName")]
    pub customer_name: String,
    pub equipment: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "totalPrice")]
    pub total_price: i64,
    #[serde(rename = "displayId")]
    pub display_id: String,
    #[serde(rename = "discountBasisPoints")]
    pub discount_basis_points: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryAlert {
    pub id: String,
    pub name: String,
    #[serde(rename = "currentStock")]
    pub current_stock: i32,
    #[serde(rename = "minStock")]
    pub min_stock: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryAlertSummary {
    #[serde(rename = "outOfStock")]
    pub out_of_stock: i32,
    #[serde(rename = "lowStock")]
    pub low_stock: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCount {
    pub status: String,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub summary: FinancialSummary,
    pub recent_orders: Vec<RecentOS>,
    pub inventory_alerts: Vec<InventoryAlert>,
    pub inventory_alert_summary: InventoryAlertSummary,
    pub status_counts: Vec<StatusCount>,
}

pub struct DashboardRepository;

impl DashboardRepository {
    pub fn get_dashboard_data() -> Result<DashboardData, rusqlite::Error> {
        let conn = get_db()?;
        Self::get_dashboard_data_with_conn(&conn)
    }

    pub(crate) fn get_dashboard_data_with_conn(
        conn: &Connection,
    ) -> Result<DashboardData, rusqlite::Error> {
        // 1. Calculate Summary
        // total_revenue: SUM(total_price) of 'Finalizada'
        // parts_cost_finalized: SUM(quantity * unit_cost) of parts in 'Finalizada' orders
        // active_orders_count: COUNT of non-'Finalizada' and non-'Cancelada'
        // parts_in_use_cost: SUM(quantity * unit_cost) of parts in active orders

        let total_revenue = sum_discounted_revenue(conn, None, None, None)?;
        let cost_of_finalized = sum_item_cost(conn, true)?;
        let parts_in_use_cost = sum_item_cost(conn, false)?;
        let active_orders_count = conn.query_row(
            "SELECT COUNT(*) FROM service_orders WHERE status NOT IN ('Finalizada', 'Cancelada') AND deleted_at IS NULL",
            [],
            |row| row.get::<_, i32>(0),
        )?;
        let estimated_gross_profit = total_revenue
            .checked_sub(cost_of_finalized)
            .ok_or(rusqlite::Error::InvalidQuery)?;

        // 2. Calculate Trends (comparing with the latest snapshot before today)
        let mut trend_stmt = conn.prepare(
            "SELECT total_revenue_cents, estimated_gross_profit_cents 
             FROM financial_snapshots 
             WHERE snapshot_date < date('now') 
             ORDER BY snapshot_date DESC LIMIT 1",
        )?;

        let (rev_trend, prof_trend) = match trend_stmt
            .query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        {
            Ok((prev_rev, prev_prof)) => {
                let calc_trend = |curr: i64, prev: i64| {
                    if prev <= 0 {
                        return ("0%".to_string(), curr > 0);
                    }
                    let diff = (curr as f64 - prev as f64) / prev as f64 * 100.0;
                    (format!("{:.0}%", diff.abs()), diff >= 0.0)
                };
                let (rv, rp) = calc_trend(total_revenue, prev_rev);
                let (pv, pp) = calc_trend(estimated_gross_profit, prev_prof);
                (
                    Trend {
                        value: rv,
                        is_positive: rp,
                    },
                    Trend {
                        value: pv,
                        is_positive: pp,
                    },
                )
            }
            Err(_) => (
                Trend {
                    value: "0%".to_string(),
                    is_positive: true,
                },
                Trend {
                    value: "0%".to_string(),
                    is_positive: true,
                },
            ),
        };

        // 3. Get Recent Orders
        let mut stmt = conn.prepare(
            "SELECT so.id, c.name, so.equipment, so.status, so.created_at, COALESCE(so.total_price_cents, 0), so.display_id, COALESCE(so.discount_basis_points, 0)
              FROM service_orders so
              LEFT JOIN customers c ON so.customer_id = c.id
              WHERE so.deleted_at IS NULL
              ORDER BY so.created_at DESC LIMIT 4"
        )?;
        let recent_orders = stmt
            .query_map([], |row| {
                Ok(RecentOS {
                    id: row.get(0)?,
                    customer_name: row.get(1)?,
                    equipment: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                    total_price: row.get(5)?,
                    display_id: row.get(6)?,
                    discount_basis_points: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // 4. Inventory Alerts
        let mut alert_summary_stmt = conn.prepare(
            "SELECT
                COALESCE(SUM(CASE WHEN current_quantity = 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN current_quantity > 0 THEN 1 ELSE 0 END), 0)
              FROM inventory_items
              WHERE type = 'part'
                AND current_quantity <= min_quantity
                AND deleted_at IS NULL",
        )?;
        let inventory_alert_summary = alert_summary_stmt.query_row([], |row| {
            Ok(InventoryAlertSummary {
                out_of_stock: row.get(0)?,
                low_stock: row.get(1)?,
            })
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, name, current_quantity, min_quantity
              FROM inventory_items
              WHERE type = 'part'
                AND current_quantity <= min_quantity
                AND deleted_at IS NULL
              ORDER BY
                CASE WHEN current_quantity = 0 THEN 0 ELSE 1 END,
                CAST(current_quantity AS REAL) / CASE WHEN min_quantity > 0 THEN min_quantity ELSE 1 END,
                name COLLATE NOCASE
              LIMIT 3",
        )?;
        let inventory_alerts = stmt
            .query_map([], |row| {
                Ok(InventoryAlert {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    current_stock: row.get(2)?,
                    min_stock: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // 5. Status Counts
        let mut stmt = conn.prepare(
            "SELECT status, COUNT(*) FROM service_orders WHERE deleted_at IS NULL GROUP BY status",
        )?;
        let status_counts = stmt
            .query_map([], |row| {
                Ok(StatusCount {
                    status: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DashboardData {
            summary: FinancialSummary {
                total_revenue,
                estimated_gross_profit,
                parts_in_use_cost,
                active_orders_count,
                revenue_trend: rev_trend,
                profit_trend: prof_trend,
            },
            recent_orders,
            inventory_alerts,
            inventory_alert_summary,
            status_counts,
        })
    }
}

pub(crate) fn sum_discounted_revenue(
    conn: &Connection,
    start: Option<&str>,
    end: Option<&str>,
    technician_id: Option<&str>,
) -> Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT total_price_cents, discount_basis_points
         FROM service_orders
         WHERE status = 'Finalizada' AND deleted_at IS NULL
           AND (?1 IS NULL OR date(COALESCE(closed_at, created_at), 'localtime') >= date(?1))
           AND (?2 IS NULL OR date(COALESCE(closed_at, created_at), 'localtime') <= date(?2))
           AND (?3 IS NULL OR user_id = ?3)",
    )?;
    let rows = stmt.query_map(rusqlite::params![start, end, technician_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut total = 0_i64;
    for row in rows {
        let (amount, basis_points) = row?;
        let discounted =
            apply_discount(amount, basis_points).ok_or(rusqlite::Error::InvalidQuery)?;
        total = total
            .checked_add(discounted)
            .ok_or(rusqlite::Error::InvalidQuery)?;
    }
    Ok(total)
}

fn sum_item_cost(conn: &Connection, finalized: bool) -> Result<i64> {
    let status = if finalized {
        "so.status = 'Finalizada'"
    } else {
        "so.status NOT IN ('Finalizada', 'Cancelada')"
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT sop.quantity, sop.unit_cost_cents FROM service_order_parts sop
         JOIN service_orders so ON sop.service_order_id = so.id
         WHERE {status} AND so.deleted_at IS NULL"
    ))?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    let mut total = 0_i128;
    for row in rows {
        let (quantity, unit_cost) = row?;
        total = total
            .checked_add(i128::from(quantity) * i128::from(unit_cost))
            .ok_or(rusqlite::Error::InvalidQuery)?;
    }
    i64::try_from(total).map_err(|_| rusqlite::Error::InvalidQuery)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::inventory_item::InventoryItem;
    use crate::models::service_order::ServiceOrder;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::inventory_repo::InventoryRepository;
    use crate::repositories::service_order_repo::ServiceOrderRepository;
    use crate::test_helpers::setup_db;
    use rusqlite::params;

    fn seed_order(
        conn: &Connection,
        status: &str,
        total_price: i64,
        discount_basis_points: i64,
    ) -> ServiceOrder {
        let customer = Customer::new(
            format!("Cliente {status}"),
            "41911112222".to_string(),
            format!("{status}@example.com"),
            "Rua X".to_string(),
        );
        CustomerRepository::create_with_conn(conn, &customer).unwrap();

        let mut order = ServiceOrder::new(
            customer.id,
            format!("Equip {status}"),
            "Descrição".to_string(),
        );
        order.status = status.to_string();
        order.total_price = Some(total_price);
        order.discount_basis_points = discount_basis_points;
        ServiceOrderRepository::create_with_conn(conn, &mut order).unwrap();
        order
    }

    #[test]
    fn summary_uses_only_finalized_orders_and_applies_discount() {
        let conn = setup_db();
        let finalized = seed_order(&conn, "Finalizada", 20_000, 1_000);
        seed_order(&conn, "Em Manutenção", 50_000, 0);
        let inventory_item = InventoryItem::new(
            "Bateria".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            1,
            10,
            3_000,
            8_000,
        );
        InventoryRepository::create_with_conn(&conn, &inventory_item).unwrap();

        conn.execute(
            "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, inventory_item_name, item_type, quantity, unit_cost_cents, unit_price_cents) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params!["part-1", finalized.id, inventory_item.id, inventory_item.name, "part", 2, 3_000, 8_000],
        )
        .unwrap();

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();

        assert_eq!(data.summary.total_revenue, 18_000);
        assert_eq!(data.summary.estimated_gross_profit, 12_000);
        assert_eq!(data.summary.active_orders_count, 1);
    }

    #[test]
    fn inventory_alerts_include_empty_stock_and_prioritize_it_before_low_stock() {
        let conn = setup_db();
        let empty_with_zero_minimum = InventoryItem::new(
            "Bateria esgotada".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            0,
            0,
            1_000,
            2_000,
        );
        let empty = InventoryItem::new(
            "Tela esgotada".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            3,
            0,
            1_000,
            2_000,
        );
        let low = InventoryItem::new(
            "Conector".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            3,
            1,
            1_000,
            2_000,
        );
        let ok = InventoryItem::new(
            "Capa".to_string(),
            "Acessório".to_string(),
            "part".to_string(),
            3,
            5,
            1_000,
            2_000,
        );
        let service = InventoryItem::new(
            "Mão de obra".to_string(),
            "Serviço".to_string(),
            "service".to_string(),
            99,
            0,
            1_000,
            2_000,
        );
        InventoryRepository::create_with_conn(&conn, &empty_with_zero_minimum).unwrap();
        InventoryRepository::create_with_conn(&conn, &empty).unwrap();
        InventoryRepository::create_with_conn(&conn, &low).unwrap();
        InventoryRepository::create_with_conn(&conn, &ok).unwrap();
        InventoryRepository::create_with_conn(&conn, &service).unwrap();

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();

        assert_eq!(data.inventory_alerts.len(), 3);
        assert_eq!(data.inventory_alert_summary.out_of_stock, 2);
        assert_eq!(data.inventory_alert_summary.low_stock, 1);
        assert_eq!(
            data.inventory_alerts
                .iter()
                .map(|alert| alert.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                empty_with_zero_minimum.id.as_str(),
                empty.id.as_str(),
                low.id.as_str(),
            ],
        );
    }

    #[test]
    fn status_counts_group_orders_by_status() {
        let conn = setup_db();
        seed_order(&conn, "Finalizada", 10_000, 0);
        seed_order(&conn, "Finalizada", 20_000, 0);
        seed_order(&conn, "Cancelada", 0, 0);

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();
        let finalized = data
            .status_counts
            .iter()
            .find(|status| status.status == "Finalizada")
            .unwrap();
        let canceled = data
            .status_counts
            .iter()
            .find(|status| status.status == "Cancelada")
            .unwrap();

        assert_eq!(finalized.count, 2);
        assert_eq!(canceled.count, 1);
    }

    #[test]
    fn deleted_orders_do_not_affect_dashboard_metrics() {
        let conn = setup_db();
        let order = seed_order(&conn, "Finalizada", 25_000, 0);
        ServiceOrderRepository::delete_with_conn(&conn, &order.id).unwrap();

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();

        assert_eq!(data.summary.total_revenue, 0);
        assert!(data.recent_orders.is_empty());
        assert!(data.status_counts.is_empty());
    }
}
