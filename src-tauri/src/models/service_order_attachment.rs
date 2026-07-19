use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOrderAttachment {
    pub id: String,
    pub service_order_id: String,
    pub file_name: String,
    pub storage_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

impl ServiceOrderAttachment {
    pub fn new(
        service_order_id: String,
        file_name: String,
        storage_name: String,
        mime_type: String,
        size_bytes: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            service_order_id,
            file_name,
            storage_name,
            mime_type,
            size_bytes,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_preserves_attachment_metadata() {
        let attachment = ServiceOrderAttachment::new(
            "os-1".to_string(),
            "entrada.jpg".to_string(),
            "uuid.jpg".to_string(),
            "image/jpeg".to_string(),
            128,
        );

        assert!(Uuid::parse_str(&attachment.id).is_ok());
        assert_eq!(attachment.service_order_id, "os-1");
        assert_eq!(attachment.mime_type, "image/jpeg");
        assert_eq!(attachment.size_bytes, 128);
    }
}
