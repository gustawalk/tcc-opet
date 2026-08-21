use crate::database::{StorageMode, StorageModeConfig};
use crate::error::AppError;
use serde::Deserialize;
use serde_json::Value;

const APP_VERSION_HEADER: &str = "x-opets-version";

#[derive(Deserialize)]
struct RemoteError {
    en: String,
    pt: String,
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

    let certificate =
        ureq::tls::Certificate::from_pem(certificate_pem.as_bytes()).map_err(|_| {
            client_error(
                "Stored host certificate is invalid.",
                "O certificado armazenado do host é inválido.",
            )
        })?;
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .https_only(true)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::new_with_certs(&[certificate]))
                .build(),
        )
        .build()
        .new_agent();
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
    let mut response = request.send(body).map_err(|error| {
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
