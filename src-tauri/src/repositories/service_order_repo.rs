use crate::database::get_db;
use crate::models::service_order::ServiceOrder;
use chrono::Utc;
use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOrderPart {
    pub id: String,
    pub service_order_id: String,
    pub inventory_item_id: String,
    pub inventory_item_name: String,
    pub quantity: i32,
    pub unit_cost: f64,
    pub unit_price: f64,
}

pub struct ServiceOrderRepository;

impl ServiceOrderRepository {
    pub fn get_service_order_parts(service_order_id: &str) -> Result<Vec<ServiceOrderPart>> {
        let conn = get_db()?;
        
        let mut stmt = conn.prepare(
            "SELECT sop.id, sop.service_order_id, sop.inventory_item_id, ii.name as inventory_item_name, sop.quantity, sop.unit_cost, sop.unit_price 
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
                quantity: row.get(4)?,
                unit_cost: row.get(5)?,
                unit_price: row.get(6)?,
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

        // Start a transaction to ensure consistency
        let transaction = conn.transaction()?;

        let (current_quantity, unit_cost, unit_price) = {
            // 1. Check if there is sufficient inventory and get prices
            let mut stmt = transaction
                .prepare("SELECT current_quantity, cost_price, sale_price FROM inventory_items WHERE id = ?1")?;
            let mut rows = stmt.query_map(params![inventory_item_id], |row: &rusqlite::Row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?))
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

        if current_quantity < quantity {
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

        // 3. Update inventory quantity
        transaction.execute(
            "UPDATE inventory_items 
             SET current_quantity = current_quantity - ?1 
             WHERE id = ?2",
            params![quantity, inventory_item_id],
        )?;

        // 4. Update the Service Order total price
        transaction.execute(
            "UPDATE service_orders 
             SET total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?1)
             WHERE id = ?1",
            params![service_order_id],
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub fn remove_part_from_service_order(part_id: &str) -> Result<()> {
        let mut conn = get_db()?;
        let transaction = conn.transaction()?;

        let (os_id, inventory_item_id, quantity) = {
            let mut stmt = transaction.prepare(
                "SELECT service_order_id, inventory_item_id, quantity FROM service_order_parts WHERE id = ?1"
            )?;
            stmt.query_row(params![part_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
            })?
        };

        // 1. Restore inventory
        transaction.execute(
            "UPDATE inventory_items SET current_quantity = current_quantity + ?1 WHERE id = ?2",
            params![quantity, inventory_item_id],
        )?;

        // 2. Delete the part record
        transaction.execute("DELETE FROM service_order_parts WHERE id = ?1", params![part_id])?;

        // 3. Recalculate OS total
        transaction.execute(
            "UPDATE service_orders 
             SET total_price = (SELECT COALESCE(SUM(quantity * unit_price), 0.0) FROM service_order_parts WHERE service_order_id = ?1)
             WHERE id = ?1",
            params![os_id],
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub fn create(order: &ServiceOrder) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "INSERT INTO service_orders (id, customer_id, customer_name, user_id, equipment, imei, description, status, total_price, signature_path, created_at, updated_at, closed_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
                order.signature_path,
                order.created_at,
                order.updated_at,
                order.closed_at
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<ServiceOrder>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT so.id, so.customer_id, COALESCE(so.customer_name, c.name) as customer_name, so.user_id, so.equipment, so.imei, so.description, so.status, so.total_price, so.signature_path, so.created_at, so.updated_at, so.closed_at 
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             WHERE so.id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                user_id: row.get(3)?,
                equipment: row.get(4)?,
                imei: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                total_price: row.get(8)?,
                signature_path: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                closed_at: row.get(12)?,
            })
        })?;

        let order = rows.next().transpose()?;
        Ok(order)
    }

    pub fn get_all() -> Result<Vec<ServiceOrder>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT so.id, so.customer_id, COALESCE(so.customer_name, c.name) as customer_name, so.user_id, so.equipment, so.imei, so.description, so.status, so.total_price, so.signature_path, so.created_at, so.updated_at, so.closed_at 
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             WHERE so.deleted_at IS NULL
             ORDER BY so.created_at DESC"
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                user_id: row.get(3)?,
                equipment: row.get(4)?,
                imei: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                total_price: row.get(8)?,
                signature_path: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                closed_at: row.get(12)?,
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

        let mut stmt = conn.prepare(
            "SELECT so.id, so.customer_id, COALESCE(so.customer_name, c.name) as customer_name, so.user_id, so.equipment, so.imei, so.description, so.status, so.total_price, so.signature_path, so.created_at, so.updated_at, so.closed_at 
             FROM service_orders so
             LEFT JOIN customers c ON so.customer_id = c.id
             WHERE so.customer_id = ?1 AND so.deleted_at IS NULL"
        )?;
        let rows = stmt.query_map(params![customer_id], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                user_id: row.get(3)?,
                equipment: row.get(4)?,
                imei: row.get(5)?,
                description: row.get(6)?,
                status: row.get(7)?,
                total_price: row.get(8)?,
                signature_path: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                closed_at: row.get(12)?,
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

        conn.execute(
            "UPDATE service_orders 
             SET customer_id = ?1, customer_name = ?2, user_id = ?3, equipment = ?4, imei = ?5, description = ?6, status = ?7, total_price = ?8, signature_path = ?9, updated_at = ?10, closed_at = ?11
             WHERE id = ?12",
            params![
                order.customer_id,
                order.customer_name,
                order.user_id,
                order.equipment,
                order.imei,
                order.description,
                order.status,
                order.total_price,
                order.signature_path,
                Utc::now().to_rfc3339(),
                order.closed_at,
                order.id
            ],
        )?;
        Ok(())
    }

    pub fn delete(id: &str) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "UPDATE service_orders SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}
