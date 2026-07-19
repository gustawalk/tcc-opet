use crate::error::AppError;
use crate::models::settings::Settings;
use crate::repositories::settings_repo::SettingsRepository;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{command, AppHandle};
use tauri_plugin_dialog::DialogExt;

const MAX_LOGO_SIZE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub database_path: String,
    pub app_version: String,
    pub tauri_version: String,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub configured: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateManifest {
    version: String,
    download_url: Option<String>,
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |version: &str| {
        let mut parts = version
            .trim_start_matches('v')
            .split(['-', '+'])
            .next()
            .unwrap_or_default()
            .split('.')
            .map(|part| part.parse::<u64>().ok());
        [
            parts.next().flatten().unwrap_or_default(),
            parts.next().flatten().unwrap_or_default(),
            parts.next().flatten().unwrap_or_default(),
        ]
    };

    parse(latest) > parse(current)
}

#[command]
pub fn get_settings() -> Result<Settings, AppError> {
    Ok(SettingsRepository::get_settings()?)
}

#[command]
pub fn update_settings(settings: Settings) -> Result<(), AppError> {
    Ok(SettingsRepository::update_settings(&settings)?)
}

#[command]
pub async fn reset_database() -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(reset_database_data)
        .await
        .map_err(|error| {
            AppError::new(
                format!("Failed to reset database: {error}"),
                format!("Erro ao resetar os dados: {error}"),
            )
        })?
}

fn reset_database_data() -> Result<(), AppError> {
    let conn = crate::database::get_db()?;
    reset_database_with_conn(&conn)?;
    drop(conn);
    let attachments_dir = crate::database::attachments_dir();
    if attachments_dir.exists() {
        std::fs::remove_dir_all(attachments_dir).map_err(|error| {
            AppError::new(
                format!("Failed to remove attachment storage: {error}"),
                format!("Erro ao remover o armazenamento de anexos: {error}"),
            )
        })?;
    }
    Ok(())
}

fn reset_database_with_conn(conn: &rusqlite::Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP TABLE IF EXISTS service_order_attachments;
        DROP TABLE IF EXISTS service_order_events;
        DROP TABLE IF EXISTS service_order_parts;
        DROP TABLE IF EXISTS service_order_checklists;
        DROP TABLE IF EXISTS template_items;
        DROP TABLE IF EXISTS checklist_templates;
        DROP TABLE IF EXISTS inventory_movements;
        DROP TABLE IF EXISTS service_orders;
        DROP TABLE IF EXISTS service_order_sequences;
        DROP TABLE IF EXISTS inventory_items;
        DROP TABLE IF EXISTS users;
        DROP TABLE IF EXISTS customers;
        DROP TABLE IF EXISTS financial_snapshots;
        DROP TABLE IF EXISTS settings;
        PRAGMA foreign_keys = ON;
        ",
    )?;
    crate::database::run_migrations(conn)?;
    Ok(())
}

#[command]
pub fn get_system_info() -> Result<SystemInfo, AppError> {
    Ok(SystemInfo {
        database_path: crate::database::database_path()
            .to_string_lossy()
            .to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        environment: if cfg!(debug_assertions) {
            "Desenvolvimento".to_string()
        } else {
            "Produção".to_string()
        },
    })
}

#[command]
pub fn check_for_updates() -> Result<UpdateCheck, AppError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let manifest_url = match std::env::var("UPDATE_MANIFEST_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            return Ok(UpdateCheck {
                configured: false,
                current_version,
                latest_version: None,
                update_available: false,
                download_url: None,
            });
        }
    };

    let mut response = ureq::get(&manifest_url).call().map_err(|error| {
        AppError::new(
            format!("Failed to check for updates: {error}"),
            format!("Erro ao verificar atualizações: {error}"),
        )
    })?;
    let body = response.body_mut().read_to_string().map_err(|error| {
        AppError::new(
            format!("Failed to read update manifest: {error}"),
            format!("Erro ao ler o manifesto de atualização: {error}"),
        )
    })?;
    let manifest: UpdateManifest = serde_json::from_str(&body).map_err(|error| {
        AppError::new(
            format!("Invalid update manifest: {error}"),
            format!("Manifesto de atualização inválido: {error}"),
        )
    })?;

    Ok(UpdateCheck {
        configured: true,
        update_available: is_newer_version(&manifest.version, &current_version),
        current_version,
        latest_version: Some(manifest.version),
        download_url: manifest.download_url,
    })
}

