use base64::{engine::general_purpose::URL_SAFE, Engine};
use sodiumoxide::crypto::secretbox;

pub const KEY_SIZE: usize = 32;

/// Generate a cryptographically random 256-bit key.
pub fn generate_key() -> [u8; KEY_SIZE] {
    let key = secretbox::gen_key();
    let mut bytes = [0u8; KEY_SIZE];
    bytes.copy_from_slice(key.as_ref());
    bytes
}

/// Encode key as URL-safe base64 string (no padding stripped — standard URL_SAFE includes padding).
pub fn key_to_base64(key: &[u8; KEY_SIZE]) -> String {
    URL_SAFE.encode(key)
}

/// Decode key from URL-safe base64 string.
pub fn key_from_base64(encoded: &str) -> Result<[u8; KEY_SIZE], KeyError> {
    let bytes = URL_SAFE.decode(encoded).map_err(|_| KeyError::InvalidBase64)?;
    if bytes.len() != KEY_SIZE {
        return Err(KeyError::InvalidLength { got: bytes.len() });
    }
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("invalid base64 encoding")]
    InvalidBase64,
    #[error("invalid key size: expected {KEY_SIZE}, got {got}")]
    InvalidLength { got: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        sodiumoxide::init().expect("sodiumoxide::init() failed");
    }

    #[test]
    fn test_key_length() {
        init();
        let key = generate_key();
        assert_eq!(key.len(), KEY_SIZE, "key must be exactly {KEY_SIZE} bytes");
    }

    #[test]
    fn test_keys_are_unique() {
        init();
        let key1 = generate_key();
        let key2 = generate_key();
        assert_ne!(key1, key2, "two generated keys must not be identical");
    }

    #[test]
    fn test_key_base64_roundtrip() {
        init();
        let key = generate_key();
        let encoded = key_to_base64(&key);
        let decoded = key_from_base64(&encoded).unwrap();
        assert_eq!(
            key, decoded,
            "key must survive a base64 encode/decode roundtrip"
        );
    }

    #[test]
    fn test_invalid_base64_rejected() {
        let result = key_from_base64("not valid base64 !!!");
        assert!(
            matches!(result, Err(KeyError::InvalidBase64)),
            "expected InvalidBase64, got: {:?}",
            result
        );
    }

    #[test]
    fn test_invalid_key_length_rejected() {
        // Encode 16 bytes (too short) as URL-safe base64
        let short_key = URL_SAFE.encode(&[0u8; 16]);
        let result = key_from_base64(&short_key);
        assert!(
            matches!(result, Err(KeyError::InvalidLength { got: 16 })),
            "expected InvalidLength {{ got: 16 }}, got: {:?}",
            result
        );
    }
}
