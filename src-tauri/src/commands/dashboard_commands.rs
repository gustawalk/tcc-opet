use crate::repositories::dashboard_repo::{DashboardData, DashboardRepository};
use tauri::command;

#[command]
pub fn get_dashboard_data() -> Result<DashboardData, String> {
    DashboardRepository::get_dashboard_data().map_err(|e| e.to_string())
}
