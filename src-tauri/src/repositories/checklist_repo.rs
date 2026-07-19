use crate::database::get_db;
use crate::models::checklist::{ChecklistItem, ChecklistTemplate};
use crate::models::service_order_event::ServiceOrderEvent;
use crate::repositories::service_order_event_repo::ServiceOrderEventRepository;
use rusqlite::{params, Connection, Result, Transaction};
use std::collections::HashMap;
use uuid::Uuid;

pub struct ChecklistRepository;

impl ChecklistRepository {
    pub fn create_template(title: &str, items: Vec<String>) -> Result<String> {
        let mut conn = get_db()?;
        Self::create_template_with_conn(&mut conn, title, items)
    }

    pub(crate) fn create_template_with_conn(
        conn: &mut Connection,
        title: &str,
        items: Vec<String>,
    ) -> Result<String> {
        let tx = conn.transaction()?;

        let template = ChecklistTemplate::new(title.to_string());

        tx.execute(
            "INSERT INTO checklist_templates (id, title, created_at) VALUES (?1, ?2, ?3)",
            params![template.id, template.title, template.created_at],
        )?;

        for item_label in items {
            tx.execute(
                "INSERT INTO template_items (id, template_id, label) VALUES (?1, ?2, ?3)",
                params![Uuid::new_v4().to_string(), template.id, item_label],
            )?;
        }

        tx.commit()?;
        Ok(template.id)
    }

    pub fn get_templates() -> Result<Vec<ChecklistTemplate>> {
        let conn = get_db()?;
        Self::get_templates_with_conn(&conn)
    }

