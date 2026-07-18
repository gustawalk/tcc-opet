use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistTemplate {
    pub id: String,
    pub title: String,
    pub items: Option<Vec<String>>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: String,
    pub label: String,
    pub checked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ServiceOrderChecklist {
    pub service_order_id: String,
    pub checklist_item_id: String,
}

impl ChecklistTemplate {
    pub fn new(title: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            items: None,
            created_at: Some(Utc::now().to_rfc3339()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_constructor_sets_defaults() {
        let template = ChecklistTemplate::new("Inspeção Inicial".to_string());

        assert!(Uuid::parse_str(&template.id).is_ok());
        assert_eq!(template.title, "Inspeção Inicial");
        assert!(template.items.is_none());
        assert!(template.created_at.is_some());
    }
}
