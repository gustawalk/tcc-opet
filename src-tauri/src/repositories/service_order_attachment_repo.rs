use crate::database::get_db;
use crate::models::service_order_attachment::ServiceOrderAttachment;
use rusqlite::{params, Connection, Result};

pub struct ServiceOrderAttachmentRepository;

impl ServiceOrderAttachmentRepository {
    pub(crate) fn create_with_conn(
        conn: &Connection,
        attachment: &ServiceOrderAttachment,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO service_order_attachments (id, service_order_id, file_name, storage_name, mime_type, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                attachment.id,
                attachment.service_order_id,
                attachment.file_name,
                attachment.storage_name,
                attachment.mime_type,
                attachment.size_bytes,
                attachment.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<ServiceOrderAttachment>> {
        let conn = get_db()?;
        Self::get_by_id_with_conn(&conn, id)
    }

    pub(crate) fn get_by_id_with_conn(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<ServiceOrderAttachment>> {
        let mut stmt = conn.prepare(
            "SELECT id, service_order_id, file_name, storage_name, mime_type, size_bytes, created_at
             FROM service_order_attachments WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], map_attachment)?;
        rows.next().transpose()
    }

    pub fn get_by_service_order_id(service_order_id: &str) -> Result<Vec<ServiceOrderAttachment>> {
        let conn = get_db()?;
        Self::get_by_service_order_id_with_conn(&conn, service_order_id)
    }

    pub(crate) fn get_by_service_order_id_with_conn(
        conn: &Connection,
        service_order_id: &str,
    ) -> Result<Vec<ServiceOrderAttachment>> {
        let mut stmt = conn.prepare(
            "SELECT id, service_order_id, file_name, storage_name, mime_type, size_bytes, created_at
             FROM service_order_attachments
             WHERE service_order_id = ?1
             ORDER BY created_at DESC",
        )?;
        let attachments = stmt
            .query_map(params![service_order_id], map_attachment)?
            .collect();
        attachments
    }

    pub(crate) fn delete_with_conn(conn: &Connection, id: &str) -> Result<ServiceOrderAttachment> {
        let attachment =
            Self::get_by_id_with_conn(conn, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        conn.execute(
            "DELETE FROM service_order_attachments WHERE id = ?1",
            params![id],
        )?;
        Ok(attachment)
    }
}

fn map_attachment(row: &rusqlite::Row<'_>) -> Result<ServiceOrderAttachment> {
    Ok(ServiceOrderAttachment {
        id: row.get(0)?,
        service_order_id: row.get(1)?,
        file_name: row.get(2)?,
        storage_name: row.get(3)?,
        mime_type: row.get(4)?,
        size_bytes: row.get(5)?,
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::customer::Customer;
    use crate::models::service_order::ServiceOrder;
    use crate::repositories::customer_repo::CustomerRepository;
    use crate::repositories::service_order_repo::ServiceOrderRepository;
    use crate::test_helpers::setup_db;

    #[test]
    fn stores_and_deletes_service_order_attachments() {
        let conn = setup_db();
        let customer = Customer::new(
            "Ana".to_string(),
            "41999999999".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let mut order = ServiceOrder::new(customer.id, "iPhone".to_string(), "Falha".to_string());
        ServiceOrderRepository::create_with_conn(&conn, &mut order).unwrap();
        let attachment = ServiceOrderAttachment::new(
            order.id.clone(),
            "entrada.jpg".to_string(),
            "uuid.jpg".to_string(),
            "image/jpeg".to_string(),
            128,
        );

        ServiceOrderAttachmentRepository::create_with_conn(&conn, &attachment).unwrap();
        assert_eq!(
            ServiceOrderAttachmentRepository::get_by_service_order_id_with_conn(&conn, &order.id)
                .unwrap()
                .len(),
            1
        );
        let deleted =
            ServiceOrderAttachmentRepository::delete_with_conn(&conn, &attachment.id).unwrap();
        assert_eq!(deleted.file_name, "entrada.jpg");
    }
}
