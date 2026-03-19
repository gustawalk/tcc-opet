use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub created_at: Option<String>,
    pub deleted_at: Option<String>,
}

impl Customer {
    pub fn new(name: String, phone: String, email: String, address: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            phone,
            email,
            address,
            created_at: Some(chrono::Utc::now().to_rfc3339()),
            deleted_at: None,
        }
    }
}
