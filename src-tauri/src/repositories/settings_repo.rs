use crate::database::get_db;
use crate::models::settings::Settings;
use rusqlite::{params, Connection, Result};

pub struct SettingsRepository;

impl SettingsRepository {
    pub fn get_settings() -> Result<Settings> {
        let conn = get_db()?;
        Self::get_settings_with_conn(&conn)
    }

    pub(crate) fn get_settings_with_conn(conn: &Connection) -> Result<Settings> {
        let mut stmt = conn.prepare(
            "SELECT id, company_name, cnpj, logo_path, address FROM settings WHERE id = 1",
        )?;

        let result = stmt.query_row([], |row| {
            Ok(Settings {
                id: row.get(0)?,
                company_name: row.get(1)?,
                cnpj: row.get(2)?,
                logo_path: row.get(3)?,
                address: row.get(4)?,
            })
        });

        match result {
            Ok(settings) => Ok(settings),
            Err(_) => Ok(Settings::default()),
        }
    }

    pub fn update_settings(settings: &Settings) -> Result<()> {
        let conn = get_db()?;
        Self::update_settings_with_conn(&conn, settings)
    }

    pub(crate) fn update_settings_with_conn(conn: &Connection, settings: &Settings) -> Result<()> {
        conn.execute(
            "UPDATE settings SET company_name = ?1, cnpj = ?2, logo_path = ?3, address = ?4 WHERE id = 1",
            params![
                settings.company_name,
                settings.cnpj,
                settings.logo_path,
                settings.address,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_db;

    #[test]
    fn get_settings_returns_singleton_row() {
        let conn = setup_db();

        let settings = SettingsRepository::get_settings_with_conn(&conn).unwrap();

        assert_eq!(settings.id, 1);
        assert_eq!(settings.company_name, "Minha Empresa");
    }

    #[test]
    fn get_settings_falls_back_to_default_when_row_is_missing() {
        let conn = setup_db();
        conn.execute("DELETE FROM settings WHERE id = 1", [])
            .unwrap();

        let settings = SettingsRepository::get_settings_with_conn(&conn).unwrap();

        assert_eq!(settings.id, 1);
        assert_eq!(settings.company_name, "Minha Empresa");
    }

    #[test]
    fn update_settings_persists_all_fields() {
        let conn = setup_db();
        let settings = Settings {
            id: 1,
            company_name: "Assistência Pro".to_string(),
            cnpj: Some("12345678000199".to_string()),
            logo_path: Some("data:image/png;base64,abc".to_string()),
            address: Some("Rua Central, 100".to_string()),
        };

        SettingsRepository::update_settings_with_conn(&conn, &settings).unwrap();
        let fetched = SettingsRepository::get_settings_with_conn(&conn).unwrap();

        assert_eq!(fetched.company_name, "Assistência Pro");
        assert_eq!(fetched.cnpj.as_deref(), Some("12345678000199"));
        assert_eq!(
            fetched.logo_path.as_deref(),
            Some("data:image/png;base64,abc")
        );
        assert_eq!(fetched.address.as_deref(), Some("Rua Central, 100"));
    }
}
