use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub contact_id: String,
    pub display_name: String,
    #[serde(with = "base64_key")]
    pub key: [u8; 32],
    pub telegram_channel_id: i64,
    pub telegram_user_id: i64,
    pub created_at: DateTime<Utc>,
    pub envelope_template: String,
}

/// Custom serde module for serializing [u8; 32] as base64.
mod base64_key {
    use base64::{engine::general_purpose::URL_SAFE, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(key: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&URL_SAFE.encode(key))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = URL_SAFE.decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "invalid key length: expected 32, got {}",
                bytes.len()
            )));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    }
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
}
