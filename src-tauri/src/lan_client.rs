use crate::database::{StorageMode, StorageModeConfig};
use crate::error::AppError;
use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

const APP_VERSION_HEADER: &str = "x-opets-version";

#[derive(Deserialize)]
struct RemoteError {
    en: String,
    pt: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CertificateResponse {
    certificate_pem: String,
}

#[derive(Deserialize)]
struct PairResponse {
    token: String,
}

pub(crate) fn pair_client(
    url: &str,
    device_name: &str,
    verification_code: &str,
) -> Result<crate::commands::settings_commands::LanModeStatus, AppError> {
    let url = url.trim().trim_end_matches('/');
    let device_name = device_name.trim();
    let (pairing_code, expected_fingerprint) =
        verification_code.trim().split_once('|').ok_or_else(|| {
            client_error(
                "Invalid LAN verification code.",
                "O código de verificação LAN é inválido.",
            )
        })?;
    if device_name.is_empty() {
        return Err(client_error(
            "Device name is required.",
            "Informe um nome para este computador.",
        ));
    }
    let mut candidate = StorageModeConfig {
        mode: StorageMode::Client,
        host_port: 8743,
        client_url: Some(url.to_string()),
        client_device_name: Some(device_name.to_string()),
        client_token: None,
        client_certificate_fingerprint: None,
        client_certificate_pem: None,
    };
    candidate.validate().map_err(|error| {
        client_error(
            format!("Invalid LAN client configuration: {error}"),
            format!("Configuração LAN inválida: {error}"),
        )
    })?;

    // Only the public certificate is fetched before the out-of-band pin is checked.
    let insecure_agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .https_only(true)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .disable_verification(true)
                .build(),
        )
        .build()
        .new_agent();
    let certificate: CertificateResponse = read_json_response(
        insecure_agent
            .get(format!("{url}/certificate"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .call(),
    )?;
    crate::lan_api::verify_certificate_fingerprint(
        certificate.certificate_pem.as_bytes(),
        expected_fingerprint,
    )?;

    let agent = pinned_agent(&certificate.certificate_pem)?;
    let body = serde_json::to_vec(&serde_json::json!({
        "code": pairing_code,
        "deviceName": device_name,
        "appVersion": env!("CARGO_PKG_VERSION"),
    }))
    .map_err(|error| {
        client_error(
            format!("Failed to serialize pairing request: {error}"),
            "Não foi possível preparar o pareamento.",
        )
    })?;
    let paired: PairResponse = read_json_response(
        agent
            .post(format!("{url}/pair"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header("content-type", "application/json")
            .send(body),
    )?;
    candidate.client_token = Some(paired.token);
    candidate.client_certificate_fingerprint = Some(expected_fingerprint.to_string());
    candidate.client_certificate_pem = Some(certificate.certificate_pem);
    crate::database::update_storage_mode_config(&candidate).map_err(|error| {
        client_error(
            format!("Failed to save LAN pairing: {error}"),
            format!("Não foi possível salvar o pareamento LAN: {error}"),
        )
    })?;
    Ok(crate::commands::settings_commands::LanModeStatus {
        config: candidate,
        active_mode: crate::database::storage_mode_config().mode,
        restart_required: true,
        storage_ready: false,
    })
}

pub(crate) fn check_connection() -> Result<Value, AppError> {
    let config = crate::database::storage_mode_config();
    ensure_client_mode(&config)?;
    let base_url = required(&config.client_url, "host URL", "endereço do host")?;
    let token = required(&config.client_token, "device token", "token do dispositivo")?;
    let agent = pinned_agent(verified_certificate(&config)?)?;
    read_json_response(
        agent
            .get(format!("{}/auth-check", base_url.trim_end_matches('/')))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header("authorization", format!("Bearer {token}"))
            .call(),
    )
}

pub(crate) fn download_remote_backup(
    destination: &str,
    passphrase: Option<&str>,
) -> Result<crate::backup_service::BackupSummary, AppError> {
    let value = remote_command(
        "create_remote_backup_download",
        serde_json::json!({ "passphrase": passphrase }),
        None,
    )?;
    let data = value
        .get("dataBase64")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            client_error(
                "Backup response has no data.",
                "O host não enviou o backup.",
            )
        })?;
    let attachment_count = value
        .get("attachmentCount")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|error| {
            client_error(
                format!("Invalid remote backup data: {error}"),
                "O backup recebido do host é inválido.",
            )
        })?;
    std::fs::write(destination, bytes).map_err(|error| {
        client_error(
            format!("Failed to save remote backup: {error}"),
            format!("Não foi possível salvar o backup remoto: {error}"),
        )
    })?;
    Ok(crate::backup_service::BackupSummary {
        path: Path::new(destination).to_string_lossy().into_owned(),
        attachment_count,
    })
}

pub(crate) fn remote_command(
    operation: &str,
    payload: Value,
    idempotency_key: Option<&str>,
) -> Result<Value, AppError> {
    if operation.is_empty()
        || !operation
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(client_error(
            "Invalid LAN operation name.",
            "O nome da operação LAN é inválido.",
        ));
    }
    let config = crate::database::storage_mode_config();
    ensure_client_mode(&config)?;
    let base_url = required(&config.client_url, "host URL", "endereço do host")?;
    let token = required(&config.client_token, "device token", "token do dispositivo")?;
    let agent = pinned_agent(verified_certificate(&config)?)?;
    let url = format!(
        "{}/api/v1/commands/{operation}",
        base_url.trim_end_matches('/')
    );
    let mut request = agent
        .post(url)
        .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    if let Some(key) = idempotency_key {
        request = request.header("x-idempotency-key", key);
    }
    let body = serde_json::to_vec(&payload).map_err(|error| {
        client_error(
            format!("Failed to serialize LAN request: {error}"),
            "Não foi possível preparar a solicitação LAN.",
        )
    })?;
    read_json_response(request.send(body))
}

