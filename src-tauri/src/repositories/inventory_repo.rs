use crate::database::get_db;
use crate::models::inventory_item::InventoryItem;
use crate::models::inventory_movement::InventoryMovement;
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InactiveInventoryItem {
    pub id: String,
    pub name: String,
    pub current_quantity: i32,
    pub last_movement_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbcInventoryGroup {
    pub classification: String,
    pub item_count: i32,
    pub inventory_value: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryInsights {
    pub inactive_items: Vec<InactiveInventoryItem>,
    pub abc_groups: Vec<AbcInventoryGroup>,
}

pub struct InventoryRepository;

impl InventoryRepository {
    pub fn create(item: &InventoryItem) -> Result<()> {
        let conn = get_db()?;
        Self::create_with_conn(&conn, item)
    }

    pub(crate) fn create_with_conn(conn: &Connection, item: &InventoryItem) -> Result<()> {
        conn.execute(
            "INSERT INTO inventory_items (id, name, description, type, min_quantity, current_quantity, cost_price, average_cost, sale_price, supplier_name, created_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                item.id,
                item.name,
                item.description,
                item.r#type,
                item.min_quantity,
                item.current_quantity,
                item.cost_price,
                item.average_cost,
                item.sale_price,
                item.supplier_name,
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

    pub(crate) fn get_by_id_with_conn(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<InventoryItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, type, min_quantity, current_quantity, cost_price, average_cost, sale_price, supplier_name, created_at, updated_at, deleted_at
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
                average_cost: row.get(7)?,
                sale_price: row.get(8)?,
                supplier_name: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                deleted_at: row.get(12)?,
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
            "SELECT id, name, description, type, min_quantity, current_quantity, cost_price, average_cost, sale_price, supplier_name, created_at, updated_at, deleted_at
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
                average_cost: row.get(7)?,
                sale_price: row.get(8)?,
                supplier_name: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                deleted_at: row.get(12)?,
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
        let updated = conn.execute(
            "UPDATE inventory_items 
             SET name = ?1, description = ?2, type = ?3, min_quantity = ?4, cost_price = ?5, sale_price = ?6, supplier_name = ?7, updated_at = ?8
              WHERE id = ?9 AND deleted_at IS NULL",
            params![
                item.name,
                item.description,
                item.r#type,
                item.min_quantity,
                item.cost_price,
                item.sale_price,
                item.supplier_name,
                Utc::now().to_rfc3339(),
                item.id
            ],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn delete(id: &str) -> Result<()> {
        let conn = get_db()?;
        Self::delete_with_conn(&conn, id)
    }

    pub(crate) fn delete_with_conn(conn: &Connection, id: &str) -> Result<()> {
        let updated = conn.execute(
            "UPDATE inventory_items SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn remove_stock(item_id: &str, quantity: i32) -> Result<()> {
        let conn = get_db()?;
        Self::remove_stock_with_conn(&conn, item_id, quantity)
    }

    pub(crate) fn remove_stock_with_conn(
        conn: &Connection,
        item_id: &str,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let transaction = conn.unchecked_transaction()?;
        let current_quantity: i32 = transaction.query_row(
            "SELECT current_quantity FROM inventory_items WHERE id = ?1 AND type = 'part' AND deleted_at IS NULL",
            params![item_id],
            |row| row.get(0),
        )?;
        if current_quantity < quantity {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let updated = transaction.execute(
            "UPDATE inventory_items
             SET current_quantity = current_quantity - ?1, updated_at = ?2
             WHERE id = ?3 AND type = 'part' AND deleted_at IS NULL AND current_quantity >= ?1",
            params![quantity, Utc::now().to_rfc3339(), item_id],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let mut movement =
            InventoryMovement::new(item_id.to_string(), "saida".to_string(), quantity, None);
        movement.reason = "manual_removal".to_string();

        transaction.execute(
            "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![movement.id, movement.inventory_item_id, movement.r#type, movement.quantity, movement.reference_os_id, movement.reason, movement.created_at],
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub fn add_stock_with_details(
        item_id: &str,
        quantity: i32,
        unit_cost: Option<f64>,
        reason: Option<String>,
    ) -> Result<()> {
        let conn = get_db()?;
        Self::add_stock_with_details_with_conn(&conn, item_id, quantity, unit_cost, reason)
    }

    pub(crate) fn add_stock_with_details_with_conn(
        conn: &Connection,
        item_id: &str,
        quantity: i32,
        unit_cost: Option<f64>,
        reason: Option<String>,
    ) -> Result<()> {
        if quantity <= 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if unit_cost.is_some_and(|cost| cost <= 0.0) {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let transaction = conn.unchecked_transaction()?;
        let (current_quantity, cost_price, average_cost): (i32, f64, f64) = transaction.query_row(
            "SELECT current_quantity, cost_price, average_cost FROM inventory_items WHERE id = ?1 AND type = 'part' AND deleted_at IS NULL",
            params![item_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let effective_cost = unit_cost.unwrap_or(cost_price);
        let previous_average = if average_cost > 0.0 {
            average_cost
        } else {
            cost_price
        };
        let new_average = ((current_quantity as f64 * previous_average)
            + (quantity as f64 * effective_cost))
            / (current_quantity + quantity) as f64;
        let updated = transaction.execute(
            "UPDATE inventory_items
              SET current_quantity = current_quantity + ?1, average_cost = ?2, updated_at = ?3
              WHERE id = ?4 AND type = 'part' AND deleted_at IS NULL",
            params![quantity, new_average, Utc::now().to_rfc3339(), item_id],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let mut movement =
            InventoryMovement::new(item_id.to_string(), "entrada".to_string(), quantity, None);
        movement.reason = reason.unwrap_or_else(|| "manual_restock".to_string());
        movement.unit_cost = Some(effective_cost);

        transaction.execute(
            "INSERT INTO inventory_movements (id, inventory_item_id, type, quantity, reference_os_id, reason, unit_cost, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![movement.id, movement.inventory_item_id, movement.r#type, movement.quantity, movement.reference_os_id, movement.reason, movement.unit_cost, movement.created_at],
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub fn get_movements(item_id: &str) -> Result<Vec<InventoryMovement>> {
        let conn = get_db()?;
        Self::get_movements_with_conn(&conn, item_id)
    }

    pub(crate) fn get_movements_with_conn(
        conn: &Connection,
        item_id: &str,
    ) -> Result<Vec<InventoryMovement>> {
        let mut stmt = conn.prepare(
            "SELECT im.id, im.inventory_item_id, im.type, im.quantity, im.reference_os_id, im.reason, im.unit_cost, im.created_at, so.display_id AS os_display_id
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
                reason: row.get(5)?,
                unit_cost: row.get(6)?,
                created_at: row.get(7)?,
                os_display_id: row.get(8)?,
            })
        })?;

        let mut movements = Vec::new();
        for row in rows {
            movements.push(row?);
        }
        Ok(movements)
    }

    pub fn get_insights(inactive_days: i32) -> Result<InventoryInsights> {
        let conn = get_db()?;
        Self::get_insights_with_conn(&conn, inactive_days)
    }

    pub(crate) fn get_insights_with_conn(
        conn: &Connection,
        inactive_days: i32,
    ) -> Result<InventoryInsights> {
        let cutoff = Utc::now() - chrono::Duration::days(inactive_days as i64);
        let cutoff = cutoff.to_rfc3339();
        let mut inactive_stmt = conn.prepare(
            "SELECT i.id, i.name, i.current_quantity, MAX(m.created_at)
             FROM inventory_items i LEFT JOIN inventory_movements m ON m.inventory_item_id = i.id
             WHERE i.type = 'part' AND i.deleted_at IS NULL
             GROUP BY i.id HAVING COALESCE(MAX(m.created_at), i.created_at) < ?1
             ORDER BY COALESCE(MAX(m.created_at), i.created_at) ASC",
        )?;
        let inactive_items = inactive_stmt
            .query_map(params![cutoff], |row| {
                Ok(InactiveInventoryItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    current_quantity: row.get(2)?,
                    last_movement_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let mut value_stmt = conn.prepare(
            "SELECT current_quantity * CASE WHEN average_cost > 0 THEN average_cost ELSE cost_price END
             FROM inventory_items WHERE type = 'part' AND current_quantity > 0 AND deleted_at IS NULL
             ORDER BY current_quantity * CASE WHEN average_cost > 0 THEN average_cost ELSE cost_price END DESC",
        )?;
        let values = value_stmt
            .query_map([], |row| row.get::<_, f64>(0))?
            .collect::<Result<Vec<_>>>()?;
        let total: f64 = values.iter().sum();
        let mut cumulative = 0.0;
        let mut groups = [0.0_f64; 3];
        let mut counts = [0_i32; 3];
        for value in values {
            cumulative += value;
            let share = if total > 0.0 { cumulative / total } else { 1.0 };
            let index = if share <= 0.80 {
                0
            } else if share <= 0.95 {
                1
            } else {
                2
            };
            groups[index] += value;
            counts[index] += 1;
        }
        Ok(InventoryInsights {
            inactive_items,
            abc_groups: ["A", "B", "C"]
                .into_iter()
                .enumerate()
                .map(|(index, classification)| AbcInventoryGroup {
                    classification: classification.to_string(),
                    item_count: counts[index],
                    inventory_value: groups[index],
                })
                .collect(),
        })
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
        assert!(InventoryRepository::get_all_with_conn(&conn)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn add_stock_updates_quantity_and_logs_movement() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        InventoryRepository::add_stock_with_details_with_conn(&conn, &item.id, 3, None, None)
            .unwrap();

        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &item.id).unwrap();

        assert_eq!(fetched.current_quantity, 8);
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].r#type, "entrada");
        assert_eq!(movements[0].quantity, 3);
        assert_eq!(movements[0].unit_cost, Some(50.0));
    }

    #[test]
    fn restock_calculates_weighted_average_cost_and_logs_effective_cost() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        InventoryRepository::add_stock_with_details_with_conn(
            &conn,
            &item.id,
            5,
            Some(80.0),
            Some("supplier_invoice".to_string()),
        )
        .unwrap();

        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();
        let movement = InventoryRepository::get_movements_with_conn(&conn, &item.id)
            .unwrap()
            .remove(0);
        assert!((fetched.average_cost - 65.0).abs() < f64::EPSILON);
        assert_eq!(movement.unit_cost, Some(80.0));
        assert_eq!(movement.reason, "supplier_invoice");
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
    fn remove_stock_rejects_quantity_above_available_stock() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        let result = InventoryRepository::remove_stock_with_conn(&conn, &item.id, 99);

        let fetched = InventoryRepository::get_by_id_with_conn(&conn, &item.id)
            .unwrap()
            .unwrap();
        let movements = InventoryRepository::get_movements_with_conn(&conn, &item.id).unwrap();

        assert!(result.is_err());
        assert_eq!(fetched.current_quantity, 5);
        assert!(movements.is_empty());
    }

    #[test]
    fn stock_changes_reject_non_positive_quantities() {
        let conn = setup_db();
        let item = sample_item();
        InventoryRepository::create_with_conn(&conn, &item).unwrap();

        assert!(InventoryRepository::add_stock_with_details_with_conn(
            &conn, &item.id, 0, None, None
        )
        .is_err());
        assert!(InventoryRepository::remove_stock_with_conn(&conn, &item.id, -1).is_err());
    }

    #[test]
    fn insights_exclude_deleted_services_and_classify_active_stock_by_value() {
        let conn = setup_db();
        for (id, name, quantity, average_cost) in [
            ("a", "A", 7, 10.0),
            ("b", "B", 2, 10.0),
            ("c", "C", 1, 10.0),
        ] {
            conn.execute(
                "INSERT INTO inventory_items (id, name, type, current_quantity, cost_price, average_cost, created_at) VALUES (?1, ?2, 'part', ?3, 1, ?4, datetime('now', '-100 days'))",
                params![id, name, quantity, average_cost],
            ).unwrap();
        }
        conn.execute("INSERT INTO inventory_items (id, name, type, current_quantity, cost_price, created_at, deleted_at) VALUES ('service', 'Servico', 'service', 99, 99, datetime('now', '-100 days'), NULL)", []).unwrap();
        conn.execute("INSERT INTO inventory_items (id, name, type, current_quantity, cost_price, created_at, deleted_at) VALUES ('deleted', 'Excluida', 'part', 99, 99, datetime('now', '-100 days'), datetime('now'))", []).unwrap();

        let insights = InventoryRepository::get_insights_with_conn(&conn, 90).unwrap();

        assert_eq!(insights.inactive_items.len(), 3);
        assert_eq!(insights.abc_groups[0].classification, "A");
        assert_eq!(insights.abc_groups[0].item_count, 1);
        assert_eq!(insights.abc_groups[1].item_count, 1);
        assert_eq!(insights.abc_groups[2].item_count, 1);
        assert_eq!(insights.abc_groups[0].inventory_value, 70.0);
    }
}
