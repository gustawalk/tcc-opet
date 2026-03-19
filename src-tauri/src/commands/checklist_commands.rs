use crate::models::checklist::{ChecklistTemplate, ChecklistItem};
use crate::repositories::checklist_repo::ChecklistRepository;
use tauri::command;

#[command]
pub fn create_checklist_template(title: String, items: Vec<String>) -> Result<String, String> {
    ChecklistRepository::create_template(&title, items).map_err(|e| e.to_string())
}

#[command]
pub fn get_checklist_templates() -> Result<Vec<ChecklistTemplate>, String> {
    ChecklistRepository::get_templates().map_err(|e| e.to_string())
}

#[command]
pub fn get_checklist_template_items(id: String) -> Result<Vec<String>, String> {
    ChecklistRepository::get_template_items(&id).map_err(|e| e.to_string())
}

#[command]
pub fn delete_checklist_template(id: String) -> Result<(), String> {
    ChecklistRepository::delete_template(&id).map_err(|e| e.to_string())
}

#[command]
pub fn update_checklist_template(id: String, title: String, items: Vec<String>) -> Result<(), String> {
    ChecklistRepository::update_template(&id, &title, items).map_err(|e| e.to_string())
}

#[command]
pub fn save_service_order_checklist(os_id: String, items: Vec<ChecklistItem>) -> Result<(), String> {
    ChecklistRepository::save_os_checklist(&os_id, items).map_err(|e| e.to_string())
}

#[command]
pub fn get_service_order_checklist(os_id: String) -> Result<Vec<ChecklistItem>, String> {
    ChecklistRepository::get_os_checklist(&os_id).map_err(|e| e.to_string())
}
