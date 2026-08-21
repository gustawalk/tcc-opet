use crate::error::AppError;
use crate::lan_auth::{LanAuthService, PairingCode};
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_server::tls_rustls::RustlsConfig;
use chrono::Utc;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

const API_VERSION: &str = "1";
const APP_VERSION_HEADER: &str = "x-opets-version";
const CERTIFICATE_FILE: &str = "lan-host-cert.pem";
const PRIVATE_KEY_FILE: &str = "lan-host-key.pem";
const PAIRING_CODE_TTL_SECONDS: i64 = 10 * 60;

#[derive(Clone)]
struct ApiState {
    auth: Arc<LanAuthService>,
}

#[derive(Clone, Debug)]
pub(crate) struct TlsIdentity {
    certificate_pem: Vec<u8>,
    private_key_pem: Vec<u8>,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanHostStatus {
    pub address: SocketAddr,
    pub certificate_fingerprint: String,
    pub pairing_code: PairingCode,
}

#[derive(Debug)]
struct RunningLanServer {
    handle: axum_server::Handle<SocketAddr>,
    status: LanHostStatus,
}

static HOST_SERVER: LazyLock<Mutex<Option<RunningLanServer>>> = LazyLock::new(|| Mutex::new(None));
static HOST_START_ERROR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static TLS_PROVIDER: LazyLock<()> = LazyLock::new(|| {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
});

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    api_version: &'static str,
    app_version: &'static str,
    mode: &'static str,
    server_time: String,
    database_ready: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequest {
    code: String,
    device_name: String,
    app_version: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    error: AppError,
}

impl ApiError {
    fn new(status: StatusCode, error: AppError) -> Self {
        Self { status, error }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({ "en": self.error.en, "pt": self.error.pt })),
        )
            .into_response()
    }
}

pub(crate) fn start_configured_host() -> Result<(), AppError> {
    let config = crate::database::storage_mode_config();
    if config.mode != crate::database::StorageMode::Host {
        return Ok(());
    }
    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.host_port));
    match start_host_server(&crate::database::app_data_dir(), address) {
        Ok(server) => {
            if let Ok(mut error) = HOST_START_ERROR.lock() {
                *error = None;
            }
            *HOST_SERVER.lock().map_err(|_| {
                lan_error(
                    "LAN server status is unavailable.",
                    "O status do servidor LAN está indisponível.",
                )
            })? = Some(server);
            Ok(())
        }
        Err(error) => {
            eprintln!("[LAN] Host server did not start: {}", error.pt);
            if let Ok(mut status_error) = HOST_START_ERROR.lock() {
                *status_error = Some(error.pt);
            }
            Ok(())
        }
    }
}

pub(crate) fn stop_configured_host() {
    if let Ok(mut server) = HOST_SERVER.lock() {
        if let Some(server) = server.take() {
            server
                .handle
                .graceful_shutdown(Some(Duration::from_secs(2)));
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanHostRuntimeStatus {
    pub server: Option<LanHostStatus>,
    pub startup_error: Option<String>,
}

#[allow(dead_code)]
pub(crate) fn configured_host_status() -> LanHostRuntimeStatus {
    let server = HOST_SERVER
        .lock()
        .ok()
        .and_then(|server| server.as_ref().map(|server| server.status.clone()));
    let startup_error = HOST_START_ERROR.lock().ok().and_then(|error| error.clone());
    LanHostRuntimeStatus {
        server,
        startup_error,
    }
}

fn start_host_server(
    app_data_dir: &Path,
    address: SocketAddr,
) -> Result<RunningLanServer, AppError> {
    let listener = TcpListener::bind(address).map_err(|error| {
        lan_error(
            format!("Failed to bind LAN server port: {error}"),
            format!("Não foi possível abrir a porta do servidor LAN: {error}"),
        )
    })?;
    listener.set_nonblocking(true).map_err(io_lan_error)?;
    let bound_address = listener.local_addr().map_err(io_lan_error)?;
    let identity = load_or_create_tls_identity(app_data_dir).map_err(io_lan_error)?;
    LazyLock::force(&TLS_PROVIDER);
    let tls = tauri::async_runtime::block_on(RustlsConfig::from_pem(
        identity.certificate_pem.clone(),
        identity.private_key_pem,
    ))
    .map_err(io_lan_error)?;
    let auth = Arc::new(LanAuthService::default());
    let pairing_code = auth.create_pairing_code(PAIRING_CODE_TTL_SECONDS, Utc::now())?;
    let router = Router::new()
        .route("/health", get(health))
        .route("/pair", post(pair))
        .route("/auth-check", get(auth_check))
        .with_state(ApiState { auth });
    let handle = axum_server::Handle::new();
    let server_handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = axum_server::from_tcp_rustls(listener, tls)
            .expect("pre-bound LAN listener must be valid")
            .handle(server_handle)
            .serve(router.into_make_service())
            .await
        {
            eprintln!("[LAN] Host server stopped with error: {error}");
        }
    });
    Ok(RunningLanServer {
        handle,
        status: LanHostStatus {
            address: bound_address,
            certificate_fingerprint: identity.fingerprint,
            pairing_code,
        },
    })
}

async fn health(headers: HeaderMap) -> Result<Json<HealthResponse>, ApiError> {
    require_matching_version(&headers)?;
    Ok(Json(HealthResponse {
        api_version: API_VERSION,
        app_version: env!("CARGO_PKG_VERSION"),
        mode: "host",
        server_time: Utc::now().to_rfc3339(),
        database_ready: crate::database::get_db().is_ok(),
    }))
}

async fn pair(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<PairRequest>,
) -> Result<Json<crate::lan_auth::PairedDevice>, ApiError> {
    require_matching_version(&headers)?;
    let conn = crate::database::get_db()
        .map_err(|error| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, error.into()))?;
    state
        .auth
        .pair_device(
            &conn,
            &request.code,
            &request.device_name,
            &request.app_version,
            env!("CARGO_PKG_VERSION"),
            Utc::now(),
        )
        .map(Json)
        .map_err(|error| ApiError::new(StatusCode::UNAUTHORIZED, error))
}

