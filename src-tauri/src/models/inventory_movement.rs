use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryMovement {
    pub id: String,
    pub inventory_item_id: String,
    pub r#type: String, // 'entrada' or 'saida'
    pub quantity: i32,
    pub reference_os_id: Option<String>,
    pub os_display_id: Option<String>,
    pub created_at: Option<String>,
}

impl InventoryMovement {
    pub fn new(
        inventory_item_id: String,
        r#type: String,
        quantity: i32,
        reference_os_id: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            inventory_item_id,
            r#type,
            quantity,
            reference_os_id,
            os_display_id: None,
            created_at: Some(Utc::now().to_rfc3339()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_sets_reference_and_defaults() {
        let movement = InventoryMovement::new(
            "item-1".to_string(),
            "saida".to_string(),
            3,
            Some("os-1".to_string()),
        );

        assert!(Uuid::parse_str(&movement.id).is_ok());
        assert_eq!(movement.inventory_item_id, "item-1");
        assert_eq!(movement.quantity, 3);
        assert_eq!(movement.reference_os_id.as_deref(), Some("os-1"));
        assert!(movement.os_display_id.is_none());
        assert!(movement.created_at.is_some());
    }
}
