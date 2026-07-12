use crate::database::get_db;
use rusqlite::Result;
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

        // 1. Calculate Summary
        // total_revenue: SUM(total_price) of 'Finalizada'
        // parts_cost_finalized: SUM(quantity * unit_cost) of parts in 'Finalizada' orders
        // active_orders_count: COUNT of non-'Finalizada' and non-'Cancelada'
        // parts_in_use_cost: SUM(quantity * unit_cost) of parts in active orders
        
        let mut stmt = conn.prepare(
            "SELECT 
                (SELECT COALESCE(SUM(total_price * (1.0 - COALESCE(discount_percent, 0.0) / 100.0)), 0.0) FROM service_orders WHERE status = 'Finalizada') as total_revenue,
                (SELECT COALESCE(SUM(sop.quantity * sop.unit_cost), 0.0) 
                 FROM service_order_parts sop 
                 JOIN service_orders so ON sop.service_order_id = so.id 
                 WHERE so.status = 'Finalizada') as cost_of_finalized,
                (SELECT COALESCE(SUM(sop.quantity * sop.unit_cost), 0.0) 
                 FROM service_order_parts sop 
                 JOIN service_orders so ON sop.service_order_id = so.id 
                 WHERE so.status NOT IN ('Finalizada', 'Cancelada')) as parts_in_use,
                (SELECT COUNT(*) FROM service_orders WHERE status NOT IN ('Finalizada', 'Cancelada')) as active_count"
        )?;

        let (total_revenue, cost_of_finalized, parts_in_use_cost, active_orders_count) = stmt.query_row([], |row| {
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
             ORDER BY snapshot_date DESC LIMIT 1"
        )?;

        let (rev_trend, prof_trend) = match trend_stmt.query_row([], |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?))) {
            Ok((prev_rev, prev_prof)) => {
                let calc_trend = |curr: f64, prev: f64| {
                    if prev <= 0.0 { return ("0%".to_string(), curr > 0.0); }
                    let diff = ((curr - prev) / prev) * 100.0;
                    (format!("{:.0}%", diff.abs()), diff >= 0.0)
                };
                let (rv, rp) = calc_trend(total_revenue, prev_rev);
                let (pv, pp) = calc_trend(net_profit, prev_prof);
                (Trend { value: rv, is_positive: rp }, Trend { value: pv, is_positive: pp })
            },
            Err(_) => (
                Trend { value: "0%".to_string(), is_positive: true },
                Trend { value: "0%".to_string(), is_positive: true }
            )
        };

        // 3. Get Recent Orders
        let mut stmt = conn.prepare(
            "SELECT so.id, c.name, so.equipment, so.status, so.created_at, COALESCE(so.total_price, 0.0), so.display_id, COALESCE(so.discount_percent, 0.0)
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             ORDER BY so.created_at DESC LIMIT 4"
        )?;
        let recent_orders = stmt.query_map([], |row| {
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
        })?.collect::<Result<Vec<_>, _>>()?;

        // 4. Inventory Alerts
        let mut stmt = conn.prepare(
            "SELECT id, name, current_quantity, min_quantity 
             FROM inventory_items 
             WHERE current_quantity < min_quantity AND deleted_at IS NULL
             LIMIT 3"
        )?;
        let inventory_alerts = stmt.query_map([], |row| {
            Ok(InventoryAlert {
                id: row.get(0)?,
                name: row.get(1)?,
                current_stock: row.get(2)?,
                min_stock: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        // 5. Status Counts
        let mut stmt = conn.prepare(
            "SELECT status, COUNT(*) FROM service_orders GROUP BY status"
        )?;
        let status_counts = stmt.query_map([], |row| {
            Ok(StatusCount {
                status: row.get(0)?,
                count: row.get(1)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

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
