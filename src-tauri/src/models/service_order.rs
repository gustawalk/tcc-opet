use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOrder {
    pub id: String,
    pub customer_id: String,
    pub customer_name: Option<String>,
    pub user_id: Option<String>, // Technician ID
    pub user_name: Option<String>,
    pub equipment: String,
    pub imei: Option<String>,
    pub description: String,
    pub status: String, // OSStatus enum
    pub total_price: Option<f64>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub closed_at: Option<String>,
    pub display_id: String,
    pub discount_percent: f64,
}

impl ServiceOrder {
    pub fn new(customer_id: String, equipment: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            customer_id,
            customer_name: None,
            user_id: None,
            user_name: None,
            equipment,
            imei: None,
            description,
            status: "Orçamento".to_string(),
            total_price: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: None,
            closed_at: None,
            display_id: String::new(),
            discount_percent: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_sets_service_order_defaults() {
        let order = ServiceOrder::new(
            "customer-1".to_string(),
            "iPhone 14".to_string(),
            "Troca de bateria".to_string(),
        );

        assert!(Uuid::parse_str(&order.id).is_ok());
        assert_eq!(order.status, "Orçamento");
        assert!(order.total_price.is_none());
        assert!(order.display_id.is_empty());
        assert_eq!(order.discount_percent, 0.0);
        assert!(order.closed_at.is_none());
    }
}