fn verified_certificate(config: &StorageModeConfig) -> Result<&str, AppError> {
    let fingerprint = required(
        &config.client_certificate_fingerprint,
        "certificate fingerprint",
        "impressão digital do certificado",
    )?;
    let certificate_pem = required(
        &config.client_certificate_pem,
        "pinned certificate",
        "certificado fixado",
    )?;
    crate::lan_api::verify_certificate_fingerprint(certificate_pem.as_bytes(), fingerprint)?;
    Ok(certificate_pem)
}

fn pinned_agent(certificate_pem: &str) -> Result<ureq::Agent, AppError> {
    let certificate =
        ureq::tls::Certificate::from_pem(certificate_pem.as_bytes()).map_err(|_| {
            client_error(
                "Stored host certificate is invalid.",
                "O certificado armazenado do host é inválido.",
            )
        })?;
    Ok(ureq::Agent::config_builder()
        .http_status_as_error(false)
        .https_only(true)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::new_with_certs(&[certificate]))
                .build(),
        )
        .build()
        .new_agent())
}

fn read_json_response<T: for<'de> Deserialize<'de>>(
    response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<T, AppError> {
    let mut response = response.map_err(|error| {
        client_error(
            format!("LAN host is unreachable: {error}"),
            "O computador host está indisponível. Leituras e alterações estão bloqueadas.",
        )
    })?;
    let status = response.status();
    let body = response.body_mut().read_to_string().map_err(|error| {
        client_error(
            format!("Failed to read LAN response: {error}"),
            "Não foi possível ler a resposta do host.",
        )
    })?;
    if !status.is_success() {
        return match serde_json::from_str::<RemoteError>(&body) {
            Ok(error) => Err(AppError::new(error.en, error.pt)),
            Err(_) => Err(client_error(
                format!("LAN host rejected the request with status {status}."),
                "O host recusou a operação solicitada.",
            )),
        };
    }
    serde_json::from_str(&body).map_err(|error| {
        client_error(
            format!("Invalid LAN response: {error}"),
            "O host retornou uma resposta inválida.",
        )
    })
}

fn ensure_client_mode(config: &StorageModeConfig) -> Result<(), AppError> {
    if config.mode != StorageMode::Client {
        return Err(client_error(
            "Remote command requires Client mode.",
            "A operação remota exige o modo Cliente.",
        ));
    }
    Ok(())
}

fn required<'a>(value: &'a Option<String>, en: &str, pt: &str) -> Result<&'a str, AppError> {
    value.as_deref().ok_or_else(|| {
        client_error(
            format!("Client mode is missing the {en}."),
            format!("O modo Cliente não possui {pt}. Faça o pareamento novamente."),
        )
    })
}

fn client_error(en: impl Into<String>, pt: impl Into<String>) -> AppError {
    AppError::new(en, pt)
}
