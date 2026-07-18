use crate::error::AppError;
use crate::repositories::dashboard_repo::{DashboardData, DashboardRepository};
use tauri::command;

#[command]
pub fn get_dashboard_data() -> Result<DashboardData, AppError> {
    Ok(DashboardRepository::get_dashboard_data()?)
}