async fn auth_check(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_matching_version(&headers)?;
    let token = bearer_token(&headers)?;
    let conn = crate::database::get_db()
        .map_err(|error| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, error.into()))?;
    let device = state
        .auth
        .authenticate(&conn, token, Utc::now())
        .map_err(|error| ApiError::new(StatusCode::UNAUTHORIZED, error))?;
    Ok(Json(
        json!({ "deviceId": device.id, "deviceName": device.name }),
    ))
}

fn require_matching_version(headers: &HeaderMap) -> Result<(), ApiError> {
    let version = headers
        .get(APP_VERSION_HEADER)
        .and_then(|value| value.to_str().ok());
    if version != Some(env!("CARGO_PKG_VERSION")) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            lan_error(
                "Client and host application builds must match exactly.",
                "As versões do aplicativo cliente e host devem ser exatamente iguais.",
            ),
        ));
    }
    Ok(())
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                lan_error(
                    "A valid device token is required.",
                    "É necessário um token de dispositivo válido.",
                ),
            )
        })
}

fn load_or_create_tls_identity(app_data_dir: &Path) -> io::Result<TlsIdentity> {
    crate::database::ensure_private_dir(app_data_dir)?;
    let certificate_path = app_data_dir.join(CERTIFICATE_FILE);
    let key_path = app_data_dir.join(PRIVATE_KEY_FILE);
    match (certificate_path.exists(), key_path.exists()) {
        (true, true) => identity_from_files(&certificate_path, &key_path),
        (false, false) => {
            let mut subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
            if let Ok(interfaces) = if_addrs::get_if_addrs() {
                for interface in interfaces {
                    let address = interface.ip().to_string();
                    if !subject_alt_names.contains(&address) {
                        subject_alt_names.push(address);
                    }
                }
            }
            let CertifiedKey { cert, signing_key } =
                generate_simple_self_signed(subject_alt_names).map_err(io::Error::other)?;
            write_private_file(&certificate_path, cert.pem().as_bytes())?;
            write_private_file(&key_path, signing_key.serialize_pem().as_bytes())?;
            identity_from_files(&certificate_path, &key_path)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "LAN TLS certificate and private key must exist together.",
        )),
    }
}

fn identity_from_files(certificate_path: &Path, key_path: &Path) -> io::Result<TlsIdentity> {
    let certificate_pem = fs::read(certificate_path)?;
    let private_key_pem = fs::read(key_path)?;
    crate::database::secure_private_file(certificate_path)?;
    crate::database::secure_private_file(key_path)?;
    Ok(TlsIdentity {
        fingerprint: certificate_fingerprint(&certificate_pem),
        certificate_pem,
        private_key_pem,
    })
}

fn write_private_file(path: &PathBuf, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)?;
    crate::database::secure_private_file(path)
}

pub(crate) fn certificate_fingerprint(certificate_pem: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(certificate_pem).to_hex())
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn verify_certificate_fingerprint(
    certificate_pem: &[u8],
    expected_fingerprint: &str,
) -> Result<(), AppError> {
    if certificate_fingerprint(certificate_pem) != expected_fingerprint {
        return Err(lan_error(
            "Host certificate changed. Pair this device again before connecting.",
            "O certificado do host mudou. Pareie este dispositivo novamente antes de conectar.",
        ));
    }
    Ok(())
}

fn io_lan_error(error: io::Error) -> AppError {
    lan_error(
        format!("LAN server error: {error}"),
        format!("Erro no servidor LAN: {error}"),
    )
}