#[command]
pub fn export_backup(
    destination: String,
) -> Result<crate::backup_service::BackupSummary, AppError> {
    crate::backup_service::export_backup_with_paths(
        &crate::database::database_path(),
        &crate::database::attachments_dir(),
        Path::new(&destination),
    )
}

#[command]
pub fn restore_backup(source: String) -> Result<crate::backup_service::BackupSummary, AppError> {
    crate::backup_service::restore_backup_with_paths(
        Path::new(&source),
        &crate::database::database_path(),
        &crate::database::attachments_dir(),
    )
}

fn logo_data_url(path: &Path) -> Result<String, AppError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        AppError::new(
            format!("Failed to read logo metadata: {error}"),
            format!("Erro ao ler os metadados da logo: {error}"),
        )
    })?;
    if !metadata.file_type().is_file() {
        return Err(crate::error::business_error(
            "Logo must be a regular file.",
            "A logo deve ser um arquivo regular.",
        ));
    }
    if metadata.len() > MAX_LOGO_SIZE_BYTES {
        return Err(crate::error::business_error(
            "Logo exceeds the 2 MB limit.",
            "A logo excede o limite de 2 MB.",
        ));
    }
    let bytes = std::fs::read(path).map_err(|error| {
        AppError::new(
            format!("Failed to read logo: {error}"),
            format!("Erro ao ler a logo: {error}"),
        )
    })?;
    let mime_type = infer::get(&bytes)
        .map(|kind| kind.mime_type())
        .filter(|mime| matches!(*mime, "image/png" | "image/jpeg" | "image/webp"))
        .ok_or_else(|| {
            crate::error::business_error(
                "Only valid PNG, JPEG, and WEBP logos are supported.",
                "Apenas logos PNG, JPEG e WEBP válidas são aceitas.",
            )
        })?;
    Ok(format!(
        "data:{mime_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[command]
pub async fn select_company_logo(app: AppHandle) -> Result<Option<String>, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .add_filter("Imagens", &["png", "jpg", "jpeg", "webp"])
            .blocking_pick_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|error| {
            AppError::new(
                format!("Failed to access selected logo: {error}"),
                format!("Erro ao acessar a logo selecionada: {error}"),
            )
        })?;
        logo_data_url(&path).map(Some)
    })
    .await
    .map_err(|error| {
        AppError::new(
            format!("Failed to select logo: {error}"),
            format!("Erro ao selecionar a logo: {error}"),
        )
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::create_temp_file;

    #[test]
    fn encodes_valid_png_logo_contents() {
        let path = create_temp_file("settings_command", "png", b"\x89PNG\r\n\x1a\n");

        let result = logo_data_url(&path).unwrap();

        assert!(result.starts_with("data:image/png;base64,"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_logo_with_invalid_contents() {
        let path = create_temp_file("settings_command", "png", b"not-an-image");

        assert!(logo_data_url(&path).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn returns_bilingual_error_when_logo_is_missing() {
        let result = logo_data_url(Path::new("/tmp/definitely-missing-file.png"));

        let err = result.unwrap_err();
        assert!(err.en.starts_with("Failed to read logo metadata:"));
        assert!(err.pt.starts_with("Erro ao ler os metadados da logo:"));
    }

    #[test]
    fn reset_database_recreates_all_core_tables() {
        let conn = crate::test_helpers::setup_db();
        conn.execute(
            "INSERT INTO customers (id, name) VALUES ('customer-1', 'Ana')",
            [],
        )
        .unwrap();

        reset_database_with_conn(&conn).unwrap();

        let customer_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM customers", [], |row| row.get(0))
            .unwrap();
        let movement_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'inventory_movements'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(customer_count, 0);
        assert_eq!(movement_table_exists, 1);
    }

    #[test]
    fn compares_semantic_versions_without_prerelease_suffixes() {
        assert!(is_newer_version("v1.2.0", "1.1.9"));
        assert!(is_newer_version("1.0.1", "1.0.0-beta.1"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.9.9", "1.0.0"));
    }
}
