use crate::database::get_db;
use crate::models::customer::Customer;
use chrono::Utc;
use rusqlite::{params, Result};

pub struct CustomerRepository;

impl CustomerRepository {
    pub fn create(customer: &Customer) -> Result<()> {
        let conn = get_db()?;

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

        conn.execute(
            "UPDATE customers 
             SET name = ?1, phone = ?2, email = ?3, address = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                customer.name,
                customer.phone,
                customer.email,
                customer.address,
                Utc::now().to_rfc3339(),
                customer.id
            ],
        )?;
        Ok(())
    }

    pub fn delete(id: &str) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "UPDATE customers SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}
