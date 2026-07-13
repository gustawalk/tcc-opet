use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub cpf: Option<String>,
    pub join_date: Option<String>,
    pub created_at: Option<String>,
    pub deleted_at: Option<String>,
}

impl User {
    pub fn new(name: String, email: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            email,
            phone: None,
            cpf: None,
            join_date: None,
            created_at: Some(Utc::now().to_rfc3339()),
            deleted_at: None,
        }
    }
}
