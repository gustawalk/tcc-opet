use crate::error::AppError;
use crate::models::checklist::ChecklistItem;
use crate::models::service_order::ServiceOrder;
use crate::models::service_order_event::ServiceOrderEvent;
use crate::repositories::inventory_repo::InventoryRepository;
use crate::repositories::service_order_event_repo::ServiceOrderEventRepository;
use crate::repositories::service_order_repo::{ServiceOrderPart, ServiceOrderRepository};
use serde::Deserialize;
use tauri::command;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveServiceOrderEditRequest {
    id: String,
    description: String,
    discount_percent: f64,
    status: String,
    restore_stock: bool,
    checklist: Vec<ChecklistItem>,
}

fn require_existing_service_order(order: Option<ServiceOrder>) -> Result<ServiceOrder, AppError> {
    order.ok_or_else(|| crate::error::not_found("Service order", "Ordem de serviço"))
}

#[command]
pub fn get_service_order_parts(
    service_order_id: String,
) -> Result<Vec<ServiceOrderPart>, AppError> {
    Ok(ServiceOrderRepository::get_service_order_parts(
        &service_order_id,
    )?)
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
pub fn get_service_orders_by_customer_id(
    customer_id: String,
) -> Result<Vec<ServiceOrder>, AppError> {
    Ok(ServiceOrderRepository::get_by_customer_id(&customer_id)?)
}

#[command]
pub fn get_service_order_events(
    service_order_id: String,
) -> Result<Vec<ServiceOrderEvent>, AppError> {
    Ok(ServiceOrderEventRepository::get_by_service_order_id(
        &service_order_id,
    )?)
}

#[command]
pub fn transition_service_order_status(
    id: String,
    status: String,
    restore_stock: bool,
) -> Result<ServiceOrder, AppError> {
    Ok(ServiceOrderRepository::transition_status(
        &id,
        &status,
        restore_stock,
    )?)
}

#[command]
pub fn save_service_order_edit(request: SaveServiceOrderEditRequest) -> Result<(), AppError> {
    ServiceOrderRepository::save_edit(
        &request.id,
        &request.description,
        request.discount_percent,
        &request.status,
        request.restore_stock,
        request.checklist,
    )
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
    closed_at: Option<String>,
    discount_percent: Option<f64>,
) -> Result<(), AppError> {
    let mut order = require_existing_service_order(ServiceOrderRepository::get_by_id(&id)?)?;
    let _ = (status, total_price, closed_at);

    order.customer_id = customer_id;
    order.customer_name = customer_name;
    order.user_id = user_id;
    order.equipment = equipment;
    order.imei = imei;
    order.description = description;
    // Status and closing timestamps are controlled by the lifecycle command.
    // Totals are always recalculated from the persisted OS line items.
    order.total_price = None;
    order.discount_percent = discount_percent.unwrap_or(0.0);
    order.updated_at = Some(chrono::Utc::now().to_rfc3339());

    Ok(ServiceOrderRepository::update(&order)?)
}

#[command]
pub fn delete_service_order(id: String) -> Result<(), AppError> {
    let order = require_existing_service_order(ServiceOrderRepository::get_by_id(&id)?)?;
    if order.status != "Cancelada" {
        return Err(crate::error::business_error(
            "Cancel the service order before deleting it.",
            "Cancele a ordem de serviço antes de excluí-la.",
        ));
    }
    Ok(ServiceOrderRepository::delete(&id)?)
}

#[command]
pub fn add_part_to_service_order(
    service_order_id: String,
    inventory_item_id: String,
    quantity: i32,
) -> Result<(), AppError> {
    if quantity <= 0 {
        return Err(crate::error::business_error(
            "Part quantity must be greater than zero.",
            "A quantidade do item deve ser maior que zero.",
        ));
    }

    let order =
        require_existing_service_order(ServiceOrderRepository::get_by_id(&service_order_id)?)?;
    if matches!(order.status.as_str(), "Finalizada" | "Cancelada") {
        return Err(crate::error::business_error(
            "Items cannot be changed on a finalized or cancelled service order.",
            "Não é possível alterar itens de uma ordem finalizada ou cancelada.",
        ));
    }

    let item = InventoryRepository::get_by_id(&inventory_item_id)?
        .ok_or_else(|| crate::error::not_found("Inventory item", "Item de inventário"))?;
    if item.r#type == "part" && item.current_quantity < quantity {
        return Err(crate::error::business_error(
            "Insufficient stock for this service order.",
            "Estoque insuficiente para esta ordem de serviço.",
        ));
    }

    Ok(ServiceOrderRepository::add_part_to_service_order(
        &service_order_id,
        &inventory_item_id,
        quantity,
    )?)
}

#[command]
pub fn remove_part_from_service_order(part_id: String) -> Result<(), AppError> {
    Ok(ServiceOrderRepository::remove_part_from_service_order(
        &part_id,
    )?)
}

#[command]
pub fn update_service_order_part_quantity(part_id: String, quantity: i32) -> Result<(), AppError> {
    ServiceOrderRepository::update_part_quantity(&part_id, quantity)
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

    #[test]
    fn delete_requires_cancelled_status_message() {
        let err = crate::error::business_error(
            "Cancel the service order before deleting it.",
            "Cancele a ordem de serviço antes de excluí-la.",
        );

        assert_eq!(err.pt, "Cancele a ordem de serviço antes de excluí-la.");
    }
}
