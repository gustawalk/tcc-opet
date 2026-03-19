use crate::database::get_db;
use crate::models::settings::Settings;
use rusqlite::{params, Result};

pub struct SettingsRepository;

impl SettingsRepository {
    pub fn get_settings() -> Result<Settings> {
        let conn = get_db()?;
        
        let mut stmt = conn.prepare(
            "SELECT id, company_name, cnpj, logo_path, address FROM settings WHERE id = 1"
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
