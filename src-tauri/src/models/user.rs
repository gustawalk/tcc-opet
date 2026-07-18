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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_sets_optional_fields_to_none() {
        let user = User::new("João".to_string(), "joao@example.com".to_string());

        assert!(Uuid::parse_str(&user.id).is_ok());
        assert_eq!(user.name, "João");
        assert!(user.phone.is_none());
        assert!(user.cpf.is_none());
        assert!(user.join_date.is_none());
        assert!(user.created_at.is_some());
        assert!(user.deleted_at.is_none());
    }
}
