use crate::error::AppError;
use crate::lan_auth::{LanAuthService, PairingCode};
use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
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
    idempotency: Arc<crate::lan_idempotency::LanIdempotencyService>,
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
    let idempotency = Arc::new(crate::lan_idempotency::LanIdempotencyService);
    let pairing_code = auth.create_pairing_code(PAIRING_CODE_TTL_SECONDS, Utc::now())?;
    let router = Router::new()
        .route("/health", get(health))
        .route("/pair", post(pair))
        .route("/auth-check", get(auth_check))
        .route("/api/v1/commands/{operation}", post(product_command))
        .with_state(ApiState { auth, idempotency });
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

async fn product_command(
    State(state): State<ApiState>,
    AxumPath(operation): AxumPath<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_matching_version(&headers)?;
    let token = bearer_token(&headers)?;
    let conn = crate::database::get_db()
        .map_err(|error| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, error.into()))?;
    let device = state
        .auth
        .authenticate(&conn, token, Utc::now())
        .map_err(|error| ApiError::new(StatusCode::UNAUTHORIZED, error))?;
    drop(conn);

    let payload: serde_json::Value = serde_json::from_slice(&body).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            lan_error(
                format!("Invalid command payload: {error}"),
                "Os dados enviados para a operação são inválidos.",
            ),
        )
    })?;
    let execute = || dispatch_catalog_command(&operation, payload);
    let result = if is_catalog_mutation(&operation) {
        let idempotency_key = headers
            .get("x-idempotency-key")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    lan_error(
                        "Idempotency key is required for mutating requests.",
                        "A chave de idempotência é obrigatória para alterações.",
                    ),
                )
            })?;
        let mut idempotency_conn =
            crate::database::open_encrypted_database(&crate::database::database_path())
                .map_err(|error| ApiError::new(StatusCode::SERVICE_UNAVAILABLE, error.into()))?;
        state
            .idempotency
            .execute(
                &mut idempotency_conn,
                &device.id,
                idempotency_key,
                &operation,
                &body,
                execute,
            )
            .map_err(|error| ApiError::new(StatusCode::BAD_REQUEST, error))?
    } else {
        execute().map_err(|error| ApiError::new(StatusCode::BAD_REQUEST, error))?
    };
    Ok(Json(result))
}

fn is_catalog_mutation(operation: &str) -> bool {
    matches!(
        operation,
        "create_customer"
            | "update_customer"
            | "delete_customer"
            | "create_user"
            | "update_user"
            | "delete_user"
            | "create_inventory_item"
            | "update_inventory_item"
            | "delete_inventory_item"
            | "restock_inventory_item"
            | "remove_stock_inventory_item"
            | "create_checklist_template"
            | "update_checklist_template"
            | "delete_checklist_template"
            | "create_full_service_order"
            | "transition_service_order_status"
            | "save_service_order_edit"
            | "delete_service_order"
            | "add_part_to_service_order"
            | "remove_part_from_service_order"
            | "update_service_order_part_quantity"
            | "save_service_order_checklist"
            | "upload_service_order_attachment"
            | "delete_service_order_attachment"
    )
}

