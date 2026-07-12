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
            created_at: Some(Utc::now().to_rfc3339()),
        }
    }
}
