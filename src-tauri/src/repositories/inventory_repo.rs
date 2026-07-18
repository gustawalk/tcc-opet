use crate::database::get_db;
use crate::models::inventory_item::InventoryItem;
use crate::models::inventory_movement::InventoryMovement;
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct InventoryRepository;

impl InventoryRepository {
    pub fn create(item: &InventoryItem) -> Result<()> {
        let conn = get_db()?;
        Self::create_with_conn(&conn, item)
    }

    pub(crate) fn create_with_conn(conn: &Connection, item: &InventoryItem) -> Result<()> {

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
        Self::get_by_id_with_conn(&conn, id)
    }

    pub(crate) fn get_by_id_with_conn(conn: &Connection, id: &str) -> Result<Option<InventoryItem>> {

        let mut stmt = conn.prepare(
            "SELECT id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, updated_at, deleted_at 
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
                updated_at: row.get(9)?,
                deleted_at: row.get(10)?,
            })
        })?;

        let item = rows.next().transpose()?;
        Ok(item)
    }

    pub fn get_all() -> Result<Vec<InventoryItem>> {
        let conn = get_db()?;
        Self::get_all_with_conn(&conn)
    }

    pub(crate) fn get_all_with_conn(conn: &Connection) -> Result<Vec<InventoryItem>> {

        let mut stmt = conn.prepare(
            "SELECT id, name, description, type, min_quantity, current_quantity, cost_price, sale_price, created_at, updated_at, deleted_at 
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
                updated_at: row.get(9)?,
                deleted_at: row.get(10)?,
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
        Self::update_with_conn(&conn, item)
    }

    pub(crate) fn update_with_conn(conn: &Connection, item: &InventoryItem) -> Result<()> {

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
        Self::delete_with_conn(&conn, id)
    }

    pub(crate) fn delete_with_conn(conn: &Connection, id: &str) -> Result<()> {

        conn.execute(
            "UPDATE inventory_items SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn remove_stock(item_id: &str, quantity: i32) -> Result<()> {
        let conn = get_db()?;
        Self::remove_stock_with_conn(&conn, item_id, quantity)
    }

    pub(crate) fn remove_stock_with_conn(conn: &Connection, item_id: &str, quantity: i32) -> Result<()> {

        conn.execute_batch("BEGIN")?;

        let actual: i32 = conn.query_row(
            "SELECT MIN(current_quantity, ?1) FROM inventory_items WHERE id = ?2 AND deleted_at IS NULL",
            params![quantity, item_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "UPDATE inventory_items SET current_quantity = current_quantity - ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            params![actual, Utc::now().to_rfc3339(), item_id],
        )?;

        let movement = InventoryMovement::new(
            item_id.to_string(),
            "saida".to_string(),
            actual,
            None,
        );

        conn.execute(
            "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![movement.id, movement.inventory_item_id, movement.r#type, movement.quantity, movement.reference_os_id, movement.created_at],
        )?;

        conn.execute_batch("COMMIT")?;
        Ok(())
    }

    pub fn add_stock(item_id: &str, quantity: i32) -> Result<()> {
        let conn = get_db()?;
        Self::add_stock_with_conn(&conn, item_id, quantity)
    }

    pub(crate) fn add_stock_with_conn(conn: &Connection, item_id: &str, quantity: i32) -> Result<()> {

        conn.execute_batch("BEGIN")?;

        conn.execute(
            "UPDATE inventory_items SET current_quantity = current_quantity + ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            params![quantity, Utc::now().to_rfc3339(), item_id],
        )?;

        let movement = InventoryMovement::new(
            item_id.to_string(),
            "entrada".to_string(),
            quantity,
            None,
        );

        conn.execute(
            "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![movement.id, movement.inventory_item_id, movement.r#type, movement.quantity, movement.reference_os_id, movement.created_at],
        )?;

        conn.execute_batch("COMMIT")?;
        Ok(())
    }

    pub fn get_movements(item_id: &str) -> Result<Vec<InventoryMovement>> {
        let conn = get_db()?;
        Self::get_movements_with_conn(&conn, item_id)
    }

    pub(crate) fn get_movements_with_conn(conn: &Connection, item_id: &str) -> Result<Vec<InventoryMovement>> {

        let mut stmt = conn.prepare(
            "SELECT im.id, im.inventory_item_id, im.type, im.quantity, im.reference_os_id, im.created_at, so.display_id AS os_display_id
             FROM inventory_movements im
             LEFT JOIN service_orders so ON im.reference_os_id = so.id
             WHERE im.inventory_item_id = ?1
             ORDER BY im.created_at DESC"
        )?;

        let rows = stmt.query_map(params![item_id], |row: &rusqlite::Row| {
            Ok(InventoryMovement {
                id: row.get(0)?,
                inventory_item_id: row.get(1)?,
                r#type: row.get(2)?,
                quantity: row.get(3)?,
                reference_os_id: row.get(4)?,
                created_at: row.get(5)?,
                os_display_id: row.get(6)?,
            })
        })?;

        let mut movements = Vec::new();
        for row in rows {
            movements.push(row?);
        }
        Ok(movements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::inventory_item::InventoryItem;
    use crate::test_helpers::setup_db;

    fn sample_item() -> InventoryItem {
        InventoryItem::new(
            "Tela".to_string(),
            "Tela OLED".to_string(),
            "part".to_string(),
            2,
            5,
            50.0,
            120.0,
        )
    }

    #[test]
    fn create_and_get_inventory_item() {
        let conn = setup_db();
        let item = sample_item();

        InventoryRepository::create_with_conn(&conn, &item).unwrap();
        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();

        assert_eq!(fetched.name, "Tela");
        assert_eq!(fetched.current_quantity, 5);
    }

    #[test]
    fn delete_inventory_item_soft_deletes_record() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        InventoryRepository::delete_with_conn(&conn, &item.id).unwrap();

        assert!(InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .is_none());
        assert!(InventoryRepository::get_all_with_conn(&conn).unwrap().is_empty());
    }

    #[test]
    fn add_stock_updates_quantity_and_logs_movement() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        InventoryRepository::add_stock_with_conn(&conn, &item.id, 3).unwrap();

        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &item.id).unwrap();

        assert_eq!(fetched.current_quantity, 8);
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].r#type, "entrada");
        assert_eq!(movements[0].quantity, 3);
    }

    #[test]
    fn remove_stock_updates_quantity_and_logs_movement() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        InventoryRepository::remove_stock_with_conn(&conn, &item.id, 2).unwrap();

        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &item.id).unwrap();

        assert_eq!(fetched.current_quantity, 3);
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].r#type, "saida");
        assert_eq!(movements[0].quantity, 2);
    }

    #[test]
    fn remove_stock_caps_removal_to_available_quantity() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        InventoryRepository::remove_stock_with_conn(&conn, &item.id, 99).unwrap();

        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &item.id).unwrap();

        assert_eq!(fetched.current_quantity, 0);
        assert_eq!(movements[0].quantity, 5);
    }
}
