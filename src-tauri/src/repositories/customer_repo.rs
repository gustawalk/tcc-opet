use crate::database::get_db;
use crate::models::customer::Customer;
use crate::page::like_search_clause;
use chrono::Utc;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Result};

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

    pub(crate) fn get_page_with_conn(
        conn: &Connection,
        limit: u32,
        offset: u32,
        search: &str,
    ) -> Result<Vec<Customer>> {
        let (clause, patterns) =
            like_search_clause(search, &["name", "email", "phone", "COALESCE(address, '')"]);
        let sql = format!(
            "SELECT id, name, phone, email, address, created_at, deleted_at
             FROM customers WHERE deleted_at IS NULL{clause}
             ORDER BY created_at DESC, id DESC
             LIMIT ? OFFSET ?"
        );
        let mut values: Vec<Value> = Vec::with_capacity(patterns.len() + 2);
        for pattern in patterns {
            values.push(pattern.into());
        }
        values.push((limit as i64).into());
        values.push((offset as i64).into());
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values), |row: &rusqlite::Row| {
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

    pub(crate) fn count_all_with_conn(conn: &Connection, search: &str) -> Result<i64> {
        let (clause, patterns) =
            like_search_clause(search, &["name", "email", "phone", "COALESCE(address, '')"]);
        let sql = format!("SELECT COUNT(*) FROM customers WHERE deleted_at IS NULL{clause}");
        let values: Vec<Value> = patterns.into_iter().map(Value::from).collect();
        conn.query_row(&sql, params_from_iter(values), |row| row.get(0))
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

    #[test]
    fn page_search_filters_by_substring() {
        let conn = setup_db();
        let ana = Customer::new(
            "Ana Silva".to_string(),
            "41911111111".to_string(),
            "ana@example.com".to_string(),
            "Rua A".to_string(),
        );
        let bruno = Customer::new(
            "Bruno Souza".to_string(),
            "41922222222".to_string(),
            "bruno@example.com".to_string(),
            "Rua B".to_string(),
        );
        CustomerRepository::create_with_conn(&conn, &ana).unwrap();
        CustomerRepository::create_with_conn(&conn, &bruno).unwrap();

        let page = CustomerRepository::get_page_with_conn(&conn, 10, 0, "souza").unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].name, "Bruno Souza");

        let phone_match = CustomerRepository::get_page_with_conn(&conn, 10, 0, "1111").unwrap();
        assert_eq!(phone_match.len(), 1);
        assert_eq!(phone_match[0].name, "Ana Silva");

        assert_eq!(
            CustomerRepository::count_all_with_conn(&conn, "").unwrap(),
            2
        );
        assert_eq!(
            CustomerRepository::count_all_with_conn(&conn, "inexistente").unwrap(),
            0
        );
    }
}
