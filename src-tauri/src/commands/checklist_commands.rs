use crate::error::AppError;
use crate::models::checklist::{ChecklistItem, ChecklistTemplate};
use crate::repositories::checklist_repo::ChecklistRepository;
use tauri::command;

#[command]
pub fn create_checklist_template(title: String, items: Vec<String>) -> Result<String, AppError> {
    Ok(ChecklistRepository::create_template(&title, items)?)
}

#[command]
pub fn get_checklist_templates() -> Result<Vec<ChecklistTemplate>, AppError> {
    Ok(ChecklistRepository::get_templates()?)
}

#[command]
pub fn get_checklist_template_items(id: String) -> Result<Vec<String>, AppError> {
    Ok(ChecklistRepository::get_template_items(&id)?)
}

#[command]
pub fn delete_checklist_template(id: String) -> Result<(), AppError> {
    Ok(ChecklistRepository::delete_template(&id)?)
}

#[command]
pub fn update_checklist_template(
    id: String,
    title: String,
    items: Vec<String>,
) -> Result<(), AppError> {
    Ok(ChecklistRepository::update_template(&id, &title, items)?)
}

#[command]
pub fn save_service_order_checklist(
    os_id: String,
    items: Vec<ChecklistItem>,
) -> Result<(), AppError> {
    Ok(ChecklistRepository::save_os_checklist(&os_id, items)?)
}

#[command]
pub fn get_service_order_checklist(os_id: String) -> Result<Vec<ChecklistItem>, AppError> {
    Ok(ChecklistRepository::get_os_checklist(&os_id)?)
}
