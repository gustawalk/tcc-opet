use crate::error::AppError;
use crate::models::settings::Settings;
use crate::repositories::settings_repo::SettingsRepository;
use base64::Engine;
use std::path::Path;
use tauri::command;

#[command]
pub fn get_settings() -> Result<Settings, AppError> {
    Ok(SettingsRepository::get_settings()?)
}

#[command]
pub fn update_settings(settings: Settings) -> Result<(), AppError> {
    Ok(SettingsRepository::update_settings(&settings)?)
}

#[command]
pub fn reset_database() -> Result<(), AppError> {
    let conn = crate::database::get_db()?;

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
    )?;

    crate::database::init_db()?;

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
pub fn read_file_as_base64(path: String) -> Result<String, AppError> {
    let file_path = Path::new(&path);
    let mime = mime_from_path(file_path);
    let bytes = std::fs::read(file_path).map_err(|e| AppError::new(
        format!("Failed to read file: {}", e),
        format!("Erro ao ler arquivo: {}", e),
    ))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::create_temp_file;

    #[test]
    fn detects_png_mime_and_encodes_contents() {
        let path = create_temp_file("settings_command", "png", b"abc");

        let result = read_file_as_base64(path.to_string_lossy().to_string()).unwrap();

        assert!(result.starts_with("data:image/png;base64,"));
        assert!(result.ends_with("YWJj"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn detects_jpeg_mime_and_encodes_contents() {
        let path = create_temp_file("settings_command", "jpg", b"xyz");

        let result = read_file_as_base64(path.to_string_lossy().to_string()).unwrap();

        assert!(result.starts_with("data:image/jpeg;base64,"));
        assert!(result.ends_with("eHl6"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn falls_back_to_png_mime_for_unknown_extension() {
        let path = create_temp_file("settings_command", "bin", b"raw");

        let result = read_file_as_base64(path.to_string_lossy().to_string()).unwrap();

        assert!(result.starts_with("data:image/png;base64,"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn returns_bilingual_error_when_file_is_missing() {
        let result = read_file_as_base64("/tmp/definitely-missing-file.png".to_string());

        let err = result.unwrap_err();
        assert!(err.en.starts_with("Failed to read file:"));
        assert!(err.pt.starts_with("Erro ao ler arquivo:"));
    }
}
