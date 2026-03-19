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
