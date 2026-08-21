use crate::error::AppError;
use serde_json::Value;
use tauri::command;

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
    let status = crate::lan_api::configured_host_status();
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
