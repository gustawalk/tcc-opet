use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub id: i32,
    pub company_name: String,
    pub cnpj: Option<String>,
    pub logo_path: Option<String>,
    pub address: Option<String>,
}

impl Settings {
    pub fn default() -> Self {
        Self {
            id: 1,
            company_name: "Minha Empresa".to_string(),
            cnpj: None,
            logo_path: None,
            address: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_returns_singleton_settings() {
        let settings = Settings::default();

        assert_eq!(settings.id, 1);
        assert_eq!(settings.company_name, "Minha Empresa");
        assert!(settings.cnpj.is_none());
        assert!(settings.logo_path.is_none());
        assert!(settings.address.is_none());
    }
}
