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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_sets_defaults_and_generates_uuid() {
        let customer = Customer::new(
            "Maria".to_string(),
            "41999999999".to_string(),
            "maria@example.com".to_string(),
            "Rua A, 123".to_string(),
        );

        assert!(Uuid::parse_str(&customer.id).is_ok());
        assert_eq!(customer.name, "Maria");
        assert!(customer.created_at.is_some());
        assert!(customer.deleted_at.is_none());
    }
}
