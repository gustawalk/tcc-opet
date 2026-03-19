use crate::models::settings::Settings;
use crate::repositories::settings_repo::SettingsRepository;
use tauri::command;

#[command]
pub fn get_settings() -> Result<Settings, String> {
    SettingsRepository::get_settings().map_err(|e| e.to_string())
}

#[command]
pub fn update_settings(settings: Settings) -> Result<(), String> {
    SettingsRepository::update_settings(&settings).map_err(|e| e.to_string())
}

#[command]
pub fn reset_database() -> Result<(), String> {
    let conn = crate::database::get_db().map_err(|e| e.to_string())?;
    
    // Disable foreign keys temporarily to drop everything
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP TABLE IF EXISTS service_order_parts;
        DROP TABLE IF EXISTS service_order_checklists;
        DROP TABLE IF EXISTS template_items;
        DROP TABLE IF EXISTS checklist_templates;
        DROP TABLE IF EXISTS service_orders;
        DROP TABLE IF EXISTS inventory_items;
        DROP TABLE IF EXISTS users;
        DROP TABLE IF EXISTS customers;
        DROP TABLE IF EXISTS financial_snapshots;
        DROP TABLE IF EXISTS settings;
        PRAGMA foreign_keys = ON;
        "
    ).map_err(|e| e.to_string())?;

    // Re-run migrations to recreate tables
    crate::database::init_db().map_err(|e| e.to_string())?;
    
    Ok(())
}
