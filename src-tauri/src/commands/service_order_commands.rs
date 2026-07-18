use crate::error::AppError;
use crate::models::service_order::ServiceOrder;
use crate::repositories::service_order_repo::{ServiceOrderRepository, ServiceOrderPart};
use tauri::command;

fn require_existing_service_order(order: Option<ServiceOrder>) -> Result<ServiceOrder, AppError> {
    order.ok_or_else(|| crate::error::not_found("Service order", "Ordem de serviço"))
}

#[command]
pub fn get_service_order_parts(service_order_id: String) -> Result<Vec<ServiceOrderPart>, AppError> {
    Ok(ServiceOrderRepository::get_service_order_parts(&service_order_id)?)
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
) -> Result<String, AppError> {
    let mut order = ServiceOrder::new(customer_id, equipment, description);
    order.customer_name = customer_name;
    order.user_id = user_id;
    order.imei = imei;
    order.discount_percent = discount_percent.unwrap_or(0.0);

    ServiceOrderRepository::create(&mut order)?;
    Ok(order.id)
}

#[command]
pub fn get_service_order(id: String) -> Result<Option<ServiceOrder>, AppError> {
    Ok(ServiceOrderRepository::get_by_id(&id)?)
}

#[command]
pub fn get_service_orders() -> Result<Vec<ServiceOrder>, AppError> {
    Ok(ServiceOrderRepository::get_all()?)
}

#[command]
pub fn get_service_orders_by_customer_id(customer_id: String) -> Result<Vec<ServiceOrder>, AppError> {
    Ok(ServiceOrderRepository::get_by_customer_id(&customer_id)?)
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
) -> Result<(), AppError> {
    let mut order = require_existing_service_order(ServiceOrderRepository::get_by_id(&id)?)?;

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

    Ok(ServiceOrderRepository::update(&order)?)
}

#[command]
pub fn delete_service_order(id: String) -> Result<(), AppError> {
    Ok(ServiceOrderRepository::delete(&id)?)
}

#[command]
pub fn add_part_to_service_order(
    service_order_id: String,
    inventory_item_id: String,
    quantity: i32,
) -> Result<(), AppError> {
    Ok(ServiceOrderRepository::add_part_to_service_order(
        &service_order_id,
        &inventory_item_id,
        quantity,
    )?)
}

#[command]
pub fn remove_part_from_service_order(part_id: String) -> Result<(), AppError> {
    Ok(ServiceOrderRepository::remove_part_from_service_order(&part_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_existing_service_order_returns_not_found_error() {
        let err = require_existing_service_order(None).unwrap_err();

        assert_eq!(err.en, "Service order not found.");
        assert_eq!(err.pt, "Ordem de serviço não encontrado(a).");
    }
}
