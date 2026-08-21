use crate::error::AppError;
use serde_json::Value;
use tauri::command;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanDeviceInfo {
    id: String,
    name: String,
    app_version: String,
    created_at: String,
    last_seen_at: Option<String>,
    revoked_at: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanHostUiStatus {
    running: bool,
    address: Option<String>,
    verification_code: Option<String>,
    certificate_fingerprint: Option<String>,
    startup_error: Option<String>,
}

#[command]
pub fn get_lan_host_status() -> LanHostUiStatus {
    host_ui_status(crate::lan_api::configured_host_status())
}

fn host_ui_status(status: crate::lan_api::LanHostRuntimeStatus) -> LanHostUiStatus {
    let (running, address, verification_code, certificate_fingerprint) = match status.server {
        Some(server) => (
            true,
            Some(
                if_addrs::get_if_addrs()
                    .ok()
                    .and_then(|interfaces| {
                        interfaces
                            .into_iter()
                            .map(|interface| interface.ip())
                            .find(|address| address.is_ipv4() && !address.is_loopback())
                    })
                    .map(|address| format!("{address}:{}", server.address.port()))
                    .unwrap_or_else(|| server.address.to_string()),
            ),
            Some(format!(
                "{}|{}",
                server.pairing_code.code, server.certificate_fingerprint
            )),
            Some(server.certificate_fingerprint),
        ),
        None => (false, None, None, None),
    };
    LanHostUiStatus {
        running,
        address,
        verification_code,
        certificate_fingerprint,
        startup_error: status.startup_error,
    }
}

#[command]
pub fn regenerate_lan_pairing_code() -> Result<LanHostUiStatus, AppError> {
    Ok(host_ui_status(crate::lan_api::regenerate_pairing_code()?))
}

#[command]
pub fn list_lan_devices() -> Result<Vec<LanDeviceInfo>, AppError> {
    ensure_host_mode()?;
    let conn = crate::database::get_db()?;
    let mut statement = conn.prepare(
        "SELECT id, name, app_version, created_at, last_seen_at, revoked_at
         FROM lan_devices ORDER BY created_at DESC",
    )?;
    let devices = statement
        .query_map([], |row| {
            Ok(LanDeviceInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                app_version: row.get(2)?,
                created_at: row.get(3)?,
                last_seen_at: row.get(4)?,
                revoked_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(devices)
}

#[command]
pub fn revoke_lan_device(id: String) -> Result<(), AppError> {
    ensure_host_mode()?;
    let conn = crate::database::get_db()?;
    let changed = conn.execute(
        "UPDATE lan_devices SET revoked_at = COALESCE(revoked_at, CURRENT_TIMESTAMP) WHERE id = ?1",
        [&id],
    )?;
    if changed == 0 {
        return Err(AppError::new(
            "LAN device was not found.",
            "O dispositivo LAN não foi encontrado.",
        ));
    }
    Ok(())
}

fn ensure_host_mode() -> Result<(), AppError> {
    if crate::database::storage_mode_config().mode != crate::database::StorageMode::Host {
        return Err(AppError::new(
            "Device management is available only in Host mode.",
            "O gerenciamento de dispositivos está disponível somente no modo Host.",
        ));
    }
    Ok(())
}

#[command]
pub fn pair_lan_client(
    url: String,
    device_name: String,
    verification_code: String,
) -> Result<crate::commands::settings_commands::LanModeStatus, AppError> {
    crate::lan_client::pair_client(&url, &device_name, &verification_code)
}

#[command]
pub fn check_lan_client_connection() -> Result<Value, AppError> {
    crate::lan_client::check_connection()
}

#[command]
pub fn download_lan_remote_backup(
    destination: String,
    passphrase: Option<String>,
) -> Result<crate::backup_service::BackupSummary, AppError> {
    crate::lan_client::download_remote_backup(&destination, passphrase.as_deref())
}

#[command]
pub fn run_scheduled_lan_remote_backup(
    destination_directory: String,
) -> Result<crate::backup_service::BackupSummary, AppError> {
    let destination = std::path::Path::new(&destination_directory).join(format!(
        "opets-lan-auto-{}.osbkp",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));
    crate::lan_client::download_remote_backup(&destination.to_string_lossy(), None)
}

#[command]
pub fn lan_remote_command(
    operation: String,
    payload: Value,
    idempotency_key: Option<String>,
) -> Result<Value, AppError> {
    crate::lan_client::remote_command(&operation, payload, idempotency_key.as_deref())
}
