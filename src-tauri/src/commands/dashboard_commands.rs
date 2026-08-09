use crate::error::AppError;
use crate::money::is_js_safe_integer;
use crate::repositories::dashboard_repo::{DashboardData, DashboardRepository};
use tauri::command;

#[command]
pub fn get_dashboard_data() -> Result<DashboardData, AppError> {
    let data = DashboardRepository::get_dashboard_data()?;
    validate_dashboard_money_for_ipc(&data)?;
    Ok(data)
}

fn validate_dashboard_money_for_ipc(data: &DashboardData) -> Result<(), AppError> {
    let values = [
        data.summary.total_revenue,
        data.summary.estimated_gross_profit,
        data.summary.parts_in_use_cost,
    ]
    .into_iter()
    .chain(data.recent_orders.iter().map(|order| order.total_price));
    if values.into_iter().all(is_js_safe_integer) {
        return Ok(());
    }
    Err(AppError::new(
        "Dashboard money exceeds the JavaScript safe integer range.",
        "Um valor monetário do painel excede o limite seguro do JavaScript.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::dashboard_repo::{FinancialSummary, InventoryAlertSummary, Trend};

    #[test]
    fn rejects_unsafe_dashboard_money_before_ipc() {
        let data = DashboardData {
            summary: FinancialSummary {
                total_revenue: crate::money::JS_MAX_SAFE_INTEGER + 1,
                estimated_gross_profit: 0,
                parts_in_use_cost: 0,
                active_orders_count: 0,
                revenue_trend: Trend {
                    value: "0%".to_string(),
                    is_positive: true,
                },
                profit_trend: Trend {
                    value: "0%".to_string(),
                    is_positive: true,
                },
            },
            recent_orders: vec![],
            inventory_alerts: vec![],
            inventory_alert_summary: InventoryAlertSummary {
                out_of_stock: 0,
                low_stock: 0,
            },
            status_counts: vec![],
        };

        assert!(validate_dashboard_money_for_ipc(&data).is_err());
    }
}