fn lan_error(en: impl Into<String>, pt: impl Into<String>) -> AppError {
    AppError::new(en, pt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn pinned_agent(certificate_pem: &[u8]) -> ureq::Agent {
        let certificate = ureq::tls::Certificate::from_pem(certificate_pem).unwrap();
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .https_only(true)
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::new_with_certs(&[certificate]))
                    .build(),
            )
            .build()
            .new_agent()
    }

    #[test]
    fn lan_api_tls_identity_persists_and_changed_certificate_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let first = load_or_create_tls_identity(directory.path()).unwrap();
        let second = load_or_create_tls_identity(directory.path()).unwrap();

        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.certificate_pem, second.certificate_pem);
        verify_certificate_fingerprint(&second.certificate_pem, &first.fingerprint).unwrap();
        assert!(
            verify_certificate_fingerprint(b"different-certificate", &first.fingerprint)
                .unwrap_err()
                .pt
                .contains("mudou")
        );
    }

    #[cfg(unix)]
    #[test]
    fn lan_api_tls_private_key_has_private_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        load_or_create_tls_identity(directory.path()).unwrap();

        let mode = fs::metadata(directory.path().join(PRIVATE_KEY_FILE))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn lan_api_bind_failure_does_not_replace_running_local_storage() {
        let _backend = crate::test_helpers::setup_global_backend();
        let directory = tempfile::tempdir().unwrap();
        let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = occupied.local_addr().unwrap();

        let error = start_host_server(directory.path(), address).unwrap_err();

        assert!(error.pt.contains("porta"));
        assert!(crate::database::get_db().is_ok());
    }

    #[test]
    fn lan_api_plaintext_request_is_not_served_as_http() {
        let directory = tempfile::tempdir().unwrap();
        let server =
            start_host_server(directory.path(), SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
                .unwrap();
        let mut stream = std::net::TcpStream::connect(server.status.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);

        assert!(!response.starts_with(b"HTTP/1.1 200"));
        server.handle.shutdown();
    }

    #[test]
    fn lan_api_version_and_bearer_headers_are_strict() {
        let mut headers = HeaderMap::new();
        assert_eq!(
            require_matching_version(&headers).unwrap_err().status,
            StatusCode::CONFLICT
        );
        headers.insert(
            APP_VERSION_HEADER,
            env!("CARGO_PKG_VERSION").parse().unwrap(),
        );
        assert!(require_matching_version(&headers).is_ok());
        assert_eq!(
            bearer_token(&headers).unwrap_err().status,
            StatusCode::UNAUTHORIZED
        );
        headers.insert(header::AUTHORIZATION, "Bearer token".parse().unwrap());
        assert_eq!(bearer_token(&headers).unwrap(), "token");
    }

    #[test]
    fn lan_api_https_health_pairing_and_authentication_contract() {
        let _backend = crate::test_helpers::setup_global_backend();
        let directory = tempfile::tempdir().unwrap();
        let server =
            start_host_server(directory.path(), SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
                .unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let certificate = fs::read(directory.path().join(CERTIFICATE_FILE)).unwrap();
        let agent = pinned_agent(&certificate);
        let base_url = format!("https://localhost:{}", server.status.address.port());

        let mut health = agent
            .get(format!("{base_url}/health"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .call()
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let health_body: serde_json::Value =
            serde_json::from_str(&health.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(health_body["appVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(health_body["apiVersion"], API_VERSION);
        assert_eq!(health_body["mode"], "host");
        assert_eq!(health_body["databaseReady"], true);

        let wrong_version = agent
            .get(format!("{base_url}/health"))
            .header(APP_VERSION_HEADER, "0.0.0")
            .call()
            .unwrap();
        assert_eq!(wrong_version.status(), StatusCode::CONFLICT);

        let pairing_body = serde_json::to_vec(&json!({
            "code": server.status.pairing_code.code,
            "deviceName": "Balcao 2",
            "appVersion": env!("CARGO_PKG_VERSION")
        }))
        .unwrap();
        let mut pairing = agent
            .post(format!("{base_url}/pair"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .send(pairing_body)
            .unwrap();
        assert_eq!(pairing.status(), StatusCode::OK);
        let paired: serde_json::Value =
            serde_json::from_str(&pairing.body_mut().read_to_string().unwrap()).unwrap();
        let token = paired["token"].as_str().unwrap();
        let device_id = paired["deviceId"].as_str().unwrap();

        let missing_token = agent
            .get(format!("{base_url}/auth-check"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .call()
            .unwrap();
        assert_eq!(missing_token.status(), StatusCode::UNAUTHORIZED);

        let invalid_token = agent
            .get(format!("{base_url}/auth-check"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), "Bearer invalid-token")
            .call()
            .unwrap();
        assert_eq!(invalid_token.status(), StatusCode::UNAUTHORIZED);

        let authenticated = agent
            .get(format!("{base_url}/auth-check"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .call()
            .unwrap();
        assert_eq!(authenticated.status(), StatusCode::OK);

        let conn = crate::database::get_db().unwrap();
        conn.execute(
            "UPDATE lan_devices SET revoked_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [device_id],
        )
        .unwrap();
        drop(conn);
        let revoked = agent
            .get(format!("{base_url}/auth-check"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .call()
            .unwrap();
        assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);

        let other_directory = tempfile::tempdir().unwrap();
        let other_identity = load_or_create_tls_identity(other_directory.path()).unwrap();
        let wrong_agent = pinned_agent(&other_identity.certificate_pem);
        assert!(wrong_agent
            .get(format!("{base_url}/health"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .call()
            .is_err());
        server.handle.shutdown();
    }
}
