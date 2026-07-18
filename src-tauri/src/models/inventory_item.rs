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
    pub updated_at: Option<String>,
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
            updated_at: None,
            deleted_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_preserves_inventory_fields() {
        let item = InventoryItem::new(
            "Tela".to_string(),
            "Tela OLED".to_string(),
            "part".to_string(),
            2,
            10,
            50.0,
            120.0,
        );

        assert!(Uuid::parse_str(&item.id).is_ok());
        assert_eq!(item.r#type, "part");
        assert_eq!(item.min_quantity, 2);
        assert_eq!(item.current_quantity, 10);
        assert_eq!(item.cost_price, 50.0);
        assert_eq!(item.sale_price, 120.0);
        assert!(item.created_at.is_some());
        assert!(item.deleted_at.is_none());
    }
}