fn dispatch_catalog_command(
    operation: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    use crate::commands::facade;

    match operation {
        "create_customer" => {
            let input: CustomerInput = decode(payload)?;
            encode(facade::create_customer(
                input.name,
                input.phone,
                input.email,
                input.address,
            )?)
        }
        "get_customer" => encode(facade::get_customer(decode::<IdInput>(payload)?.id)?),
        "get_customers_page" => {
            let input: PageInput = decode(payload)?;
            encode(facade::get_customers_page(
                input.limit,
                input.offset,
                input.search,
            )?)
        }
        "update_customer" => {
            let input: CustomerInput = decode(payload)?;
            encode(facade::update_customer(
                required_id(input.id)?,
                input.name,
                input.phone,
                input.email,
                input.address,
            )?)
        }
        "delete_customer" => encode(facade::delete_customer(decode::<IdInput>(payload)?.id)?),
        "create_user" => {
            let input: UserInput = decode(payload)?;
            encode(facade::create_user(
                input.name,
                input.email,
                input.phone,
                input.cpf,
                input.join_date,
            )?)
        }
        "get_user" => encode(facade::get_user(decode::<IdInput>(payload)?.id)?),
        "get_users_page" => {
            let input: PageInput = decode(payload)?;
            encode(facade::get_users_page(
                input.limit,
                input.offset,
                input.search,
            )?)
        }
        "update_user" => {
            let input: UserInput = decode(payload)?;
            encode(facade::update_user(
                required_id(input.id)?,
                input.name,
                input.email,
                input.phone,
                input.cpf,
                input.join_date,
            )?)
        }
        "delete_user" => encode(facade::delete_user(decode::<IdInput>(payload)?.id)?),
        "create_inventory_item" => {
            let input: InventoryInput = decode(payload)?;
            encode(facade::create_inventory_item(
                input.name,
                input.description,
                input.item_type,
                input.min_quantity,
                input.current_quantity,
                input.cost_price,
                input.sale_price,
                input.supplier_name,
            )?)
        }
        "get_inventory_item" => encode(facade::get_inventory_item(decode::<IdInput>(payload)?.id)?),
        "get_inventory_items_page" => {
            let input: InventoryPageInput = decode(payload)?;
            encode(facade::get_inventory_items_page(
                input.limit,
                input.offset,
                input.search,
                input.item_type,
            )?)
        }
        "get_inventory_summary" => encode(facade::get_inventory_summary()?),
        "get_inventory_insights" => encode(facade::get_inventory_insights(
            decode::<InventoryInsightsInput>(payload)?.inactive_days,
        )?),
        "get_inventory_movements" => encode(facade::get_inventory_movements(
            decode::<IdInput>(payload)?.id,
        )?),
        "update_inventory_item" => {
            let input: InventoryInput = decode(payload)?;
            encode(facade::update_inventory_item(
                required_id(input.id)?,
                input.name,
                input.description,
                input.item_type,
                input.min_quantity,
                input.current_quantity,
                input.cost_price,
                input.sale_price,
                input.supplier_name,
            )?)
        }
        "delete_inventory_item" => encode(facade::delete_inventory_item(
            decode::<IdInput>(payload)?.id,
        )?),
        "restock_inventory_item" => {
            let input: StockInput = decode(payload)?;
            encode(facade::restock_inventory_item(
                input.id,
                input.quantity,
                input.unit_cost,
                input.reason,
            )?)
        }
        "remove_stock_inventory_item" => {
            let input: StockInput = decode(payload)?;
            encode(facade::remove_stock_inventory_item(
                input.id,
                input.quantity,
            )?)
        }
        "create_checklist_template" => {
            let input: ChecklistInput = decode(payload)?;
            encode(facade::create_checklist_template(input.title, input.items)?)
        }
        "get_checklist_templates_page" => {
            let input: PageInput = decode(payload)?;
            encode(facade::get_checklist_templates_page(
                input.limit,
                input.offset,
                input.search,
            )?)
        }
        "get_checklist_template_items" => encode(facade::get_checklist_template_items(
            decode::<IdInput>(payload)?.id,
        )?),
        "update_checklist_template" => {
            let input: ChecklistInput = decode(payload)?;
            encode(facade::update_checklist_template(
                required_id(input.id)?,
                input.title,
                input.items,
            )?)
        }
        "delete_checklist_template" => encode(facade::delete_checklist_template(
            decode::<IdInput>(payload)?.id,
        )?),
        "create_full_service_order" => {
            let input: RequestEnvelope<
                crate::commands::service_order_commands::CreateFullServiceOrderRequest,
            > = decode(payload)?;
            encode(facade::create_full_service_order(input.request)?)
        }
        "get_service_order" => encode(facade::get_service_order(decode::<IdInput>(payload)?.id)?),
        "get_service_orders_page" => {
            let input: ServiceOrderPageInput = decode(payload)?;
            encode(facade::get_service_orders_page(
                input.limit,
                input.offset,
                input.search,
                input.status,
                input.user_id,
                input.customer_id,
                input.created_date_from,
                input.created_date_to,
                input.finalized_date_from,
                input.finalized_date_to,
            )?)
        }
        "get_service_orders_by_customer_id" => encode(facade::get_service_orders_by_customer_id(
            decode::<CustomerIdInput>(payload)?.customer_id,
        )?),
        "get_service_order_events" => encode(facade::get_service_order_events(
            decode::<ServiceOrderIdInput>(payload)?.service_order_id,
        )?),
        "get_service_order_parts" => encode(facade::get_service_order_parts(
            decode::<ServiceOrderIdInput>(payload)?.service_order_id,
        )?),
        "transition_service_order_status" => {
            let input: TransitionStatusInput = decode(payload)?;
            encode(facade::transition_service_order_status(
                input.id,
                input.status,
                input.restore_stock,
            )?)
        }
        "save_service_order_edit" => {
            let input: RequestEnvelope<
                crate::commands::service_order_commands::SaveServiceOrderEditRequest,
            > = decode(payload)?;
            encode(facade::save_service_order_edit(input.request)?)
        }
        "delete_service_order" => encode(facade::delete_service_order(
            decode::<IdInput>(payload)?.id,
        )?),
        "add_part_to_service_order" => {
            let input: ServiceOrderPartInput = decode(payload)?;
            encode(facade::add_part_to_service_order(
                input.service_order_id,
                required_inventory_id(input.inventory_item_id)?,
                input.quantity,
            )?)
        }
        "remove_part_from_service_order" => encode(facade::remove_part_from_service_order(
            decode::<PartInput>(payload)?.part_id,
        )?),
        "update_service_order_part_quantity" => {
            let input: PartInput = decode(payload)?;
            encode(facade::update_service_order_part_quantity(
                input.part_id,
                input.quantity,
            )?)
        }
        "save_service_order_checklist" => {
            let input: ServiceOrderChecklistInput = decode(payload)?;
            encode(facade::save_service_order_checklist(
                input.os_id,
                input.items,
            )?)
        }
        "get_service_order_checklist" => encode(facade::get_service_order_checklist(
            decode::<ServiceOrderChecklistInput>(payload)?.os_id,
        )?),
        "upload_service_order_attachment" => {
            let input: AttachmentUploadInput = decode(payload)?;
            encode(facade::upload_service_order_attachment(
                input.service_order_id,
                input.file_name,
                input.data_base64,
            )?)
        }
        "get_service_order_attachments" => encode(facade::get_service_order_attachments(
            decode::<ServiceOrderIdInput>(payload)?.service_order_id,
        )?),
        "read_service_order_attachment" => encode(facade::read_service_order_attachment(
            decode::<IdInput>(payload)?.id,
        )?),
        "delete_service_order_attachment" => encode(facade::delete_service_order_attachment(
            decode::<IdInput>(payload)?.id,
        )?),
        "preview_service_order_pdf" => encode(facade::preview_service_order_pdf(
            decode::<ServiceOrderIdInput>(payload)?.service_order_id,
        )?),
        "download_pdf_preview" => encode(facade::download_pdf_preview(
            decode::<TokenInput>(payload)?.token,
        )?),
        "get_dashboard_data" => encode(facade::get_dashboard_data()?),
        "get_financial_report" => {
            let input: FinancialReportInput = decode(payload)?;
            encode(facade::get_financial_report(
                input.start_date,
                input.end_date,
                input.technician_id,
                input.ranking_metric,
                input.ranking_limit,
            )?)
        }
        "create_remote_backup_download" => encode(facade::create_remote_backup_download(
            decode::<RemoteBackupInput>(payload)?.passphrase,
        )?),
        "restore_backup"
        | "reset_database"
        | "inspect_backup"
        | "validate_backup_passphrase"
        | "update_automatic_backup_settings"
        | "run_automatic_backup_now" => Err(lan_error(
            "This storage operation is available only on the host computer.",
            "Esta operação de armazenamento está disponível apenas no computador host.",
        )),
        _ => Err(lan_error(
            format!("Unknown LAN product operation: {operation}"),
            "A operação solicitada não está disponível pela rede LAN.",
        )),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdInput {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInput {
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomerInput {
    id: Option<String>,
    name: String,
    phone: String,
    email: String,
    address: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInput {
    id: Option<String>,
    name: String,
    email: String,
    phone: Option<String>,
    cpf: Option<String>,
    join_date: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryInput {
    id: Option<String>,
    name: String,
    description: String,
    #[serde(rename = "type")]
    item_type: String,
    min_quantity: i32,
    current_quantity: i32,
    cost_price: i64,
    sale_price: i64,
    supplier_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryPageInput {
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    #[serde(rename = "itemType")]
    item_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryInsightsInput {
    inactive_days: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StockInput {
    id: String,
    quantity: i32,
    unit_cost: Option<i64>,
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChecklistInput {
    id: Option<String>,
    title: String,
    items: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceOrderPageInput {
    limit: Option<u32>,
    offset: Option<u32>,
    search: Option<String>,
    status: Option<String>,
    user_id: Option<String>,
    customer_id: Option<String>,
    created_date_from: Option<String>,
    created_date_to: Option<String>,
    finalized_date_from: Option<String>,
    finalized_date_to: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomerIdInput {
    customer_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceOrderIdInput {
    service_order_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransitionStatusInput {
    id: String,
    status: String,
    restore_stock: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceOrderPartInput {
    service_order_id: String,
    inventory_item_id: Option<String>,
    quantity: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartInput {
    part_id: String,
    #[serde(default)]
    quantity: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServiceOrderChecklistInput {
    os_id: String,
    #[serde(default)]
    items: Vec<crate::models::checklist::ChecklistItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentUploadInput {
    service_order_id: String,
    file_name: String,
    data_base64: String,
}

#[derive(Deserialize)]
struct TokenInput {
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinancialReportInput {
    start_date: Option<String>,
    end_date: Option<String>,
    technician_id: Option<String>,
    ranking_metric: Option<String>,
    ranking_limit: Option<i32>,
}

#[derive(Deserialize)]
struct RemoteBackupInput {
    passphrase: Option<String>,
}

#[derive(Deserialize)]
struct RequestEnvelope<T> {
    request: T,
}

fn required_id(id: Option<String>) -> Result<String, AppError> {
    id.ok_or_else(|| {
        lan_error(
            "Operation requires an id.",
            "A operação exige um identificador.",
        )
    })
}

fn required_inventory_id(id: Option<String>) -> Result<String, AppError> {
    id.ok_or_else(|| {
        lan_error(
            "Operation requires an inventory item id.",
            "A operação exige um identificador de item de inventário.",
        )
    })
}

fn decode<T: for<'de> Deserialize<'de>>(payload: serde_json::Value) -> Result<T, AppError> {
    serde_json::from_value(payload).map_err(|error| {
        lan_error(
            format!("Invalid operation payload: {error}"),
            "Os dados enviados para a operação são inválidos.",
        )
    })
}

fn encode<T: Serialize>(value: T) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(value).map_err(|error| {
        lan_error(
            format!("Failed to serialize operation result: {error}"),
            "Não foi possível preparar o resultado da operação.",
        )
    })
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
    use base64::Engine;
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

        let unauthorized_command = agent
            .post(format!("{base_url}/api/v1/commands/create_customer"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "unauthorized-customer")
            .send(br#"{"name":"Ana","phone":"1","email":"a@b.com","address":"Rua"}"#)
            .unwrap();
        assert_eq!(unauthorized_command.status(), StatusCode::UNAUTHORIZED);

        let customer_payload = br#"{"name":"Cliente LAN","phone":"41999990000","email":"lan@example.com","address":"Rua LAN"}"#;
        let mut customer = agent
            .post(format!("{base_url}/api/v1/commands/create_customer"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "customer-1")
            .send(customer_payload)
            .unwrap();
        let customer_id: String =
            serde_json::from_str(&customer.body_mut().read_to_string().unwrap()).unwrap();
        let mut customer_replay = agent
            .post(format!("{base_url}/api/v1/commands/create_customer"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "customer-1")
            .send(customer_payload)
            .unwrap();
        let replayed_customer_id: String =
            serde_json::from_str(&customer_replay.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(replayed_customer_id, customer_id);

        let user_payload = br#"{"name":"Tecnico LAN","email":"tecnico-lan@example.com","phone":null,"cpf":null,"joinDate":null}"#;
        for _ in 0..2 {
            let response = agent
                .post(format!("{base_url}/api/v1/commands/create_user"))
                .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
                .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
                .header(header::CONTENT_TYPE.as_str(), "application/json")
                .header("x-idempotency-key", "user-1")
                .send(user_payload)
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let inventory_payload = br#"{"name":"Peca LAN","description":"Teste","type":"part","minQuantity":1,"currentQuantity":0,"costPrice":1000,"salePrice":2000,"supplierName":null}"#;
        let mut inventory = agent
            .post(format!("{base_url}/api/v1/commands/create_inventory_item"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "inventory-1")
            .send(inventory_payload)
            .unwrap();
        let inventory_item: serde_json::Value =
            serde_json::from_str(&inventory.body_mut().read_to_string().unwrap()).unwrap();
        let inventory_id = inventory_item["id"].as_str().unwrap();
        let restock_payload = serde_json::to_vec(&json!({
            "id": inventory_id,
            "quantity": 2,
            "unitCost": 1000,
            "reason": "LAN"
        }))
        .unwrap();
        for _ in 0..2 {
            let response = agent
                .post(format!("{base_url}/api/v1/commands/restock_inventory_item"))
                .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
                .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
                .header(header::CONTENT_TYPE.as_str(), "application/json")
                .header("x-idempotency-key", "restock-1")
                .send(restock_payload.as_slice())
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let checklist_payload = br#"{"title":"Entrada LAN","items":["Liga"]}"#;
        for _ in 0..2 {
            let response = agent
                .post(format!(
                    "{base_url}/api/v1/commands/create_checklist_template"
                ))
                .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
                .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
                .header(header::CONTENT_TYPE.as_str(), "application/json")
                .header("x-idempotency-key", "checklist-1")
                .send(checklist_payload)
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let invalid_inventory = agent
            .post(format!("{base_url}/api/v1/commands/create_inventory_item"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "inventory-invalid")
            .send(br#"{"name":"Invalido","description":"","type":"part","minQuantity":-1,"currentQuantity":0,"costPrice":0,"salePrice":0,"supplierName":null}"#)
            .unwrap();
        assert_eq!(invalid_inventory.status(), StatusCode::BAD_REQUEST);

        let conn = crate::database::get_db().unwrap();
        let customer_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM customers WHERE name = 'Cliente LAN'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let user_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE name = 'Tecnico LAN'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let checklist_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM checklist_templates WHERE title = 'Entrada LAN'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let inventory_quantity: i32 = conn
            .query_row(
                "SELECT current_quantity FROM inventory_items WHERE id = ?1",
                [inventory_id],
                |row| row.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(customer_count, 1);
        assert_eq!(user_count, 1);
        assert_eq!(checklist_count, 1);
        assert_eq!(inventory_quantity, 2);

        let order_payload = serde_json::to_vec(&json!({
            "request": {
                "customerAction": { "type": "existing", "id": customer_id, "update": null },
                "userId": null,
                "equipment": "Notebook LAN",
                "imei": null,
                "description": "Criada pela rede",
                "parts": [{ "inventoryItemId": inventory_id, "quantity": 1 }],
                "checklistItems": [{ "label": "Liga", "checked": true }],
                "attachmentToken": null
            }
        }))
        .unwrap();
        let mut created_order = agent
            .post(format!(
                "{base_url}/api/v1/commands/create_full_service_order"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "order-1")
            .send(order_payload.as_slice())
            .unwrap();
        let order_id: String =
            serde_json::from_str(&created_order.body_mut().read_to_string().unwrap()).unwrap();
        let mut replayed_order = agent
            .post(format!(
                "{base_url}/api/v1/commands/create_full_service_order"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "order-1")
            .send(order_payload.as_slice())
            .unwrap();
        let replayed_order_id: String =
            serde_json::from_str(&replayed_order.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(replayed_order_id, order_id);

        let upload_payload = serde_json::to_vec(&json!({
            "serviceOrderId": order_id,
            "fileName": "foto.png",
            "dataBase64": "iVBORw0KGgo="
        }))
        .unwrap();
        let mut uploaded = agent
            .post(format!(
                "{base_url}/api/v1/commands/upload_service_order_attachment"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "attachment-1")
            .send(upload_payload)
            .unwrap();
        let attachment: serde_json::Value =
            serde_json::from_str(&uploaded.body_mut().read_to_string().unwrap()).unwrap();
        let attachment_id = attachment["id"].as_str().unwrap();
        assert_eq!(attachment["fileName"], "foto.png");
        let read_payload = serde_json::to_vec(&json!({ "id": attachment_id })).unwrap();
        let mut read_attachment = agent
            .post(format!(
                "{base_url}/api/v1/commands/read_service_order_attachment"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .send(read_payload)
            .unwrap();
        let attachment_data: String =
            serde_json::from_str(&read_attachment.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(attachment_data, "data:image/png;base64,iVBORw0KGgo=");
        let delete_attachment = agent
            .post(format!(
                "{base_url}/api/v1/commands/delete_service_order_attachment"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "attachment-delete-1")
            .send(serde_json::to_vec(&json!({ "id": attachment_id })).unwrap())
            .unwrap();
        assert_eq!(delete_attachment.status(), StatusCode::OK);

        let transition_payload = serde_json::to_vec(&json!({
            "id": order_id,
            "status": "Em Manutenção",
            "restoreStock": false
        }))
        .unwrap();
        let transition = agent
            .post(format!(
                "{base_url}/api/v1/commands/transition_service_order_status"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .header("x-idempotency-key", "status-1")
            .send(transition_payload)
            .unwrap();
        assert_eq!(transition.status(), StatusCode::OK);

        let conn = crate::database::get_db().unwrap();
        let order_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM service_orders WHERE id = ?1",
                [&order_id],
                |row| row.get(0),
            )
            .unwrap();
        let stock_after_order: i32 = conn
            .query_row(
                "SELECT current_quantity FROM inventory_items WHERE id = ?1",
                [inventory_id],
                |row| row.get(0),
            )
            .unwrap();
        let order_status: String = conn
            .query_row(
                "SELECT status FROM service_orders WHERE id = ?1",
                [&order_id],
                |row| row.get(0),
            )
            .unwrap();
        let attachment_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM service_order_attachments WHERE id = ?1",
                [attachment_id],
                |row| row.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(order_count, 1);
        assert_eq!(stock_after_order, 1);
        assert_eq!(order_status, "Em Manutenção");
        assert_eq!(attachment_count, 0);

        let mut dashboard = agent
            .post(format!("{base_url}/api/v1/commands/get_dashboard_data"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .send(b"{}")
            .unwrap();
        let dashboard_data: serde_json::Value =
            serde_json::from_str(&dashboard.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(dashboard_data["summary"]["activeOrdersCount"], 1);
        assert!(dashboard_data["recentOrders"]
            .as_array()
            .unwrap()
            .iter()
            .any(|order| order["id"] == order_id));

        let mut report = agent
            .post(format!("{base_url}/api/v1/commands/get_financial_report"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .send(br#"{"startDate":null,"endDate":null,"technicianId":null,"rankingMetric":null,"rankingLimit":null}"#)
            .unwrap();
        let report_data: serde_json::Value =
            serde_json::from_str(&report.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(report_data["newOrders"], 1);
        assert_eq!(report_data["finalizedOrders"], 0);

        let mut backup = agent
            .post(format!(
                "{base_url}/api/v1/commands/create_remote_backup_download"
            ))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .send(br#"{"passphrase":"senha-remota-segura"}"#)
            .unwrap();
        let backup_data: serde_json::Value =
            serde_json::from_str(&backup.body_mut().read_to_string().unwrap()).unwrap();
        assert!(backup_data["fileName"]
            .as_str()
            .unwrap()
            .ends_with(".osbkp"));
        assert_eq!(backup_data["attachmentCount"], 0);
        let backup_bytes = base64::engine::general_purpose::STANDARD
            .decode(backup_data["dataBase64"].as_str().unwrap())
            .unwrap();
        assert!(backup_bytes.starts_with(b"OPETBKP2"));

        let mut reset = agent
            .post(format!("{base_url}/api/v1/commands/reset_database"))
            .header(APP_VERSION_HEADER, env!("CARGO_PKG_VERSION"))
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header(header::CONTENT_TYPE.as_str(), "application/json")
            .send(b"{}")
            .unwrap();
        assert_eq!(reset.status(), StatusCode::BAD_REQUEST);
        let reset_error: serde_json::Value =
            serde_json::from_str(&reset.body_mut().read_to_string().unwrap()).unwrap();
        assert!(reset_error["pt"].as_str().unwrap().contains("host"));

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
