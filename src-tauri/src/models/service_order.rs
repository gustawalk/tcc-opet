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
    pub equipment: String,
    pub imei: Option<String>,
    pub description: String,
    pub status: String, // OSStatus enum
    pub total_price: Option<f64>,
    pub signature_path: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub closed_at: Option<String>,
}

impl ServiceOrder {
    pub fn new(customer_id: String, equipment: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            customer_id,
            customer_name: None,
            user_id: None,
            equipment,
            imei: None,
            description,
            status: "Orçamento".to_string(),
            total_price: None,
            signature_path: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: None,
            closed_at: None,
        }
    }
}
