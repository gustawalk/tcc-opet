use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    pub role: String, // admin or tech
    pub created_at: Option<String>,
    pub deleted_at: Option<String>,
}

impl User {
    pub fn new(name: String, email: String, role: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            email,
            password: Some("123456".to_string()),
            role,
            created_at: Some(Utc::now().to_rfc3339()),
            deleted_at: None,
        }
    }
}
