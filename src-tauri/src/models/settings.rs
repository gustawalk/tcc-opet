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
