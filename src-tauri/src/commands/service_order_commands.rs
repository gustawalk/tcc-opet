use crate::models::service_order::ServiceOrder;
use crate::repositories::service_order_repo::{ServiceOrderRepository, ServiceOrderPart};
use tauri::command;

#[command]
pub fn get_service_order_parts(service_order_id: String) -> Result<Vec<ServiceOrderPart>, String> {
    ServiceOrderRepository::get_service_order_parts(&service_order_id).map_err(|e| e.to_string())
}

#[command]
pub fn create_service_order(
    customer_id: String,
    customer_name: Option<String>,
    user_id: Option<String>,
    equipment: String,
    imei: Option<String>,
    description: String,
    discount_percent: Option<f64>,
) -> Result<String, String> {
    let mut order = ServiceOrder::new(customer_id, equipment, description);
    order.customer_name = customer_name;
    order.user_id = user_id;
    order.imei = imei;
    order.discount_percent = discount_percent.unwrap_or(0.0);
    
    ServiceOrderRepository::create(&mut order).map_err(|e: rusqlite::Error| e.to_string())?;
    Ok(order.id)
}

#[command]
pub fn get_service_order(id: String) -> Result<Option<ServiceOrder>, String> {
    ServiceOrderRepository::get_by_id(&id).map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn get_service_orders() -> Result<Vec<ServiceOrder>, String> {
    ServiceOrderRepository::get_all().map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn get_service_orders_by_customer_id(customer_id: String) -> Result<Vec<ServiceOrder>, String> {
    ServiceOrderRepository::get_by_customer_id(&customer_id).map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
#[allow(clippy::too_many_arguments)]
pub fn update_service_order(
    id: String,
    customer_id: String,
    customer_name: Option<String>,
    user_id: Option<String>,
    equipment: String,
    imei: Option<String>,
    description: String,
    status: String,
    total_price: Option<f64>,
    signature_path: Option<String>,
    closed_at: Option<String>,
    discount_percent: Option<f64>,
) -> Result<(), String> {
    let mut order = ServiceOrderRepository::get_by_id(&id)
        .map_err(|e: rusqlite::Error| e.to_string())?
        .ok_or_else(|| "Service order not found".to_string())?;

    order.customer_id = customer_id;
    order.customer_name = customer_name;
    order.user_id = user_id;
    order.equipment = equipment;
    order.imei = imei;
    order.description = description;
    order.status = status;
    order.total_price = total_price;
    order.signature_path = signature_path;
    order.closed_at = closed_at;
    order.discount_percent = discount_percent.unwrap_or(0.0);
    order.updated_at = Some(chrono::Utc::now().to_rfc3339());

    ServiceOrderRepository::update(&order).map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn delete_service_order(id: String) -> Result<(), String> {
    ServiceOrderRepository::delete(&id).map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn add_part_to_service_order(
    service_order_id: String,
    inventory_item_id: String,
    quantity: i32,
) -> Result<(), String> {
    ServiceOrderRepository::add_part_to_service_order(
        &service_order_id,
        &inventory_item_id,
        quantity,
    )
    .map_err(|e: rusqlite::Error| e.to_string())
}

#[command]
pub fn remove_part_from_service_order(part_id: String) -> Result<(), String> {
    ServiceOrderRepository::remove_part_from_service_order(&part_id)
        .map_err(|e: rusqlite::Error| e.to_string())
}
