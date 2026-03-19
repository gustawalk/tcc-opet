use crate::database::get_db;
use crate::models::user::User;
use chrono::Utc;
use rusqlite::{params, Result};

pub struct UserRepository;

impl UserRepository {
    pub fn create(user: &User) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "INSERT INTO users (id, name, email, password, role, created_at, deleted_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user.id,
                user.name,
                user.email,
                user.password,
                user.role,
                user.created_at,
                user.deleted_at
            ],
        )?;
        Ok(())
    }

    pub fn get_by_id(id: &str) -> Result<Option<User>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, email, password, role, created_at, deleted_at 
             FROM users WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query_map(params![id], |row: &rusqlite::Row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                password: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        })?;

        let user = rows.next().transpose()?;
        Ok(user)
    }

    pub fn get_by_email(email: &str) -> Result<Option<User>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, email, password, role, created_at, deleted_at 
             FROM users WHERE email = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query_map(params![email], |row: &rusqlite::Row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                password: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
                deleted_at: row.get(6)?,
            })
        })?;

        let user = rows.next().transpose()?;
        Ok(user)
    }

    pub fn get_all() -> Result<Vec<User>> {
        let conn = get_db()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, email, password, role, created_at, deleted_at 
             FROM users WHERE deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(params![], |row: &rusqlite::Row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                password: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
                deleted_at: row.get(6)?,
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

        conn.execute(
            "UPDATE users 
             SET name = ?1, email = ?2, role = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                user.name,
                user.email,
                user.role,
                Utc::now().to_rfc3339(),
                user.id
            ],
        )?;
        Ok(())
    }

    pub fn delete(id: &str) -> Result<()> {
        let conn = get_db()?;

        conn.execute(
            "UPDATE users SET deleted_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn reset_password(id: &str, new_password: &str) -> Result<()> {
        let conn = get_db()?;
        conn.execute(
            "UPDATE users SET password = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_password, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }
}
