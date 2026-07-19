use crate::database::get_db;
use crate::models::customer::Customer;
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct CustomerRepository;

impl CustomerRepository {
    pub fn create(customer: &Customer) -> Result<()> {
        let conn = get_db()?;
        Self::create_with_conn(&conn, customer)
    }

    pub(crate) fn create_with_conn(conn: &Connection, customer: &Customer) -> Result<()> {
        conn.execute(
            "INSERT INTO customers (id, name, phone, email, address, created_at, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                customer.id,
                customer.name,
                customer.phone,
                customer.email,
                customer.address,
                customer.created_at,
                customer.deleted_at
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<Customer>> {
        let conn = get_db()?;
        Self::get_by_id_with_conn(&conn, id)
    }

    pub(crate) fn get_by_id_with_conn(conn: &Connection, id: &str) -> Result<Option<Customer>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, phone, email, address, created_at, deleted_at 
             FROM customers WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(Customer {
                id: row.get(0)?,
                name: row.get(1)?,
                phone: row.get(2)?,
                email: row.get(3)?,
                address: row.get(4)?,
                created_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        })?;

        let customer = rows.next().transpose()?;
        Ok(customer)
    }

    pub fn get_all() -> Result<Vec<Customer>> {
        let conn = get_db()?;
        Self::get_all_with_conn(&conn)
    }

    pub(crate) fn get_all_with_conn(conn: &Connection) -> Result<Vec<Customer>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, phone, email, address, created_at, deleted_at 
             FROM customers WHERE deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(Customer {
                id: row.get(0)?,
                name: row.get(1)?,
                phone: row.get(2)?,
                email: row.get(3)?,
                address: row.get(4)?,
                created_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        })?;

        let mut customers = Vec::new();
        for row in rows {
            customers.push(row?);
        }
        Ok(customers)
    }

    pub fn update(customer: &Customer) -> Result<()> {
        let conn = get_db()?;
        Self::update_with_conn(&conn, customer)
    }

    pub(crate) fn update_with_conn(conn: &Connection, customer: &Customer) -> Result<()> {
        let updated = conn.execute(
            "UPDATE customers 
             SET name = ?1, phone = ?2, email = ?3, address = ?4, updated_at = ?5
              WHERE id = ?6 AND deleted_at IS NULL",
            params![
                customer.name,
                customer.phone,
                customer.email,
                customer.address,
                Utc::now().to_rfc3339(),
                customer.id
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
            "UPDATE customers SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id],
        )?;
        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_db;

    fn sample_customer() -> Customer {
        Customer::new(
            "Maria".to_string(),
            "41999999999".to_string(),
            "maria@example.com".to_string(),
            "Rua A, 123".to_string(),
        )
    }

    #[test]
    fn create_and_get_customer() {
        let conn = setup_db();
        let customer = sample_customer();

        CustomerRepository::create_with_conn(&conn, &customer).unwrap();
        let fetched = CustomerRepository::get_by_id_with_conn(&conn, &customer.id).unwrap();

        assert_eq!(fetched.unwrap().email, "maria@example.com");
    }

    #[test]
    fn update_customer_persists_changes() {
        let conn = setup_db();
        let mut customer = sample_customer();
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();

        customer.name = "Maria Silva".to_string();
        customer.address = "Rua B, 999".to_string();
        CustomerRepository::update_with_conn(&conn, &customer).unwrap();

        let fetched = CustomerRepository::get_by_id_with_conn(&conn, &customer.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.name, "Maria Silva");
        assert_eq!(fetched.address, "Rua B, 999");
    }

    #[test]
    fn delete_customer_soft_deletes_record() {
        let conn = setup_db();
        let customer = sample_customer();
        CustomerRepository::create_with_conn(&conn, &customer).unwrap();

        CustomerRepository::delete_with_conn(&conn, &customer.id).unwrap();

        assert!(CustomerRepository::get_by_id_with_conn(&conn, &customer.id)
            .unwrap()
            .is_none());
        assert!(CustomerRepository::get_all_with_conn(&conn)
            .unwrap()
            .is_empty());
    }
}
