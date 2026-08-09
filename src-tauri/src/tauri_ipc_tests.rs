use crate::test_helpers::setup_global_backend;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;

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
