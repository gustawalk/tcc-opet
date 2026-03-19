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
}

pub struct ServiceOrderRepository;

impl ServiceOrderRepository {
    pub fn get_service_order_parts(service_order_id: &str) -> Result<Vec<ServiceOrderPart>> {
        let conn = get_db()?;
        
        let mut stmt = conn.prepare(
            "SELECT sop.id, sop.service_order_id, sop.inventory_item_id, ii.name as inventory_item_name, sop.quantity, sop.unit_cost 
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
            })
        })?;
        
        let mut parts = Vec::new();
        for row in rows {
            parts.push(row?);
        }
        Ok(parts)
    }

    /// Add a part to a service order and update inventory
    pub fn add_part_to_service_order(
        service_order_id: &str,
        inventory_item_id: &str,
        quantity: i32,
    ) -> Result<()> {
        let mut conn = get_db()?;

        // Start a transaction to ensure consistency
        let transaction = conn.transaction()?;

        let (current_quantity, unit_cost) = {
            // 1. Check if there is sufficient inventory
            let mut stmt = transaction
                .prepare("SELECT current_quantity, cost_price FROM inventory_items WHERE id = ?1")?;
            let mut rows = stmt.query_map(params![inventory_item_id], |row: &rusqlite::Row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, f64>(1)?))
            })?;

            let qty_cost = match rows.next() {
                Some(Ok(qty_cost)) => qty_cost,
                Some(Err(e)) => return Err(e),
                None => return Err(rusqlite::Error::QueryReturnedNoRows),
            };
            qty_cost
        };

        if current_quantity < quantity {
            return Err(rusqlite::Error::InvalidQuery);
        }

        // 2. Record the part usage
        transaction.execute(
            "INSERT INTO service_order_parts (id, service_order_id, inventory_item_id, quantity, unit_cost) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                service_order_id,
                inventory_item_id,
                quantity,
                unit_cost
            ],
        )?;

        // 3. Update inventory quantity
        transaction.execute(
            "UPDATE inventory_items 
             SET current_quantity = current_quantity - ?1 
             WHERE id = ?2",
            params![quantity, inventory_item_id],
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub fn create(order: &ServiceOrder) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "INSERT INTO service_orders (id, customer_id, customer_name, equipment, imei, description, status, total_price, signature_path, created_at, updated_at, closed_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                order.id,
                order.customer_id,
                order.customer_name,
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
            "SELECT id, customer_id, customer_name, equipment, imei, description, status, total_price, signature_path, created_at, updated_at, closed_at 
             FROM service_orders WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                equipment: row.get(3)?,
                imei: row.get(4)?,
                description: row.get(5)?,
                status: row.get(6)?,
                total_price: row.get(7)?,
                signature_path: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                closed_at: row.get(11)?,
            })
        })?;

        let order = rows.next().transpose()?;
        Ok(order)
    }

    pub fn get_all() -> Result<Vec<ServiceOrder>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT id, customer_id, customer_name, equipment, imei, description, status, total_price, signature_path, created_at, updated_at, closed_at 
             FROM service_orders"
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                equipment: row.get(3)?,
                imei: row.get(4)?,
                description: row.get(5)?,
                status: row.get(6)?,
                total_price: row.get(7)?,
                signature_path: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                closed_at: row.get(11)?,
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
            "SELECT id, customer_id, customer_name, equipment, imei, description, status, total_price, signature_path, created_at, updated_at, closed_at 
             FROM service_orders WHERE customer_id = ?1"
        )?;
        let rows = stmt.query_map(params![customer_id], |row: &rusqlite::Row| {
            Ok(ServiceOrder {
                id: row.get(0)?,
                customer_id: row.get(1)?,
                customer_name: row.get(2)?,
                equipment: row.get(3)?,
                imei: row.get(4)?,
                description: row.get(5)?,
                status: row.get(6)?,
                total_price: row.get(7)?,
                signature_path: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                closed_at: row.get(11)?,
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
             SET customer_id = ?1, customer_name = ?2, equipment = ?3, imei = ?4, description = ?5, status = ?6, total_price = ?7, signature_path = ?8, updated_at = ?9, closed_at = ?10
             WHERE id = ?11",
            params![
                order.customer_id,
                order.customer_name,
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

        conn.execute("DELETE FROM service_orders WHERE id = ?1", params![id])?;
        Ok(())
    }
}
