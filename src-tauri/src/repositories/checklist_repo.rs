use crate::database::get_db;
use crate::models::checklist::{ChecklistTemplate, ChecklistItem};
use rusqlite::{params, Result};
use uuid::Uuid;

pub struct ChecklistRepository;

impl ChecklistRepository {
    pub fn create_template(title: &str, items: Vec<String>) -> Result<String> {
        let mut conn = get_db()?;
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
        let mut stmt = conn.prepare("SELECT id, title, created_at FROM checklist_templates")?;
        
        let rows = stmt.query_map([], |row| {
            Ok(ChecklistTemplate {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row?);
        }
        Ok(templates)
    }

    pub fn get_template_items(template_id: &str) -> Result<Vec<String>> {
        let conn = get_db()?;
        let mut stmt = conn.prepare("SELECT label FROM template_items WHERE template_id = ?1")?;
        
        let rows = stmt.query_map(params![template_id], |row| {
            row.get::<_, String>(0)
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn delete_template(id: &str) -> Result<()> {
        let conn = get_db()?;
        // Foreign key with ON DELETE CASCADE will handle template_items
        conn.execute("DELETE FROM checklist_templates WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_template(id: &str, title: &str, items: Vec<String>) -> Result<()> {
        let mut conn = get_db()?;
        let tx = conn.transaction()?;

        tx.execute(
            "UPDATE checklist_templates SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;

        // Delete old items and insert new ones
        tx.execute("DELETE FROM template_items WHERE template_id = ?1", params![id])?;

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
        let tx = conn.transaction()?;

        // Clean existing checklist for this OS first
        tx.execute("DELETE FROM service_order_checklists WHERE service_order_id = ?1", params![os_id])?;

        for item in items {
            tx.execute(
                "INSERT INTO service_order_checklists (id, service_order_id, label, checked) VALUES (?1, ?2, ?3, ?4)",
                params![Uuid::new_v4().to_string(), os_id, item.label, item.checked],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_os_checklist(os_id: &str) -> Result<Vec<ChecklistItem>> {
        let conn = get_db()?;
        let mut stmt = conn.prepare(
            "SELECT id, label, checked FROM service_order_checklists WHERE service_order_id = ?1"
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
