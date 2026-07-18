use crate::database::get_db;
use crate::models::user::User;
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct UserRepository;

impl UserRepository {
    pub fn create(user: &User) -> Result<()> {
        let conn = get_db()?;
        Self::create_with_conn(&conn, user)
    }

    pub(crate) fn create_with_conn(conn: &Connection, user: &User) -> Result<()> {

        conn.execute(
            "INSERT INTO users (id, name, email, phone, cpf, join_date, created_at, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                user.id,
                user.name,
                user.email,
                user.phone,
                user.cpf,
                user.join_date,
                user.created_at,
                user.deleted_at
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<User>> {
        let conn = get_db()?;
        Self::get_by_id_with_conn(&conn, id)
    }

    pub(crate) fn get_by_id_with_conn(conn: &Connection, id: &str) -> Result<Option<User>> {

        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, cpf, join_date, created_at, deleted_at 
             FROM users WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                phone: row.get(3)?,
                cpf: row.get(4)?,
                join_date: row.get(5)?,
                created_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        })?;

        let user = rows.next().transpose()?;
        Ok(user)
    }

    pub fn get_by_email(email: &str) -> Result<Option<User>> {
        let conn = get_db()?;
        Self::get_by_email_with_conn(&conn, email)
    }

    pub(crate) fn get_by_email_with_conn(conn: &Connection, email: &str) -> Result<Option<User>> {

        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, cpf, join_date, created_at, deleted_at 
             FROM users WHERE email = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query_map(params![email], |row: &rusqlite::Row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                phone: row.get(3)?,
                cpf: row.get(4)?,
                join_date: row.get(5)?,
                created_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        })?;

        let user = rows.next().transpose()?;
        Ok(user)
    }

    pub fn get_all() -> Result<Vec<User>> {
        let conn = get_db()?;
        Self::get_all_with_conn(&conn)
    }

    pub(crate) fn get_all_with_conn(conn: &Connection) -> Result<Vec<User>> {

        let mut stmt = conn.prepare(
            "SELECT id, name, email, phone, cpf, join_date, created_at, deleted_at 
             FROM users WHERE deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                phone: row.get(3)?,
                cpf: row.get(4)?,
                join_date: row.get(5)?,
                created_at: row.get(6)?,
                deleted_at: row.get(7)?,
            })
        })?;

        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }

    pub fn update(user: &User) -> Result<()> {
        let conn = get_db()?;
        Self::update_with_conn(&conn, user)
    }

    pub(crate) fn update_with_conn(conn: &Connection, user: &User) -> Result<()> {

        conn.execute(
            "UPDATE users 
             SET name = ?1, email = ?2, phone = ?3, cpf = ?4, join_date = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                user.name,
                user.email,
                user.phone,
                user.cpf,
                user.join_date,
                Utc::now().to_rfc3339(),
                user.id
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
            "UPDATE users SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_db;

    fn sample_user(email: &str) -> User {
        let mut user = User::new("João".to_string(), email.to_string());
        user.phone = Some("41988887777".to_string());
        user.cpf = Some("12345678900".to_string());
        user.join_date = Some("2026-01-10".to_string());
        user
    }

    #[test]
    fn create_and_get_user_by_email() {
        let conn = setup_db();
        let user = sample_user("joao@example.com");

        UserRepository::create_with_conn(&conn, &user).unwrap();
        let fetched = UserRepository::get_by_email_with_conn(&conn, "joao@example.com")
            .unwrap()
            .unwrap();

        assert_eq!(fetched.id, user.id);
        assert_eq!(fetched.phone.as_deref(), Some("41988887777"));
    }

    #[test]
    fn update_user_persists_optional_fields() {
        let conn = setup_db();
        let mut user = sample_user("joao@example.com");
        UserRepository::create_with_conn(&conn, &user).unwrap();

        user.name = "João Silva".to_string();
        user.phone = Some("41977776666".to_string());
        UserRepository::update_with_conn(&conn, &user).unwrap();

        let fetched = UserRepository::get_by_id_with_conn(&conn, &user.id)
            .unwrap()
            .unwrap();
        assert_eq!(fetched.name, "João Silva");
        assert_eq!(fetched.phone.as_deref(), Some("41977776666"));
    }

    #[test]
    fn delete_user_soft_deletes_record() {
        let conn = setup_db();
        let user = sample_user("joao@example.com");
        UserRepository::create_with_conn(&conn, &user).unwrap();

        UserRepository::delete_with_conn(&conn, &user.id).unwrap();

        assert!(UserRepository::get_by_id_with_conn(&conn, &user.id)
            .unwrap()
            .is_none());
        assert!(UserRepository::get_all_with_conn(&conn).unwrap().is_empty());
    }

    #[test]
    fn create_user_rejects_duplicate_email() {
        let conn = setup_db();
        let first = sample_user("duplicated@example.com");
        let second = sample_user("duplicated@example.com");

        UserRepository::create_with_conn(&conn, &first).unwrap();

        assert!(UserRepository::create_with_conn(&conn, &second).is_err());
    }
}
