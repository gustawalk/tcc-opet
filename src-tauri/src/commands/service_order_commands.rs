use crate::attachment_service;
use crate::commands::attachment_commands::PENDING_ATTACHMENT_SELECTIONS;
use crate::error::AppError;
use crate::models::checklist::ChecklistItem;
use crate::models::service_order::ServiceOrder;
use crate::models::service_order_event::ServiceOrderEvent;
use crate::repositories::checklist_repo::ChecklistRepository;
use crate::repositories::customer_repo::CustomerRepository;
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFullServiceOrderRequest {
    pub customer_action: CustomerAction,
    pub user_id: Option<String>,
    pub equipment: String,
    pub imei: Option<String>,
    pub description: String,
    #[serde(default)]
    pub parts: Vec<CreateServiceOrderPartRequest>,
    #[serde(default)]
    pub checklist_items: Vec<ChecklistItemInput>,
    pub attachment_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum CustomerAction {
    Existing {
        id: String,
        #[serde(default)]
        update: Option<CustomerUpdate>,
    },
    New {
        name: String,
        phone: String,
        email: String,
        address: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerUpdate {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItemInput {
    pub label: String,
    #[serde(default)]
    pub checked: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServiceOrderPartRequest {
    pub inventory_item_id: String,
    pub quantity: i32,
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
pub fn create_full_service_order(
    request: CreateFullServiceOrderRequest,
) -> Result<String, AppError> {
    let conn = crate::database::get_db()?;
    create_full_service_order_with_conn(&conn, request)
}

pub(crate) fn create_full_service_order_with_conn(
    conn: &rusqlite::Connection,
    request: CreateFullServiceOrderRequest,
) -> Result<String, AppError> {
    let tx = conn.unchecked_transaction()?;

    let (customer_id, customer_name) = match request.customer_action {
        CustomerAction::Existing { id, update } => {
            if let Some(u) = update {
                let mut customer = CustomerRepository::get_by_id_with_conn(&tx, &id)?
                    .ok_or_else(|| crate::error::not_found("Customer", "Cliente"))?;
                if let Some(phone) = u.phone {
                    customer.phone = phone;
                }
                if let Some(email) = u.email {
                    customer.email = email;
                }
                if let Some(address) = u.address {
                    customer.address = address;
                }
                CustomerRepository::update_with_conn(&tx, &customer)?;
            }
            let customer = CustomerRepository::get_by_id_with_conn(&tx, &id)?
                .ok_or_else(|| crate::error::not_found("Customer", "Cliente"))?;
            (customer.id.clone(), customer.name.clone())
        }
        CustomerAction::New {
            name,
            phone,
            email,
            address,
        } => {
            let customer = crate::models::customer::Customer::new(name, phone, email, address);
            CustomerRepository::create_with_conn(&tx, &customer)?;
            (customer.id.clone(), customer.name.clone())
        }
    };

    let mut order = ServiceOrder::new(
        customer_id.clone(),
        request.equipment.clone(),
        request.description.clone(),
    );
    order.customer_name = Some(customer_name.clone());
    order.user_id = request.user_id.clone();
    order.imei = request.imei.clone();
    ServiceOrderRepository::create_with_conn(&tx, &mut order)?;
    let order_id = order.id.clone();

    for part in &request.parts {
        if part.quantity <= 0 {
            return Err(crate::error::business_error(
                "Part quantity must be greater than zero.",
                "A quantidade do item deve ser maior que zero.",
            ));
        }

        let (item_type, current_qty, unit_cost, unit_price): (String, i32, f64, f64) = tx
            .query_row(
                "SELECT type, current_quantity,
                        CASE WHEN average_cost > 0 THEN average_cost ELSE cost_price END,
                        sale_price
                 FROM inventory_items WHERE id = ?1 AND deleted_at IS NULL",
                rusqlite::params![part.inventory_item_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    crate::error::not_found("Inventory item", "Item de inventário")
                }
                other => AppError::from(other),
            })?;

        if item_type == "part" && current_qty < part.quantity {
            return Err(crate::error::business_error(
                "Insufficient stock for this service order.",
                "Estoque insuficiente para esta ordem de serviço.",
            ));
        }

        tx.execute(
            "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, quantity, unit_cost, unit_price)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                order_id,
                part.inventory_item_id,
                part.quantity,
                unit_cost,
                unit_price,
            ],
        )?;

        if item_type == "part" {
            let updated = tx.execute(
                "UPDATE inventory_items
                 SET current_quantity = current_quantity - ?1, updated_at = ?2
                 WHERE id = ?3 AND deleted_at IS NULL AND current_quantity >= ?1",
                rusqlite::params![
                    part.quantity,
                    chrono::Utc::now().to_rfc3339(),
                    part.inventory_item_id,
                ],
            )?;
            if updated == 0 {
                return Err(crate::error::business_error(
                    "Insufficient stock.",
                    "Estoque insuficiente.",
                ));
            }

            tx.execute(
                "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, unit_cost, created_at)
                 VALUES (?1, ?2, 'saida', ?3, ?4, 'service_order_add', ?5, ?6)",
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    part.inventory_item_id,
                    part.quantity,
                    order_id,
                    unit_cost,
                    chrono::Utc::now().to_rfc3339(),
                ],
            )?;
        }

        tx.execute(
            "UPDATE service_orders
             SET total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?1), updated_at = ?2
             WHERE id = ?1",
            rusqlite::params![order_id, chrono::Utc::now().to_rfc3339()],
        )?;

        let event = ServiceOrderEvent::new(
            order_id.clone(),
            "item_added".to_string(),
            serde_json::json!({
                "inventoryItemId": part.inventory_item_id,
                "quantity": part.quantity,
                "itemType": item_type,
            })
            .to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(&tx, &event)?;
    }

    if !request.checklist_items.is_empty() {
        let items: Vec<ChecklistItem> = request
            .checklist_items
            .iter()
            .map(|input| ChecklistItem {
                id: uuid::Uuid::new_v4().to_string(),
                label: input.label.clone(),
                checked: input.checked,
            })
            .collect();
        ChecklistRepository::replace_os_checklist_in_transaction(&tx, &order_id, items)?;
    }

    if let Some(token) = &request.attachment_token {
        let paths = PENDING_ATTACHMENT_SELECTIONS
            .lock()
            .map_err(|_| {
                AppError::new(
                    "Pending attachment storage is unavailable.",
                    "O armazenamento temporário de anexos está indisponível.",
                )
            })?
            .remove(token)
            .ok_or_else(|| {
                AppError::new(
                    "Selected attachments are no longer available.",
                    "Os anexos selecionados não estão mais disponíveis.",
                )
            })?;
        for path in &paths {
            attachment_service::add_attachment_with_paths(
                &tx,
                &order_id,
                path,
                &crate::database::attachments_dir(),
            )?;
        }
    }

    tx.commit()?;
    Ok(order_id)
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
    use crate::models::customer::Customer;
    use crate::models::inventory_item::InventoryItem;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::inventory_repo::InventoryRepository;
    use crate::repositories::service_order_repo::ServiceOrderRepository;
    use crate::test_helpers::setup_db;

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

    #[test]
    fn create_full_service_order_creates_customer_os_parts_and_checklist() {
        let conn = setup_db();
        let part = InventoryItem::new(
            "Tela".to_string(),
            "Tela iPhone".to_string(),
            "part".to_string(),
            1,
            10,
            50.0,
            120.0,
        );
        InventoryRepository::create_with_conn(&conn, &part).unwrap();

        let request = CreateFullServiceOrderRequest {
            customer_action: CustomerAction::New {
                name: "João".to_string(),
                phone: "41999999999".to_string(),
                email: "joao@example.com".to_string(),
                address: "Rua A".to_string(),
            },
            user_id: None,
            equipment: "iPhone 14".to_string(),
            imei: Some("123456789".to_string()),
            description: "Troca de tela".to_string(),
            parts: vec![CreateServiceOrderPartRequest {
                inventory_item_id: part.id.clone(),
                quantity: 1,
            }],
            checklist_items: vec![
                ChecklistItemInput {
                    label: "Testar câmera".to_string(),
                    checked: false,
                },
                ChecklistItemInput {
                    label: "Testar touch".to_string(),
                    checked: false,
                },
            ],
            attachment_token: None,
        };

        let order_id = create_full_service_order_with_conn(&conn, request).unwrap();

        let order = ServiceOrderRepository::get_by_id_with_conn(&conn, &order_id)
            .unwrap()
            .unwrap();
        assert_eq!(order.equipment, "iPhone 14");
        assert_eq!(order.description, "Troca de tela");
        assert_eq!(order.status, "Orçamento");
        assert!(order.total_price.unwrap_or(0.0) > 0.0);

        let parts =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order_id).unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].inventory_item_id, part.id);
        assert_eq!(parts[0].quantity, 1);

        let checklist =
            crate::repositories::checklist_repo::ChecklistRepository::get_os_checklist_with_conn(
                &conn, &order_id,
            )
            .unwrap();
        assert_eq!(checklist.len(), 2);
        assert!(checklist.iter().all(|c| !c.checked));
    }

    #[test]
    fn create_full_service_order_rolls_back_on_stock_failure() {
        let conn = setup_db();
        let existing = Customer::new(
            "Carlos".to_string(),
            "41988888888".to_string(),
            "carlos@example.com".to_string(),
            "Rua B".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &existing).unwrap();

        let part = InventoryItem::new(
            "Bateria".to_string(),
            "Bateria iPhone".to_string(),
            "part".to_string(),
            1,
            1,
            30.0,
            80.0,
        );
        InventoryRepository::create_with_conn(&conn, &part).unwrap();
        let customer_id = existing.id.clone();

        let request = CreateFullServiceOrderRequest {
            customer_action: CustomerAction::Existing {
                id: existing.id.clone(),
                update: None,
            },
            user_id: None,
            equipment: "iPhone 14".to_string(),
            imei: None,
            description: "Troca de bateria".to_string(),
            parts: vec![
                CreateServiceOrderPartRequest {
                    inventory_item_id: part.id.clone(),
                    quantity: 1,
                },
                CreateServiceOrderPartRequest {
                    inventory_item_id: part.id.clone(),
                    quantity: 1,
                },
            ],
            checklist_items: vec![],
            attachment_token: None,
        };

        let result = create_full_service_order_with_conn(&conn, request);
        assert!(result.is_err());

        let orders =
            ServiceOrderRepository::get_by_customer_id_with_conn(&conn, &customer_id).unwrap();
        assert_eq!(orders.len(), 0);

        let updated_part = InventoryRepository::get_by_id_with_conn(&conn, &part.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated_part.current_quantity, 1);
    }

    #[test]
    fn create_full_service_order_updates_existing_customer() {
        let conn = setup_db();
        let existing = Customer::new(
            "Maria".to_string(),
            "41977777777".to_string(),
            "maria@old.com".to_string(),
            "Rua Antiga".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &existing).unwrap();

        let service = InventoryItem::new(
            "Reparo".to_string(),
            "Reparo geral".to_string(),
            "service".to_string(),
            0,
            0,
            0.0,
            100.0,
        );
        InventoryRepository::create_with_conn(&conn, &service).unwrap();
        let customer_id = existing.id.clone();

        let request = CreateFullServiceOrderRequest {
            customer_action: CustomerAction::Existing {
                id: existing.id.clone(),
                update: Some(CustomerUpdate {
                    phone: Some("41999999999".to_string()),
                    email: Some("maria@new.com".to_string()),
                    address: Some("Rua Nova".to_string()),
                }),
            },
            user_id: None,
            equipment: "Notebook".to_string(),
            imei: None,
            description: "Reparo".to_string(),
            parts: vec![CreateServiceOrderPartRequest {
                inventory_item_id: service.id.clone(),
                quantity: 1,
            }],
            checklist_items: vec![],
            attachment_token: None,
        };

        create_full_service_order_with_conn(&conn, request).unwrap();

        let updated = CustomerRepository::get_by_id_with_conn(&conn, &customer_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.email, "maria@new.com");
        assert_eq!(updated.phone, "41999999999");
        assert_eq!(updated.address, "Rua Nova");
    }
}
