use crate::test_helpers::setup_global_backend;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;

#[test]
fn lan_host_api_preserves_the_frontend_ipc_contract() {
    crate::lan_api::tests::run_https_host_api_contract_workflow();
}

fn request(command: &str, body: Value) -> InvokeRequest {
    InvokeRequest {
        cmd: command.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "http://tauri.localhost".parse().unwrap(),
        body: InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

#[test]
fn lan_mode_settings_preserve_camel_case_ipc_contract() {
    let _backend = setup_global_backend();
    let app = register_commands!(mock_builder())
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let initial = get_ipc_response(&webview, request("get_lan_mode_config", json!({})))
        .unwrap()
        .deserialize::<Value>()
        .unwrap();
    assert_eq!(initial["config"]["mode"], "local");
    assert_eq!(initial["config"]["hostPort"], 8743);
    assert_eq!(initial["activeMode"], "local");
    assert_eq!(initial["restartRequired"], false);
    assert_eq!(initial["storageReady"], true);

    let updated = get_ipc_response(
        &webview,
        request(
            "update_lan_mode_config",
            json!({
                "config": {
                    "mode": "client",
                    "hostPort": 8743,
                    "clientUrl": "https://192.168.1.10:8743",
                    "clientDeviceName": "Balcao 2",
                    "clientToken": "device-token",
                    "clientCertificateFingerprint": "sha256:fingerprint"
                }
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(updated["config"]["mode"], "client");
    assert_eq!(updated["config"]["clientDeviceName"], "Balcao 2");
    assert_eq!(updated["restartRequired"], true);

    let persisted = get_ipc_response(&webview, request("get_lan_mode_config", json!({})))
        .unwrap()
        .deserialize::<Value>()
        .unwrap();
    assert_eq!(persisted["config"], updated["config"]);
}

#[test]
fn core_commands_preserve_the_frontend_ipc_contract() {
    let _backend = setup_global_backend();
    let app = register_commands!(mock_builder())
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let customer_id = get_ipc_response(
        &webview,
        request(
            "create_customer",
            json!({
                "name": "Cliente IPC",
                "phone": "41955556666",
                "email": "ipc@example.com",
                "address": "Rua IPC"
            }),
        ),
    )
    .unwrap()
    .deserialize::<String>()
    .unwrap();

    let part = get_ipc_response(
        &webview,
        request(
            "create_inventory_item",
            json!({
                "name": "Peça IPC",
                "description": "Contrato IPC",
                "type": "part",
                "minQuantity": 1,
                "currentQuantity": 0,
                "costPrice": 2000,
                "salePrice": 7000,
                "supplierName": null
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    let part_id = part["id"].as_str().unwrap().to_string();
    assert_eq!(part["currentQuantity"], 0);
    assert!(part["costPrice"].is_i64());

    get_ipc_response(
        &webview,
        request(
            "restock_inventory_item",
            json!({
                "id": &part_id,
                "quantity": 3,
                "unitCost": 2000,
                "reason": "Teste IPC"
            }),
        ),
    )
    .unwrap();

    let order_id = get_ipc_response(
        &webview,
        request(
            "create_full_service_order",
            json!({
                "request": {
                    "customerAction": { "type": "existing", "id": customer_id, "update": null },
                    "userId": null,
                    "equipment": "Equipamento IPC",
                    "imei": null,
                    "description": "Fluxo IPC",
                    "parts": [{ "inventoryItemId": &part_id, "quantity": 1 }],
                    "checklistItems": [],
                    "attachmentToken": null
                }
            }),
        ),
    )
    .unwrap()
    .deserialize::<String>()
    .unwrap();

    for status in ["Em Manutenção", "Finalizada"] {
        let order = get_ipc_response(
            &webview,
            request(
                "transition_service_order_status",
                json!({ "id": &order_id, "status": status, "restoreStock": false }),
            ),
        )
        .unwrap()
        .deserialize::<Value>()
        .unwrap();
        assert_eq!(order["status"], status);
    }

    let dashboard = get_ipc_response(&webview, request("get_dashboard_data", json!({})))
        .unwrap()
        .deserialize::<Value>()
        .unwrap();
    assert_eq!(dashboard["summary"]["totalRevenue"], 7000);
    assert_eq!(dashboard["summary"]["estimatedGrossProfit"], 5000);
    assert!(dashboard["summary"].get("netProfit").is_none());

    let report = get_ipc_response(
        &webview,
        request(
            "get_financial_report",
            json!({
                "startDate": "2000-01-01",
                "endDate": "2100-12-31",
                "technicianId": null,
                "rankingMetric": "quantity",
                "rankingLimit": 10
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(report["totalRevenue"], 7000);
    assert_eq!(report["estimatedGrossProfit"], 5000);
    assert!(report.get("netProfit").is_none());
    assert_eq!(report["finalizedOrders"], 1);
    let top_item = &report["topItems"][0];
    assert_eq!(top_item["inventoryItemId"], part_id);
    assert_eq!(top_item["itemType"], "part");
    assert_eq!(top_item["displayLabel"], "Peça IPC");
    assert!(top_item["key"].as_str().unwrap().contains(&part_id));

    let automatic_backup =
        get_ipc_response(&webview, request("get_automatic_backup_status", json!({})))
            .unwrap()
            .deserialize::<Value>()
            .unwrap();
    assert!(automatic_backup["enabled"].is_boolean());
    assert!(automatic_backup["intervalHours"].is_u64());
    assert!(automatic_backup["running"].is_boolean());

    let updated_automatic_backup = get_ipc_response(
        &webview,
        request(
            "update_automatic_backup_settings",
            json!({
                "settings": {
                    "enabled": false,
                    "destination": null,
                    "intervalHours": 48
                }
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(updated_automatic_backup["enabled"], false);
    assert_eq!(updated_automatic_backup["intervalHours"], 48);

    let automatic_destination = tempfile::tempdir().unwrap();
    let destination = automatic_destination.path().to_string_lossy().to_string();
    let enabled_at = chrono::Utc::now();
    let enabled_automatic_backup = get_ipc_response(
        &webview,
        request(
            "update_automatic_backup_settings",
            json!({
                "settings": {
                    "enabled": true,
                    "destination": destination,
                    "intervalHours": 48
                }
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    let next_backup = chrono::DateTime::parse_from_rfc3339(
        enabled_automatic_backup["nextBackupAt"].as_str().unwrap(),
    )
    .unwrap()
    .with_timezone(&chrono::Utc);
    assert!(next_backup >= enabled_at + chrono::Duration::hours(48));

    drop(automatic_destination);
    let disabled_with_unavailable_destination = get_ipc_response(
        &webview,
        request(
            "update_automatic_backup_settings",
            json!({
                "settings": {
                    "enabled": false,
                    "destination": destination,
                    "intervalHours": 48
                }
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(disabled_with_unavailable_destination["enabled"], false);

    let error = get_ipc_response(
        &webview,
        request(
            "create_inventory_item",
            json!({
                "name": "Inválido",
                "description": "Erro IPC",
                "type": "part",
                "minQuantity": -1,
                "currentQuantity": 0,
                "costPrice": 1000,
                "salePrice": 2000,
                "supplierName": null
            }),
        ),
    )
    .unwrap_err();
    assert!(error["en"].as_str().unwrap().contains("cannot be negative"));
    assert!(error["pt"]
        .as_str()
        .unwrap()
        .contains("não podem ser negativos"));
}

#[test]
fn paginated_list_commands_return_items_and_total() {
    let _backend = setup_global_backend();
    let app = register_commands!(mock_builder())
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let orders_page = get_ipc_response(
        &webview,
        request(
            "get_service_orders_page",
            json!({ "limit": 1, "offset": 0 }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(orders_page["items"].is_array());
    assert!(orders_page["total"].is_i64());

    let inventory_page = get_ipc_response(
        &webview,
        request(
            "get_inventory_items_page",
            json!({ "limit": 1, "offset": 0 }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(inventory_page["items"].is_array());
    assert!(inventory_page["total"].is_i64());

    let customer_id = get_ipc_response(
        &webview,
        request(
            "create_customer",
            json!({
                "name": "Cliente Busca",
                "phone": "41955557777",
                "email": "busca@example.com",
                "address": "Rua Busca"
            }),
        ),
    )
    .unwrap()
    .deserialize::<String>()
    .unwrap();

    let part = get_ipc_response(
        &webview,
        request(
            "create_inventory_item",
            json!({
                "name": "Peça FiltroBusca",
                "description": "Usada para validar o LIKE no estoque",
                "type": "part",
                "minQuantity": 1,
                "currentQuantity": 0,
                "costPrice": 2000,
                "salePrice": 7000,
                "supplierName": "Fornecedor FiltroBusca"
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    let part_id = part["id"].as_str().unwrap().to_string();

    get_ipc_response(
        &webview,
        request(
            "restock_inventory_item",
            json!({
                "id": &part_id,
                "quantity": 3,
                "unitCost": 2000,
                "reason": "Teste busca paginada"
            }),
        ),
    )
    .unwrap();

    let order_id = get_ipc_response(
        &webview,
        request(
            "create_full_service_order",
            json!({
                "request": {
                    "customerAction": { "type": "existing", "id": customer_id, "update": null },
                    "userId": null,
                    "equipment": "Equipamento FiltroBusca",
                    "imei": null,
                    "description": "OS de busca paginada",
                    "parts": [{ "inventoryItemId": &part_id, "quantity": 1 }],
                    "checklistItems": [],
                    "attachmentToken": null
                }
            }),
        ),
    )
    .unwrap()
    .deserialize::<String>()
    .unwrap();
    assert!(!order_id.is_empty());

    let searched_orders = get_ipc_response(
        &webview,
        request(
            "get_service_orders_page",
            json!({ "limit": 10, "offset": 0, "search": "FiltroBusca" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(searched_orders["total"].as_i64().unwrap() >= 1);
    for item in searched_orders["items"].as_array().unwrap() {
        let haystack = format!(
            "{} {} {} {}",
            item["customerName"].as_str().unwrap_or(""),
            item["equipment"].as_str().unwrap_or(""),
            item["description"].as_str().unwrap_or(""),
            item["displayId"].as_str().unwrap_or(""),
        );
        assert!(haystack.to_lowercase().contains("filtrobusca"));
    }

    let created_orders = get_ipc_response(
        &webview,
        request(
            "get_service_orders_page",
            json!({
                "limit": 10,
                "offset": 0,
                "createdDateFrom": "2000-01-01",
                "createdDateTo": "2100-12-31"
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(created_orders["total"].as_i64().unwrap() >= 1);

    get_ipc_response(
        &webview,
        request(
            "transition_service_order_status",
            json!({ "id": &order_id, "status": "Em Manutenção", "restoreStock": false }),
        ),
    )
    .unwrap();
    get_ipc_response(
        &webview,
        request(
            "transition_service_order_status",
            json!({ "id": &order_id, "status": "Finalizada", "restoreStock": false }),
        ),
    )
    .unwrap();
    let finalized_orders = get_ipc_response(
        &webview,
        request(
            "get_service_orders_page",
            json!({
                "limit": 10,
                "offset": 0,
                "finalizedDateFrom": "2000-01-01",
                "finalizedDateTo": "2100-12-31"
            }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(finalized_orders["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == order_id));

    let searched_items = get_ipc_response(
        &webview,
        request(
            "get_inventory_items_page",
            json!({ "limit": 10, "offset": 0, "search": "FiltroBusca" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(searched_items["total"].as_i64().unwrap() >= 1);
    for item in searched_items["items"].as_array().unwrap() {
        let haystack = format!(
            "{} {} {}",
            item["name"].as_str().unwrap_or(""),
            item["description"].as_str().unwrap_or(""),
            item["supplierName"].as_str().unwrap_or(""),
        );
        assert!(haystack.to_lowercase().contains("filtrobusca"));
    }

    let no_match = get_ipc_response(
        &webview,
        request(
            "get_inventory_items_page",
            json!({ "limit": 10, "offset": 0, "search": "zzzz-inexistente" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(no_match["total"], 0);
    assert!(no_match["items"].as_array().unwrap().is_empty());

    let customers_page = get_ipc_response(
        &webview,
        request(
            "get_customers_page",
            json!({ "limit": 10, "offset": 0, "search": "Cliente" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(customers_page["total"].as_i64().unwrap() >= 1);
    assert!(customers_page["items"].is_array());

    let users_page = get_ipc_response(
        &webview,
        request("get_users_page", json!({ "limit": 10, "offset": 0 })),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(users_page["total"].is_i64());
    assert!(users_page["items"].is_array());

    get_ipc_response(
        &webview,
        request(
            "create_checklist_template",
            json!({ "title": "Template FiltroBusca", "items": ["Tela"] }),
        ),
    )
    .unwrap();

    let templates_page = get_ipc_response(
        &webview,
        request(
            "get_checklist_templates_page",
            json!({ "limit": 10, "offset": 0, "search": "FiltroBusca" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert!(templates_page["total"].as_i64().unwrap() >= 1);

    let parts_only = get_ipc_response(
        &webview,
        request(
            "get_inventory_items_page",
            json!({ "limit": 10, "offset": 0, "itemType": "part" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    for item in parts_only["items"].as_array().unwrap() {
        assert_eq!(item["type"], "part");
    }

    let cancelled_orders = get_ipc_response(
        &webview,
        request(
            "get_service_orders_page",
            json!({ "limit": 10, "offset": 0, "status": "Cancelada" }),
        ),
    )
    .unwrap()
    .deserialize::<Value>()
    .unwrap();
    assert_eq!(cancelled_orders["total"], 0);

    let summary = get_ipc_response(&webview, request("get_inventory_summary", json!({})))
        .unwrap()
        .deserialize::<Value>()
        .unwrap();
    assert!(summary["lowStock"].is_i64());
    assert!(summary["outOfStock"].is_i64());
    assert!(summary["totalStockValue"].is_i64());
}
