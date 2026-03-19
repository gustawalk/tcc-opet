use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub r#type: String, // 'part' or 'service'
    pub min_quantity: i32,
    pub current_quantity: i32,
    pub cost_price: f64,
    pub sale_price: f64,
    pub created_at: Option<String>,
    pub deleted_at: Option<String>,
}

impl InventoryItem {
    pub fn new(
        name: String,
        description: String,
        r#type: String,
        min_quantity: i32,
        current_quantity: i32,
        cost_price: f64,
        sale_price: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            r#type,
            min_quantity,
            current_quantity,
            cost_price,
            sale_price,
            created_at: Some(Utc::now().to_rfc3339()),
            deleted_at: None,
        }
    }
}
