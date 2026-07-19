use crate::database::get_db;
use crate::error::{business_error, not_found, AppError};
use crate::models::checklist::ChecklistItem;
use crate::models::service_order::ServiceOrder;
use crate::models::service_order_event::ServiceOrderEvent;
use crate::repositories::checklist_repo::ChecklistRepository;
use crate::repositories::service_order_event_repo::ServiceOrderEventRepository;
use chrono::Utc;
use rusqlite::{params, Connection, Result, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOrderPart {
    pub id: String,
    pub service_order_id: String,
    pub inventory_item_id: String,
    pub inventory_item_name: String,
    pub item_type: String,
    pub current_quantity: i32,
    pub quantity: i32,
    pub unit_cost: f64,
    pub unit_price: f64,
}

pub struct ServiceOrderRepository;

impl ServiceOrderRepository {
    pub fn get_service_order_parts(service_order_id: &str) -> Result<Vec<ServiceOrderPart>> {
        let conn = get_db()?;
        Self::get_service_order_parts_with_conn(&conn, service_order_id)
    }

    pub(crate) fn get_service_order_parts_with_conn(
        conn: &Connection,
        service_order_id: &str,
    ) -> Result<Vec<ServiceOrderPart>> {
        let mut stmt = conn.prepare(
            "SELECT sop.id, sop.service_order_id, sop.inventory_item_id, ii.name as inventory_item_name, ii.type, ii.current_quantity, sop.quantity, sop.unit_cost, sop.unit_price
             FROM service_order_parts sop
             JOIN inventory_items ii ON sop.inventory_item_id = ii.id
             WHERE sop.service_order_id = ?1"
        )?;

        let rows = stmt.query_map(params![service_order_id], |row| {
            Ok(ServiceOrderPart {
                id: row.get(0)?,
                service_order_id: row.get(1)?,
                inventory_item_id: row.get(2)?,
                inventory_item_name: row.get(3)?,
                item_type: row.get(4)?,
                current_quantity: row.get(5)?,
                quantity: row.get(6)?,
                unit_cost: row.get(7)?,
                unit_price: row.get(8)?,
            })
        })?;

        let mut parts = Vec::new();
        for row in rows {
            parts.push(row?);
        }
        Ok(parts)
    }

    /// Add a part to a service order and update inventory and OS total price
    pub fn add_part_to_service_order(
        service_order_id: &str,
        inventory_item_id: &str,
        quantity: i32,
    ) -> Result<()> {
        let mut conn = get_db()?;
        Self::add_part_to_service_order_with_conn(
            &mut conn,
            service_order_id,
            inventory_item_id,
            quantity,
        )
    }

    pub(crate) fn add_part_to_service_order_with_conn(
        conn: &mut Connection,
        service_order_id: &str,
        inventory_item_id: &str,
        quantity: i32,
    ) -> Result<()> {
        // Start a transaction to ensure consistency
        let transaction = conn.transaction()?;

        let status: String = transaction.query_row(
            "SELECT status FROM service_orders WHERE id = ?1 AND deleted_at IS NULL",
            params![service_order_id],
            |row| row.get(0),
        )?;
        if matches!(status.as_str(), "Finalizada" | "Cancelada") {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let (item_type, current_quantity, unit_cost, unit_price) = {
            // Check the active catalog item and snapshot its prices for this OS.
            let mut stmt = transaction.prepare(
                "SELECT type, current_quantity,
                            CASE WHEN average_cost > 0 THEN average_cost ELSE cost_price END,
                            sale_price
                     FROM inventory_items WHERE id = ?1 AND deleted_at IS NULL",
            )?;
            let mut rows = stmt.query_map(params![inventory_item_id], |row: &rusqlite::Row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            })?;

            let qty_cost_price = match rows.next() {
                Some(Ok(v)) => v,
                Some(Err(e)) => return Err(e),
                None => return Err(rusqlite::Error::QueryReturnedNoRows),
            };
            qty_cost_price
        };

        if quantity <= 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }

        if item_type == "part" && current_quantity < quantity {
            return Err(rusqlite::Error::InvalidQuery);
        }

        // 2. Record the part usage
        transaction.execute(
            "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, quantity, unit_cost, unit_price) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                Uuid::new_v4().to_string(),
                service_order_id,
                inventory_item_id,
                quantity,
                unit_cost,
                unit_price
            ],
        )?;

        // Services are billable catalog entries but do not consume physical stock.
        if item_type == "part" {
            let updated = transaction.execute(
                "UPDATE inventory_items
                 SET current_quantity = current_quantity - ?1, updated_at = ?2
                 WHERE id = ?3 AND deleted_at IS NULL AND current_quantity >= ?1",
                params![quantity, Utc::now().to_rfc3339(), inventory_item_id],
            )?;
            if updated == 0 {
                return Err(rusqlite::Error::InvalidQuery);
            }

            transaction.execute(
                "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, unit_cost, created_at)
                 VALUES (?1, ?2, 'saida', ?3, ?4, 'service_order_add', ?5, ?6)",
                params![
                    Uuid::new_v4().to_string(),
                    inventory_item_id,
                    quantity,
                    service_order_id,
                    unit_cost,
                    Utc::now().to_rfc3339(),
                ],
            )?;
        }

        // 5. Update the Service Order total price
        transaction.execute(
            "UPDATE service_orders 
             SET total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?1), updated_at = ?2
             WHERE id = ?1",
            params![service_order_id, Utc::now().to_rfc3339()],
        )?;
        let event = ServiceOrderEvent::new(
            service_order_id.to_string(),
            "item_added".to_string(),
            serde_json::json!({
                "inventoryItemId": inventory_item_id,
                "quantity": quantity,
                "itemType": item_type,
            })
            .to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(&transaction, &event)?;

        transaction.commit()?;
        Ok(())
    }

    pub fn remove_part_from_service_order(part_id: &str) -> Result<()> {
        let mut conn = get_db()?;
        Self::remove_part_from_service_order_with_conn(&mut conn, part_id)
    }

    pub(crate) fn remove_part_from_service_order_with_conn(
        conn: &mut Connection,
        part_id: &str,
    ) -> Result<()> {
        let transaction = conn.transaction()?;

        let (os_id, inventory_item_id, quantity, item_type, status) = {
            let mut stmt = transaction.prepare(
                "SELECT sop.service_order_id, sop.inventory_item_id, sop.quantity, ii.type, so.status
                 FROM service_order_parts sop
                 JOIN inventory_items ii ON sop.inventory_item_id = ii.id
                 JOIN service_orders so ON sop.service_order_id = so.id
                 WHERE sop.id = ?1"
            )?;
            stmt.query_row(params![part_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
        };

        if matches!(status.as_str(), "Finalizada" | "Cancelada") {
            return Err(rusqlite::Error::InvalidQuery);
        }

        if item_type == "part" {
            transaction.execute(
                "UPDATE inventory_items
                 SET current_quantity = current_quantity + ?1, updated_at = ?2
                 WHERE id = ?3",
                params![quantity, Utc::now().to_rfc3339(), inventory_item_id],
            )?;

            transaction.execute(
                "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, created_at)
                 VALUES (?1, ?2, 'entrada', ?3, ?4, 'service_order_remove', ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    inventory_item_id,
                    quantity,
                    os_id,
                    Utc::now().to_rfc3339(),
                ],
            )?;
        }

        // 3. Delete the part record
        transaction.execute(
            "DELETE FROM service_order_parts WHERE id = ?1",
            params![part_id],
        )?;

        // 3. Recalculate OS total
        transaction.execute(
            "UPDATE service_orders 
             SET total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?1), updated_at = ?2
             WHERE id = ?1",
            params![os_id, Utc::now().to_rfc3339()],
        )?;
        let event = ServiceOrderEvent::new(
            os_id.clone(),
            "item_removed".to_string(),
            serde_json::json!({
                "inventoryItemId": inventory_item_id,
                "quantity": quantity,
                "itemType": item_type,
            })
            .to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(&transaction, &event)?;

        transaction.commit()?;
        Ok(())
    }

    pub fn update_part_quantity(part_id: &str, quantity: i32) -> std::result::Result<(), AppError> {
        let conn = get_db()?;
        Self::update_part_quantity_with_conn(&conn, part_id, quantity)
    }

    pub(crate) fn update_part_quantity_with_conn(
        conn: &Connection,
        part_id: &str,
        quantity: i32,
    ) -> std::result::Result<(), AppError> {
        if quantity <= 0 {
            return Err(business_error(
                "Part quantity must be greater than zero.",
                "A quantidade do item deve ser maior que zero.",
            ));
        }

        let transaction = conn.unchecked_transaction()?;
        let (
            service_order_id,
            inventory_item_id,
            previous_quantity,
            unit_cost,
            item_type,
            stock,
            status,
        ): (String, String, i32, f64, String, i32, String) = transaction
            .query_row(
                "SELECT sop.service_order_id, sop.inventory_item_id, sop.quantity, sop.unit_cost,
                        ii.type, ii.current_quantity, so.status
                 FROM service_order_parts sop
                 JOIN inventory_items ii ON ii.id = sop.inventory_item_id
                 JOIN service_orders so ON so.id = sop.service_order_id
                 WHERE sop.id = ?1 AND so.deleted_at IS NULL",
                params![part_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    not_found("Service order item", "Item da ordem de serviço")
                }
                other => other.into(),
            })?;

        if matches!(status.as_str(), "Finalizada" | "Cancelada") {
            return Err(business_error(
                "Items cannot be changed on a finalized or cancelled service order.",
                "Não é possível alterar itens de uma ordem finalizada ou cancelada.",
            ));
        }

        let quantity_difference = quantity - previous_quantity;
        if quantity_difference == 0 {
            transaction.commit()?;
            return Ok(());
        }

        if item_type == "part" {
            if quantity_difference > stock {
                return Err(business_error(
                    "Insufficient stock for this service order.",
                    "Estoque insuficiente para esta ordem de serviço.",
                ));
            }

            let updated = transaction.execute(
                "UPDATE inventory_items
                 SET current_quantity = current_quantity - ?1, updated_at = ?2
                 WHERE id = ?3 AND current_quantity >= ?1",
                params![
                    quantity_difference,
                    Utc::now().to_rfc3339(),
                    inventory_item_id,
                ],
            )?;
            if updated == 0 {
                return Err(business_error(
                    "Insufficient stock for this service order.",
                    "Estoque insuficiente para esta ordem de serviço.",
                ));
            }

            transaction.execute(
                "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, unit_cost, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'service_order_quantity_update', ?6, ?7)",
                params![
                    Uuid::new_v4().to_string(),
                    inventory_item_id,
                    if quantity_difference > 0 { "saida" } else { "entrada" },
                    quantity_difference.abs(),
                    service_order_id,
                    unit_cost,
                    Utc::now().to_rfc3339(),
                ],
            )?;
        }

        transaction.execute(
            "UPDATE service_order_parts SET quantity = ?1 WHERE id = ?2",
            params![quantity, part_id],
        )?;
        transaction.execute(
            "UPDATE service_orders
             SET total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?1),
                 updated_at = ?2
             WHERE id = ?1",
            params![service_order_id, Utc::now().to_rfc3339()],
        )?;
        let event = ServiceOrderEvent::new(
            service_order_id,
            "item_updated".to_string(),
            serde_json::json!({
                "previousQuantity": previous_quantity,
                "quantity": quantity,
                "itemType": item_type,
            })
            .to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(&transaction, &event)?;
        transaction.commit()?;
        Ok(())
    }

    fn next_display_id(conn: &Connection) -> Result<String> {
        let next_num: i32 = conn.query_row(
            "INSERT INTO service_order_sequences (name, value)
             VALUES ('service_order', 1)
             ON CONFLICT(name) DO UPDATE SET value = service_order_sequences.value + 1
             RETURNING value",
            [],
            |row| row.get(0),
        )?;
        Ok(format!("OS-{:06}", next_num))
    }

    pub fn create(order: &mut ServiceOrder) -> Result<()> {
        let conn = get_db()?;
        Self::create_with_conn(&conn, order)
    }

    pub(crate) fn create_with_conn(conn: &Connection, order: &mut ServiceOrder) -> Result<()> {
        order.display_id = Self::next_display_id(conn)?;

        conn.execute(
            "INSERT INTO service_orders (id, customer_id, customer_name, user_id, equipment, imei, description, status, total_price, created_at, updated_at, closed_at, display_id, discount_percent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                order.id,
                order.customer_id,
                order.customer_name,
                order.user_id,
                order.equipment,
                order.imei,
                order.description,
                order.status,
                order.total_price,
                order.created_at,
                order.updated_at,
                order.closed_at,
                order.display_id,
                order.discount_percent
            ],
        )?;
        let event = ServiceOrderEvent::new(
            order.id.clone(),
            "created".to_string(),
            serde_json::json!({ "status": order.status }).to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(conn, &event)?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<ServiceOrder>> {
        let conn = get_db()?;
        Self::get_by_id_with_conn(&conn, id)
    }

    pub(crate) fn get_by_id_with_conn(conn: &Connection, id: &str) -> Result<Option<ServiceOrder>> {
        let mut stmt = conn.prepare(
            "SELECT so.id, so.customer_id, COALESCE(so.customer_name, c.name) as customer_name, so.user_id, so.equipment, so.imei, so.description, so.status, so.total_price, so.created_at, so.updated_at, so.closed_at, so.display_id, so.discount_percent, users.name as user_name
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             LEFT JOIN users ON so.user_id = users.id
              WHERE so.id = ?1 AND so.deleted_at IS NULL"
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                user_id: row.get(3)?,
                user_name: row.get(14)?,
                equipment: row.get(4)?,
                imei: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                total_price: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                closed_at: row.get(11)?,
                display_id: row.get(12)?,
                discount_percent: row.get(13)?,
            })
        })?;

        let order = rows.next().transpose()?;
        Ok(order)
    }

    pub fn get_all() -> Result<Vec<ServiceOrder>> {
        let conn = get_db()?;
        Self::get_all_with_conn(&conn)
    }

    pub(crate) fn get_all_with_conn(conn: &Connection) -> Result<Vec<ServiceOrder>> {
        let mut stmt = conn.prepare(
            "SELECT so.id, so.customer_id, COALESCE(so.customer_name, c.name) as customer_name, so.user_id, so.equipment, so.imei, so.description, so.status, so.total_price, so.created_at, so.updated_at, so.closed_at, so.display_id, so.discount_percent, users.name as user_name
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             LEFT JOIN users ON so.user_id = users.id
             WHERE so.deleted_at IS NULL
             ORDER BY so.created_at DESC"
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                user_id: row.get(3)?,
                user_name: row.get(14)?,
                equipment: row.get(4)?,
                imei: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                total_price: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                closed_at: row.get(11)?,
                display_id: row.get(12)?,
                discount_percent: row.get(13)?,
            })
        })?;

        let mut orders = Vec::new();
        for row in rows {
            orders.push(row?);
        }
        Ok(orders)
    }

    pub fn get_by_customer_id(customer_id: &str) -> Result<Vec<ServiceOrder>> {
        let conn = get_db()?;
        Self::get_by_customer_id_with_conn(&conn, customer_id)
    }

    pub(crate) fn get_by_customer_id_with_conn(
        conn: &Connection,
        customer_id: &str,
    ) -> Result<Vec<ServiceOrder>> {
        let mut stmt = conn.prepare(
            "SELECT so.id, so.customer_id, COALESCE(so.customer_name, c.name) as customer_name, so.user_id, so.equipment, so.imei, so.description, so.status, so.total_price, so.created_at, so.updated_at, so.closed_at, so.display_id, so.discount_percent, users.name as user_name
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             LEFT JOIN users ON so.user_id = users.id
             WHERE so.customer_id = ?1 AND so.deleted_at IS NULL"
        )?;
        let rows = stmt.query_map(params![customer_id], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                user_id: row.get(3)?,
                user_name: row.get(14)?,
                equipment: row.get(4)?,
                imei: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                total_price: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                closed_at: row.get(11)?,
                display_id: row.get(12)?,
                discount_percent: row.get(13)?,
            })
        })?;

        let mut orders = Vec::new();
        for row in rows {
            orders.push(row?);
        }
        Ok(orders)
    }

    pub fn update(order: &ServiceOrder) -> Result<()> {
        let conn = get_db()?;
        Self::update_with_conn(&conn, order)
    }

    pub(crate) fn update_with_conn(conn: &Connection, order: &ServiceOrder) -> Result<()> {
        let updated = conn.execute(
            "UPDATE service_orders
               SET customer_id = ?1, customer_name = ?2, user_id = ?3, equipment = ?4, imei = ?5, description = ?6, total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?9), updated_at = ?7, discount_percent = ?8
                WHERE id = ?9 AND deleted_at IS NULL",
            params![
                order.customer_id,
                order.customer_name,
                order.user_id,
                order.equipment,
                order.imei,
                order.description,
                Utc::now().to_rfc3339(),
                order.discount_percent,
                order.id
            ],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let event = ServiceOrderEvent::new(
            order.id.clone(),
            "updated".to_string(),
            serde_json::json!({ "discountPercent": order.discount_percent }).to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(conn, &event)?;
        Ok(())
    }

    pub fn save_edit(
        service_order_id: &str,
        description: &str,
        discount_percent: f64,
        next_status: &str,
        restore_stock: bool,
        checklist: Vec<ChecklistItem>,
    ) -> std::result::Result<(), AppError> {
        let conn = get_db()?;
        Self::save_edit_with_conn(
            &conn,
            service_order_id,
            description,
            discount_percent,
            next_status,
            restore_stock,
            checklist,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn save_edit_with_conn(
        conn: &Connection,
        service_order_id: &str,
        description: &str,
        discount_percent: f64,
        next_status: &str,
        restore_stock: bool,
        checklist: Vec<ChecklistItem>,
    ) -> std::result::Result<(), AppError> {
        if !(0.0..=100.0).contains(&discount_percent) || !discount_percent.is_finite() {
            return Err(business_error(
                "Discount percentage must be between 0 and 100.",
                "O percentual de desconto deve estar entre 0 e 100.",
            ));
        }
        let transaction = conn.unchecked_transaction()?;
        ChecklistRepository::replace_os_checklist_in_transaction(
            &transaction,
            service_order_id,
            checklist,
        )?;

        let current_status: String = transaction
            .query_row(
                "SELECT status FROM service_orders WHERE id = ?1 AND deleted_at IS NULL",
                params![service_order_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    not_found("Service order", "Ordem de serviço")
                }
                other => other.into(),
            })?;
        if current_status != next_status {
            Self::transition_status_in_transaction(
                &transaction,
                service_order_id,
                next_status,
                restore_stock,
            )?;
        }

        Self::update_edit_in_transaction(
            &transaction,
            service_order_id,
            description,
            discount_percent,
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn update_edit_in_transaction(
        transaction: &Transaction<'_>,
        service_order_id: &str,
        description: &str,
        discount_percent: f64,
    ) -> Result<()> {
        let updated = transaction.execute(
            "UPDATE service_orders
             SET description = ?1,
                 discount_percent = ?2,
                 total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?3),
                 updated_at = ?4
             WHERE id = ?3 AND deleted_at IS NULL",
            params![
                description,
                discount_percent,
                service_order_id,
                Utc::now().to_rfc3339(),
            ],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let event = ServiceOrderEvent::new(
            service_order_id.to_string(),
            "updated".to_string(),
            serde_json::json!({ "discountPercent": discount_percent }).to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(transaction, &event)?;
        Ok(())
    }

    pub fn transition_status(
        service_order_id: &str,
        next_status: &str,
        restore_stock: bool,
    ) -> std::result::Result<ServiceOrder, AppError> {
        let conn = get_db()?;
        Self::transition_status_with_conn(&conn, service_order_id, next_status, restore_stock)
    }

    pub(crate) fn transition_status_with_conn(
        conn: &Connection,
        service_order_id: &str,
        next_status: &str,
        restore_stock: bool,
    ) -> std::result::Result<ServiceOrder, AppError> {
        let transaction = conn.unchecked_transaction()?;
        Self::transition_status_in_transaction(
            &transaction,
            service_order_id,
            next_status,
            restore_stock,
        )?;
        transaction.commit()?;

        Self::get_by_id_with_conn(conn, service_order_id)?
            .ok_or_else(|| not_found("Service order", "Ordem de serviço"))
    }

    fn transition_status_in_transaction(
        transaction: &Transaction<'_>,
        service_order_id: &str,
        next_status: &str,
        restore_stock: bool,
    ) -> std::result::Result<(), AppError> {
        const STATUSES: [&str; 5] = [
            "Orçamento",
            "Em Manutenção",
            "Aguardando Peça",
            "Finalizada",
            "Cancelada",
        ];

        if !STATUSES.contains(&next_status) {
            return Err(business_error(
                "Invalid service order status.",
                "Status da ordem de serviço inválido.",
            ));
        }

        let current_status: String = transaction
            .query_row(
                "SELECT status FROM service_orders WHERE id = ?1 AND deleted_at IS NULL",
                params![service_order_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    not_found("Service order", "Ordem de serviço")
                }
                other => other.into(),
            })?;

        let transition_allowed = matches!(
            (current_status.as_str(), next_status),
            (
                "Orçamento",
                "Em Manutenção" | "Aguardando Peça" | "Cancelada"
            ) | (
                "Em Manutenção",
                "Aguardando Peça" | "Finalizada" | "Cancelada"
            ) | ("Aguardando Peça", "Em Manutenção" | "Cancelada")
                | ("Finalizada", "Em Manutenção")
                | ("Cancelada", "Orçamento" | "Em Manutenção")
        );
        if !transition_allowed {
            return Err(business_error(
                "This service order status transition is not allowed.",
                "Essa transição de status da ordem de serviço não é permitida.",
            ));
        }

        if next_status == "Cancelada" {
            let mut stmt = transaction.prepare(
                "SELECT sop.inventory_item_id, sop.quantity
                 FROM service_order_parts sop
                 JOIN inventory_items ii ON ii.id = sop.inventory_item_id
                 WHERE sop.service_order_id = ?1 AND ii.type = 'part'",
            )?;
            let part_rows = stmt
                .query_map(params![service_order_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            if !part_rows.is_empty() && !restore_stock {
                return Err(business_error(
                    "Cancelling an order with consumed parts requires restoring stock.",
                    "Cancelar uma ordem com peças consumidas exige a devolução ao estoque.",
                ));
            }

            if restore_stock {
                for (inventory_item_id, quantity) in part_rows {
                    transaction.execute(
                        "UPDATE inventory_items
                         SET current_quantity = current_quantity + ?1, updated_at = ?2
                         WHERE id = ?3",
                        params![quantity, Utc::now().to_rfc3339(), inventory_item_id],
                    )?;
                    transaction.execute(
                        "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, created_at)
                         VALUES (?1, ?2, 'entrada', ?3, ?4, 'service_order_cancel', ?5)",
                        params![
                            Uuid::new_v4().to_string(),
                            inventory_item_id,
                            quantity,
                            service_order_id,
                            Utc::now().to_rfc3339(),
                        ],
                    )?;
                }
                transaction.execute(
                    "DELETE FROM service_order_parts
                     WHERE service_order_id = ?1
                     AND inventory_item_id IN (SELECT id FROM inventory_items WHERE type = 'part')",
                    params![service_order_id],
                )?;
            }
        }

        let closed_at = if matches!(next_status, "Finalizada" | "Cancelada") {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };
        transaction.execute(
            "UPDATE service_orders
             SET status = ?1,
                 closed_at = ?2,
                 total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?3),
                 updated_at = ?4
             WHERE id = ?3 AND deleted_at IS NULL",
            params![next_status, closed_at, service_order_id, Utc::now().to_rfc3339()],
        )?;
        let event = ServiceOrderEvent::new(
            service_order_id.to_string(),
            "status_changed".to_string(),
            serde_json::json!({
                "from": current_status,
                "to": next_status,
                "restoredStock": restore_stock,
            })
            .to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(&transaction, &event)?;
        Ok(())
    }

    pub fn delete(id: &str) -> Result<()> {
        let conn = get_db()?;
        Self::delete_with_conn(&conn, id)
    }

    pub(crate) fn delete_with_conn(conn: &Connection, id: &str) -> Result<()> {
        let updated = conn.execute(
            "UPDATE service_orders SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let event = ServiceOrderEvent::new(id.to_string(), "deleted".to_string(), "{}".to_string());
        ServiceOrderEventRepository::create_with_conn(conn, &event)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::inventory_item::InventoryItem;
    use crate::models::user::User;
    use crate::repositories::checklist_repo::ChecklistRepository;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::inventory_repo::InventoryRepository;
    use crate::repositories::user_repo::UserRepository;
    use crate::test_helpers::setup_db;

    fn seed_customer(conn: &Connection) -> Customer {
        let customer = Customer::new(
            "Ana".to_string(),
            "41999990000".to_string(),
            "ana@example.com".to_string(),
            "Rua das Flores".to_string(),
        );
        CustomerRepository::create_with_conn(conn, &customer).unwrap();
        customer
    }

    fn seed_user(conn: &Connection) -> User {
        let user = User::new("Técnico 1".to_string(), "tecnico@example.com".to_string());
        UserRepository::create_with_conn(conn, &user).unwrap();
        user
    }

    fn seed_part(conn: &Connection, stock: i32) -> InventoryItem {
        let part = InventoryItem::new(
            "Bateria".to_string(),
            "Bateria iPhone".to_string(),
            "part".to_string(),
            1,
            stock,
            30.0,
            80.0,
        );
        InventoryRepository::create_with_conn(conn, &part).unwrap();
        part
    }

    fn seed_service(conn: &Connection) -> InventoryItem {
        let service = InventoryItem::new(
            "Limpeza técnica".to_string(),
            "Limpeza interna do aparelho".to_string(),
            "service".to_string(),
            0,
            0,
            0.0,
            50.0,
        );
        InventoryRepository::create_with_conn(conn, &service).unwrap();
        service
    }

    fn build_order(customer_id: &str) -> ServiceOrder {
        ServiceOrder::new(
            customer_id.to_string(),
            "iPhone 14".to_string(),
            "Troca de bateria".to_string(),
        )
    }

    #[test]
    fn create_assigns_incrementing_display_ids() {
        let conn = setup_db();
        let customer = seed_customer(&conn);
        let mut first = build_order(&customer.id);
        let mut second = build_order(&customer.id);

        ServiceOrderRepository::create_with_conn(&conn, &mut first).unwrap();
        ServiceOrderRepository::create_with_conn(&conn, &mut second).unwrap();

        assert_eq!(first.display_id, "OS-000001");
        assert_eq!(second.display_id, "OS-000002");
    }

    #[test]
    fn get_by_id_returns_joined_customer_and_user_names() {
        let conn = setup_db();
        let customer = seed_customer(&conn);
        let user = seed_user(&conn);
        let mut order = build_order(&customer.id);
        order.user_id = Some(user.id.clone());

        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();

        let fetched = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched.customer_name.as_deref(), Some("Ana"));
        assert_eq!(fetched.user_name.as_deref(), Some("Técnico 1"));
    }

    #[test]
    fn update_persists_metadata_without_overwriting_status() {
        let conn = setup_db();
        let customer = seed_customer(&conn);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();

        order.status = "Finalizada".to_string();
        order.description = "Troca de bateria e limpeza".to_string();
        order.discount_percent = 15.0;
        ServiceOrderRepository::update_with_conn(&conn, &order).unwrap();

        let fetched = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.status, "Orçamento");
        assert_eq!(fetched.description, "Troca de bateria e limpeza");
        assert_eq!(fetched.discount_percent, 15.0);
    }

    #[test]
    fn add_part_updates_inventory_movements_and_order_total() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let part = seed_part(&conn, 5);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();

        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 2,
        )
        .unwrap();

        let updated_part = InventoryRepository::get_by_id_with_conn(&conn, &part.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &part.id).unwrap();
        let parts =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id).unwrap();
        let total_price: f64 = conn
            .query_row(
                "SELECT COALESCE(total_price, 0.0) FROM service_orders WHERE id = ?1",
                params![order.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(updated_part.current_quantity, 3);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].quantity, 2);
        assert_eq!(total_price, 160.0);
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].r#type, "saida");
        assert_eq!(
            movements[0].reference_os_id.as_deref(),
            Some(order.id.as_str())
        );
    }

    #[test]
    fn add_part_rejects_insufficient_stock() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let part = seed_part(&conn, 1);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();

        let result = ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 2,
        );

        assert!(result.is_err());
        assert!(
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn remove_part_restores_inventory_and_recalculates_total() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let part = seed_part(&conn, 5);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 2,
        )
        .unwrap();

        let part_row = ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id)
            .unwrap()
            .remove(0);

        ServiceOrderRepository::remove_part_from_service_order_with_conn(&mut conn, &part_row.id)
            .unwrap();

        let updated_part = InventoryRepository::get_by_id_with_conn(&conn, &part.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &part.id).unwrap();
        let remaining_parts =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id).unwrap();
        let total_price: f64 = conn
            .query_row(
                "SELECT COALESCE(total_price, 0.0) FROM service_orders WHERE id = ?1",
                params![order.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(updated_part.current_quantity, 5);
        assert!(remaining_parts.is_empty());
        assert_eq!(total_price, 0.0);
        assert_eq!(movements.len(), 2);
        assert_eq!(movements[0].r#type, "entrada");
        assert_eq!(movements[1].r#type, "saida");
    }

    #[test]
    fn updating_part_quantity_adjusts_stock_movements_and_total() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let part = seed_part(&conn, 5);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 2,
        )
        .unwrap();
        let order_part =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id)
                .unwrap()
                .remove(0);

        ServiceOrderRepository::update_part_quantity_with_conn(&conn, &order_part.id, 4).unwrap();
        let part_after_increase = InventoryRepository::get_by_id_with_conn(&conn, &part.id)
            .unwrap()
            .unwrap();
        let increased_order = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();
        assert_eq!(part_after_increase.current_quantity, 1);
        assert_eq!(increased_order.total_price, Some(320.0));

        ServiceOrderRepository::update_part_quantity_with_conn(&conn, &order_part.id, 1).unwrap();
        let part_after_decrease = InventoryRepository::get_by_id_with_conn(&conn, &part.id)
            .unwrap()
            .unwrap();
        let decreased_order = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &part.id).unwrap();
        assert_eq!(part_after_decrease.current_quantity, 4);
        assert_eq!(decreased_order.total_price, Some(80.0));
        assert_eq!(movements.len(), 3);
    }

    #[test]
    fn updating_service_quantity_does_not_change_stock() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let service = seed_service(&conn);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn,
            &order.id,
            &service.id,
            1,
        )
        .unwrap();
        let order_part =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id)
                .unwrap()
                .remove(0);

        ServiceOrderRepository::update_part_quantity_with_conn(&conn, &order_part.id, 3).unwrap();
        let persisted_service = InventoryRepository::get_by_id_with_conn(&conn, &service.id)
            .unwrap()
            .unwrap();
        let updated_order = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();

        assert_eq!(persisted_service.current_quantity, 0);
        assert_eq!(updated_order.total_price, Some(150.0));
        assert!(
            InventoryRepository::get_movements_with_conn(&conn, &service.id)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn updating_part_quantity_rejects_invalid_or_unavailable_stock() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let part = seed_part(&conn, 3);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 1,
        )
        .unwrap();
        let order_part =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id)
                .unwrap()
                .remove(0);

        let zero_quantity =
            ServiceOrderRepository::update_part_quantity_with_conn(&conn, &order_part.id, 0);
        let insufficient_stock =
            ServiceOrderRepository::update_part_quantity_with_conn(&conn, &order_part.id, 4);

        assert!(zero_quantity.is_err());
        assert!(insufficient_stock.is_err());
    }

    #[test]
    fn finalizing_allows_unchecked_checklist_items() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ChecklistRepository::save_os_checklist_with_conn(
            &mut conn,
            &order.id,
            vec![crate::models::checklist::ChecklistItem {
                id: "check-1".to_string(),
                label: "Tela sem riscos".to_string(),
                checked: false,
            }],
        )
        .unwrap();

        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();
        let result = ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Finalizada",
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn save_edit_finalizes_with_unchecked_checklist_items() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ChecklistRepository::save_os_checklist_with_conn(
            &mut conn,
            &order.id,
            vec![ChecklistItem {
                id: "check-1".to_string(),
                label: "Checklist original".to_string(),
                checked: true,
            }],
        )
        .unwrap();
        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();

        ServiceOrderRepository::save_edit_with_conn(
            &conn,
            &order.id,
            "Serviço concluído",
            15.0,
            "Finalizada",
            false,
            vec![ChecklistItem {
                id: "check-2".to_string(),
                label: "Item não checado".to_string(),
                checked: false,
            }],
        )
        .unwrap();

        let persisted = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();
        let checklist = ChecklistRepository::get_os_checklist_with_conn(&conn, &order.id).unwrap();
        assert_eq!(persisted.status, "Finalizada");
        assert_eq!(persisted.description, "Serviço concluído");
        assert_eq!(persisted.discount_percent, 15.0);
        assert_eq!(checklist.len(), 1);
        assert_eq!(checklist[0].label, "Item não checado");
        assert!(!checklist[0].checked);
    }

    #[test]
    fn save_edit_updates_checklist_status_and_metadata_together() {
        let conn = setup_db();
        let customer = seed_customer(&conn);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Em Manutenção",
            false,
        )
        .unwrap();

        ServiceOrderRepository::save_edit_with_conn(
            &conn,
            &order.id,
            "Troca de bateria concluída",
            10.0,
            "Finalizada",
            false,
            vec![ChecklistItem {
                id: "check-1".to_string(),
                label: "Teste final".to_string(),
                checked: true,
            }],
        )
        .unwrap();

        let persisted = ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
            .unwrap()
            .unwrap();
        let checklist = ChecklistRepository::get_os_checklist_with_conn(&conn, &order.id).unwrap();
        assert_eq!(persisted.status, "Finalizada");
        assert_eq!(persisted.description, "Troca de bateria concluída");
        assert_eq!(persisted.discount_percent, 10.0);
        assert!(persisted.closed_at.is_some());
        assert_eq!(checklist.len(), 1);
        assert!(checklist[0].checked);
    }

    #[test]
    fn cancelling_with_stock_restore_returns_parts_and_removes_lines() {
        let mut conn = setup_db();
        let customer = seed_customer(&conn);
        let part = seed_part(&conn, 4);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::add_part_to_service_order_with_conn(
            &mut conn, &order.id, &part.id, 2,
        )
        .unwrap();

        let blocked = ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Cancelada",
            false,
        );
        assert!(blocked.is_err());

        let cancelled = ServiceOrderRepository::transition_status_with_conn(
            &conn,
            &order.id,
            "Cancelada",
            true,
        )
        .unwrap();
        let item = InventoryRepository::get_by_id_with_conn(&conn, &part.id)
            .unwrap()
            .unwrap();
        let lines =
            ServiceOrderRepository::get_service_order_parts_with_conn(&conn, &order.id).unwrap();

        assert_eq!(cancelled.status, "Cancelada");
        assert_eq!(item.current_quantity, 4);
        assert!(lines.is_empty());
        assert!(cancelled.closed_at.is_some());
    }

    #[test]
    fn deleted_order_is_not_returned_by_id() {
        let conn = setup_db();
        let customer = seed_customer(&conn);
        let mut order = build_order(&customer.id);
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        ServiceOrderRepository::delete_with_conn(&conn, &order.id).unwrap();

        assert!(
            ServiceOrderRepository::get_by_id_with_conn(&conn, &order.id)
                .unwrap()
                .is_none()
        );
    }
}
