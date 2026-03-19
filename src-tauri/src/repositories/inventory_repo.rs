use crate::database::get_db;
use crate::models::inventory_item::InventoryItem;
use chrono::Utc;
use rusqlite::{params, Result};

pub struct InventoryRepository;

impl InventoryRepository {
    pub fn create(item: &InventoryItem) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "INSERT INTO inventory_items (id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                item.id,
                item.name,
                item.description,
                item.r#type,
                item.min_quantity,
                item.current_quantity,
                item.cost_price,
                item.sale_price,
                item.created_at,
                item.deleted_at
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<InventoryItem>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, deleted_at 
             FROM inventory_items WHERE id = ?1 AND deleted_at IS NULL"
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                r#type: row.get(3)?,
                min_quantity: row.get(4)?,
                current_quantity: row.get(5)?,
                cost_price: row.get(6)?,
                sale_price: row.get(7)?,
                created_at: row.get(8)?,
                deleted_at: row.get(9)?,
            })
        })?;

        let item = rows.next().transpose()?;
        Ok(item)
    }

    pub fn get_all() -> Result<Vec<InventoryItem>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, deleted_at 
             FROM inventory_items WHERE deleted_at IS NULL"
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(InventoryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                r#type: row.get(3)?,
                min_quantity: row.get(4)?,
                current_quantity: row.get(5)?,
                cost_price: row.get(6)?,
                sale_price: row.get(7)?,
                created_at: row.get(8)?,
                deleted_at: row.get(9)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn update(item: &InventoryItem) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "UPDATE inventory_items 
             SET name = ?1, description = ?2, type = ?3, min_quantity = ?4, current_quantity = ?5, cost_price = ?6, sale_price = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                item.name,
                item.description,
                item.r#type,
                item.min_quantity,
                item.current_quantity,
                item.cost_price,
                item.sale_price,
                Utc::now().to_rfc3339(),
                item.id
            ],
        )?;
        Ok(())
    }

    pub fn delete(id: &str) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "UPDATE inventory_items SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}
