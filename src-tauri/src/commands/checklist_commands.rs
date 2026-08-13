use crate::error::AppError;
use crate::models::checklist::{ChecklistItem, ChecklistTemplate};
use crate::page::Page;
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

const CHECKLIST_TEMPLATES_PAGE_DEFAULT_LIMIT: u32 = 200;

#[command]
pub fn get_checklist_templates_page(
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
) -> Result<Page<ChecklistTemplate>, AppError> {
    let conn = crate::database::get_db()?;
    let limit = limit
        .unwrap_or(CHECKLIST_TEMPLATES_PAGE_DEFAULT_LIMIT)
        .clamp(1, 1000);
    let offset = offset.unwrap_or(0);
    let search = search.unwrap_or_default();
    let items = ChecklistRepository::get_page_with_conn(&conn, limit, offset, &search)?;
    let total = ChecklistRepository::count_all_with_conn(&conn, &search)?;
    Ok(Page { items, total })
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
