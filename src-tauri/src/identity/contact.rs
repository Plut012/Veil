use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::crypto::ratchet::RatchetState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub contact_id: String,
    pub display_name: String,
    #[serde(with = "crate::crypto::keys::base64_serde")]
    pub key: [u8; 32],
    pub telegram_channel_id: i64,
    pub telegram_user_id: i64,
    pub created_at: DateTime<Utc>,
    pub envelope_template: String,
    #[serde(default)]
    pub is_initiator: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratchet: Option<RatchetState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_contact() -> Contact {
        Contact {
            contact_id: "test-id-1".to_string(),
            display_name: "Alice".to_string(),
            key: [42u8; 32],
            telegram_channel_id: 100,
            telegram_user_id: 200,
            created_at: Utc::now(),
            envelope_template: "v1".to_string(),
            is_initiator: false,
            ratchet: None,
        }
    }

    #[test]
    fn test_contact_serde_roundtrip() {
        let contact = make_contact();
        let json = serde_json::to_string(&contact).expect("serialize");
        let decoded: Contact = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.contact_id, contact.contact_id);
        assert_eq!(decoded.display_name, contact.display_name);
        assert_eq!(decoded.key, contact.key);
        assert_eq!(decoded.telegram_channel_id, contact.telegram_channel_id);
        assert_eq!(decoded.telegram_user_id, contact.telegram_user_id);
        assert_eq!(decoded.envelope_template, contact.envelope_template);
    }

    #[test]
    fn test_key_stored_as_base64_not_raw_bytes() {
        let contact = make_contact();
        let json = serde_json::to_string(&contact).expect("serialize");
        // The key should appear as a base64 string, not as an array of numbers
        assert!(json.contains("\"key\":\""), "key should be serialized as a base64 string");
        assert!(!json.contains("\"key\":["), "key should not be serialized as a byte array");
    }

    #[test]
    fn test_backward_compat_deserialize_without_ratchet_fields() {
        // Simulate a contact serialized by a pre-ratchet version (no is_initiator, no ratchet).
        let json = r#"{
            "contact_id": "old-id",
            "display_name": "Legacy",
            "key": "KioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKio=",
            "telegram_channel_id": 100,
            "telegram_user_id": 200,
            "created_at": "2025-01-01T00:00:00Z",
            "envelope_template": "v1"
        }"#;
        let contact: Contact = serde_json::from_str(json).expect("should deserialize without ratchet fields");
        assert!(!contact.is_initiator);
        assert!(contact.ratchet.is_none());
    }
}
