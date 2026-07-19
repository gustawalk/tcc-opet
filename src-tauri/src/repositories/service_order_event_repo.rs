use crate::database::get_db;
use crate::models::service_order_event::ServiceOrderEvent;
use rusqlite::{params, Connection, Result};

pub struct ServiceOrderEventRepository;

impl ServiceOrderEventRepository {
    pub(crate) fn create_with_conn(conn: &Connection, event: &ServiceOrderEvent) -> Result<()> {
        conn.execute(
            "INSERT INTO service_order_events (id, service_order_id, event_type, details, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.id,
                event.service_order_id,
                event.event_type,
                event.details,
                event.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_by_service_order_id(service_order_id: &str) -> Result<Vec<ServiceOrderEvent>> {
        let conn = get_db()?;
        Self::get_by_service_order_id_with_conn(&conn, service_order_id)
    }

    pub(crate) fn get_by_service_order_id_with_conn(
        conn: &Connection,
        service_order_id: &str,
    ) -> Result<Vec<ServiceOrderEvent>> {
        let mut stmt = conn.prepare(
            "SELECT id, service_order_id, event_type, details, created_at
             FROM service_order_events
             WHERE service_order_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![service_order_id], |row| {
            Ok(ServiceOrderEvent {
                id: row.get(0)?,
                service_order_id: row.get(1)?,
                event_type: row.get(2)?,
                details: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        rows.collect()
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

    #[test]
    fn stores_events_in_reverse_chronological_order() {
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

        let mut first =
            ServiceOrderEvent::new(order.id.clone(), "first".to_string(), "{}".to_string());
        first.created_at = "2099-01-01T10:00:00Z".to_string();
        let mut second =
            ServiceOrderEvent::new(order.id.clone(), "second".to_string(), "{}".to_string());
        second.created_at = "2099-01-01T10:01:00Z".to_string();
        ServiceOrderEventRepository::create_with_conn(&conn, &first).unwrap();
        ServiceOrderEventRepository::create_with_conn(&conn, &second).unwrap();

        let events =
            ServiceOrderEventRepository::get_by_service_order_id_with_conn(&conn, &order.id)
                .unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "second");
    }
}
