use crate::database::get_db;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialSummary {
    #[serde(rename = "totalRevenue")]
    pub total_revenue: f64,
    #[serde(rename = "netProfit")]
    pub net_profit: f64,
    #[serde(rename = "partsInUseCost")]
    pub parts_in_use_cost: f64,
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
    pub total_price: f64,
    #[serde(rename = "displayId")]
    pub display_id: String,
    #[serde(rename = "discountPercent")]
    pub discount_percent: f64,
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

        let mut stmt = conn.prepare(
            "SELECT 
                (SELECT COALESCE(SUM(total_price * (1.0 - COALESCE(discount_percent, 0.0) / 100.0)), 0.0) FROM service_orders WHERE status = 'Finalizada' AND deleted_at IS NULL) as total_revenue,
                (SELECT COALESCE(SUM(sop.quantity * sop.unit_cost), 0.0) 
                 FROM service_order_parts sop 
                 JOIN service_orders so ON sop.service_order_id = so.id 
                  WHERE so.status = 'Finalizada' AND so.deleted_at IS NULL) as cost_of_finalized,
                (SELECT COALESCE(SUM(sop.quantity * sop.unit_cost), 0.0) 
                 FROM service_order_parts sop 
                 JOIN service_orders so ON sop.service_order_id = so.id 
                  WHERE so.status NOT IN ('Finalizada', 'Cancelada') AND so.deleted_at IS NULL) as parts_in_use,
                (SELECT COUNT(*) FROM service_orders WHERE status NOT IN ('Finalizada', 'Cancelada') AND deleted_at IS NULL) as active_count"
        )?;

        let (total_revenue, cost_of_finalized, parts_in_use_cost, active_orders_count) = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, i32>(3)?,
                ))
            })?;

        let net_profit = total_revenue - cost_of_finalized;

        // 2. Calculate Trends (comparing with the latest snapshot before today)
        let mut trend_stmt = conn.prepare(
            "SELECT total_revenue, net_profit 
             FROM financial_snapshots 
             WHERE snapshot_date < date('now') 
             ORDER BY snapshot_date DESC LIMIT 1",
        )?;

        let (rev_trend, prof_trend) = match trend_stmt
            .query_row([], |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)))
        {
            Ok((prev_rev, prev_prof)) => {
                let calc_trend = |curr: f64, prev: f64| {
                    if prev <= 0.0 {
                        return ("0%".to_string(), curr > 0.0);
                    }
                    let diff = ((curr - prev) / prev) * 100.0;
                    (format!("{:.0}%", diff.abs()), diff >= 0.0)
                };
                let (rv, rp) = calc_trend(total_revenue, prev_rev);
                let (pv, pp) = calc_trend(net_profit, prev_prof);
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
            "SELECT so.id, c.name, so.equipment, so.status, so.created_at, COALESCE(so.total_price, 0.0), so.display_id, COALESCE(so.discount_percent, 0.0)
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
                    discount_percent: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // 4. Inventory Alerts
        let mut stmt = conn.prepare(
            "SELECT id, name, current_quantity, min_quantity 
             FROM inventory_items 
             WHERE current_quantity < min_quantity AND deleted_at IS NULL
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
                net_profit,
                parts_in_use_cost,
                active_orders_count,
                revenue_trend: rev_trend,
                profit_trend: prof_trend,
            },
            recent_orders,
            inventory_alerts,
            status_counts,
        })
    }
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
        total_price: f64,
        discount_percent: f64,
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
        order.discount_percent = discount_percent;
        ServiceOrderRepository::create_with_conn(conn, &mut order).unwrap();
        order
    }

    #[test]
    fn summary_uses_only_finalized_orders_and_applies_discount() {
        let conn = setup_db();
        let finalized = seed_order(&conn, "Finalizada", 200.0, 10.0);
        seed_order(&conn, "Em Manutenção", 500.0, 0.0);
        let inventory_item = InventoryItem::new(
            "Bateria".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            1,
            10,
            30.0,
            80.0,
        );
        InventoryRepository::create_with_conn(&conn, &inventory_item).unwrap();

        conn.execute(
            "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, quantity, unit_cost, unit_price) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["part-1", finalized.id, inventory_item.id, 2, 30.0, 80.0],
        )
        .unwrap();

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();

        assert_eq!(data.summary.total_revenue, 180.0);
        assert_eq!(data.summary.net_profit, 120.0);
        assert_eq!(data.summary.active_orders_count, 1);
    }

    #[test]
    fn inventory_alerts_only_include_items_below_minimum() {
        let conn = setup_db();
        let low = InventoryItem::new(
            "Conector".to_string(),
            "Peça".to_string(),
            "part".to_string(),
            3,
            1,
            10.0,
            20.0,
        );
        let ok = InventoryItem::new(
            "Capa".to_string(),
            "Acessório".to_string(),
            "part".to_string(),
            3,
            5,
            10.0,
            20.0,
        );
        InventoryRepository::create_with_conn(&conn, &low).unwrap();
        InventoryRepository::create_with_conn(&conn, &ok).unwrap();

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();

        assert_eq!(data.inventory_alerts.len(), 1);
        assert_eq!(data.inventory_alerts[0].id, low.id);
    }

    #[test]
    fn status_counts_group_orders_by_status() {
        let conn = setup_db();
        seed_order(&conn, "Finalizada", 100.0, 0.0);
        seed_order(&conn, "Finalizada", 200.0, 0.0);
        seed_order(&conn, "Cancelada", 0.0, 0.0);

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
        let order = seed_order(&conn, "Finalizada", 250.0, 0.0);
        ServiceOrderRepository::delete_with_conn(&conn, &order.id).unwrap();

        let data = DashboardRepository::get_dashboard_data_with_conn(&conn).unwrap();

        assert_eq!(data.summary.total_revenue, 0.0);
        assert!(data.recent_orders.is_empty());
        assert!(data.status_counts.is_empty());
    }
}