    pub(crate) fn get_templates_with_conn(conn: &Connection) -> Result<Vec<ChecklistTemplate>> {
        let mut stmt = conn.prepare("SELECT id, title, created_at FROM checklist_templates")?;
        let templates_raw: Vec<(String, String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut item_stmt = conn.prepare("SELECT template_id, label FROM template_items")?;
        let all_items: Vec<(String, String)> = item_stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut items_map: HashMap<String, Vec<String>> = HashMap::new();
        for (tid, label) in all_items {
            items_map.entry(tid).or_default().push(label);
        }

        let templates = templates_raw
            .into_iter()
            .map(|(id, title, created_at)| ChecklistTemplate {
                items: items_map.remove(&id),
                id,
                title,
                created_at,
            })
            .collect();

        Ok(templates)
    }

    pub fn get_template_items(template_id: &str) -> Result<Vec<String>> {
        let conn = get_db()?;
        Self::get_template_items_with_conn(&conn, template_id)
    }

    pub(crate) fn get_template_items_with_conn(
        conn: &Connection,
        template_id: &str,
    ) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT label FROM template_items WHERE template_id = ?1")?;

        let rows = stmt.query_map(params![template_id], |row| row.get::<_, String>(0))?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn delete_template(id: &str) -> Result<()> {
        let conn = get_db()?;
        Self::delete_template_with_conn(&conn, id)
    }

    pub(crate) fn delete_template_with_conn(conn: &Connection, id: &str) -> Result<()> {
        // Foreign key with ON DELETE CASCADE will handle template_items
        conn.execute("DELETE FROM checklist_templates WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_template(id: &str, title: &str, items: Vec<String>) -> Result<()> {
        let mut conn = get_db()?;
        Self::update_template_with_conn(&mut conn, id, title, items)
    }

    pub(crate) fn update_template_with_conn(
        conn: &mut Connection,
        id: &str,
        title: &str,
        items: Vec<String>,
    ) -> Result<()> {
        let tx = conn.transaction()?;

        tx.execute(
            "UPDATE checklist_templates SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;

        // Delete old items and insert new ones
        tx.execute(
            "DELETE FROM template_items WHERE template_id = ?1",
            params![id],
        )?;

        for item_label in items {
            tx.execute(
                "INSERT INTO template_items (id, template_id, label) VALUES (?1, ?2, ?3)",
                params![Uuid::new_v4().to_string(), id, item_label],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn save_os_checklist(os_id: &str, items: Vec<ChecklistItem>) -> Result<()> {
        let mut conn = get_db()?;
        Self::save_os_checklist_with_conn(&mut conn, os_id, items)
    }

    pub(crate) fn save_os_checklist_with_conn(
        conn: &mut Connection,
        os_id: &str,
        items: Vec<ChecklistItem>,
    ) -> Result<()> {
        let tx = conn.transaction()?;
        Self::replace_os_checklist_in_transaction(&tx, os_id, items)?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn replace_os_checklist_in_transaction(
        tx: &Transaction<'_>,
        os_id: &str,
        items: Vec<ChecklistItem>,
    ) -> Result<()> {
        let item_count = items.len();

        // Clean existing checklist for this OS first
        tx.execute(
            "DELETE FROM service_order_checklists WHERE service_order_id = ?1",
            params![os_id],
        )?;

        for item in items {
            tx.execute(
                "INSERT INTO service_order_checklists (id, service_order_id, label, checked) VALUES (?1, ?2, ?3, ?4)",
                params![Uuid::new_v4().to_string(), os_id, item.label, item.checked],
            )?;
        }

        let event = ServiceOrderEvent::new(
            os_id.to_string(),
            "checklist_updated".to_string(),
            serde_json::json!({ "itemCount": item_count }).to_string(),
        );
        ServiceOrderEventRepository::create_with_conn(&tx, &event)?;

        Ok(())
    }

    pub fn get_os_checklist(os_id: &str) -> Result<Vec<ChecklistItem>> {
        let conn = get_db()?;
        Self::get_os_checklist_with_conn(&conn, os_id)
    }

    pub(crate) fn get_os_checklist_with_conn(
        conn: &Connection,
        os_id: &str,
    ) -> Result<Vec<ChecklistItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, label, checked FROM service_order_checklists WHERE service_order_id = ?1",
        )?;

        let rows = stmt.query_map(params![os_id], |row| {
            Ok(ChecklistItem {
                id: row.get(0)?,
                label: row.get(1)?,
                checked: row.get(2)?,
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::service_order::ServiceOrder;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::service_order_repo::ServiceOrderRepository;
    use crate::test_helpers::setup_db;

    fn seed_service_order(conn: &Connection) -> ServiceOrder {
        let customer = Customer::new(
            "Carlos".to_string(),
            "41911112222".to_string(),
            "carlos@example.com".to_string(),
            "Rua X".to_string(),
        );
        CustomerRepository::create_with_conn(conn, &customer).unwrap();

        let mut order = ServiceOrder::new(
            customer.id,
            "Galaxy S23".to_string(),
            "Avaliação inicial".to_string(),
        );
        ServiceOrderRepository::create_with_conn(conn, &mut order).unwrap();
        order
    }

    #[test]
    fn create_template_and_load_grouped_items() {
        let mut conn = setup_db();

        let template_id = ChecklistRepository::create_template_with_conn(
            &mut conn,
            "Recepção",
            vec!["Tela".to_string(), "Carcaça".to_string()],
        )
        .unwrap();

        let templates = ChecklistRepository::get_templates_with_conn(&conn).unwrap();
        let template = templates
            .into_iter()
            .find(|item| item.id == template_id)
            .unwrap();

        assert_eq!(template.title, "Recepção");
        assert_eq!(template.items.unwrap(), vec!["Tela", "Carcaça"]);
    }

    #[test]
    fn update_template_replaces_old_items() {
        let mut conn = setup_db();
        let template_id = ChecklistRepository::create_template_with_conn(
            &mut conn,
            "Recepção",
            vec!["Tela".to_string(), "Carcaça".to_string()],
        )
        .unwrap();

        ChecklistRepository::update_template_with_conn(
            &mut conn,
            &template_id,
            "Recepção revisada",
            vec!["Bateria".to_string()],
        )
        .unwrap();

        let items = ChecklistRepository::get_template_items_with_conn(&conn, &template_id).unwrap();
        assert_eq!(items, vec!["Bateria"]);
    }

    #[test]
    fn delete_template_removes_template_and_items() {
        let mut conn = setup_db();
        let template_id = ChecklistRepository::create_template_with_conn(
            &mut conn,
            "Recepção",
            vec!["Tela".to_string()],
        )
        .unwrap();

        ChecklistRepository::delete_template_with_conn(&conn, &template_id).unwrap();

        let templates = ChecklistRepository::get_templates_with_conn(&conn).unwrap();
        let items = ChecklistRepository::get_template_items_with_conn(&conn, &template_id).unwrap();
        assert!(templates.is_empty());
        assert!(items.is_empty());
    }

    #[test]
    fn save_os_checklist_replaces_previous_items() {
        let mut conn = setup_db();
        let order = seed_service_order(&conn);

        ChecklistRepository::save_os_checklist_with_conn(
            &mut conn,
            &order.id,
            vec![ChecklistItem {
                id: "1".to_string(),
                label: "Tela".to_string(),
                checked: true,
            }],
        )
        .unwrap();

        ChecklistRepository::save_os_checklist_with_conn(
            &mut conn,
            &order.id,
            vec![ChecklistItem {
                id: "2".to_string(),
                label: "Bateria".to_string(),
                checked: false,
            }],
        )
        .unwrap();

        let items = ChecklistRepository::get_os_checklist_with_conn(&conn, &order.id).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "Bateria");
        assert!(!items[0].checked);
    }
}
