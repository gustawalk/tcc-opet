use crate::models::settings::Settings;
use crate::repositories::settings_repo::SettingsRepository;
use base64::Engine;
use std::path::Path;
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

fn mime_from_path(path: &Path) -> &str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/png",
    }
}

#[command]
pub fn read_file_as_base64(path: String) -> Result<String, String> {
    let file_path = Path::new(&path);
    let mime = mime_from_path(file_path);
    let bytes = std::fs::read(file_path).map_err(|e| format!("Erro ao ler arquivo: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}
