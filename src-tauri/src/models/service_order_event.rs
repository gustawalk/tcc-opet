use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOrderEvent {
    pub id: String,
    pub service_order_id: String,
    pub event_type: String,
    pub details: String,
    pub created_at: String,
}

impl ServiceOrderEvent {
    pub fn new(service_order_id: String, event_type: String, details: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            service_order_id,
            event_type,
            details,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_creates_timestamped_event() {
        let event =
            ServiceOrderEvent::new("os-1".to_string(), "created".to_string(), "{}".to_string());

        assert!(Uuid::parse_str(&event.id).is_ok());
        assert_eq!(event.service_order_id, "os-1");
        assert_eq!(event.event_type, "created");
        assert!(!event.created_at.is_empty());
    }
}
