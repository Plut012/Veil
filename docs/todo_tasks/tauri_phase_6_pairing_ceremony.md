# Tauri Phase 6: Pairing Ceremony (Rust)

**Status:** Not Started
**Dependencies:** Phase 2 (Crypto), Phase 4 (Identity), Phase 5 (Bridge)
**Output:** `src-tauri/src/identity/pairing.rs`, unit tests

---

## Purpose

Port the pairing ceremony to Rust. Generates QR codes for initiator, parses them for joiner, handles the encrypted handshake, creates Telegram channels, and saves contacts.

---

## Dependencies (Cargo.toml)

```toml
qrcode = "0.14"
image = "0.25"
```

---

## Files

### `src-tauri/src/identity/pairing.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::{encrypt, decrypt, generate_key, key_to_base64, key_from_base64};
use crate::identity::contact::Contact;
use crate::identity::store::IdentityStore;
use crate::bridges::TelegramBridge;

const HANDSHAKE_TYPE: &str = "veil_handshake";

#[derive(Debug, Clone)]
pub struct QrPayload {
    pub key: [u8; 32],
    pub telegram_user_id: i64,
    pub display_name: String,
    pub envelope_template: String,
}

#[derive(Serialize, Deserialize)]
struct QrPayloadJson {
    v: u32,
    key: String,
    tid: i64,
    name: String,
    #[serde(default)]
    env: String,
}

#[derive(Serialize, Deserialize)]
struct HandshakeJson {
    r#type: String,
    name: String,
    #[serde(default)]
    env: String,
}

/// Generate QR payload and PNG bytes for initiator.
pub fn create_qr_payload(
    telegram_user_id: i64,
    display_name: &str,
    envelope_template: &str,
) -> (QrPayload, Vec<u8>) {
    let key = generate_key();
    let payload = QrPayload {
        key,
        telegram_user_id,
        display_name: display_name.to_string(),
        envelope_template: envelope_template.to_string(),
    };

    let json = serde_json::to_string(&QrPayloadJson {
        v: 1,
        key: key_to_base64(&key),
        tid: telegram_user_id,
        name: display_name.to_string(),
        env: envelope_template.to_string(),
    }).unwrap();

    // Generate QR code as PNG bytes
    let qr = qrcode::QrCode::new(json.as_bytes()).unwrap();
    let image = qr.render::<image::Luma<u8>>().build();
    let mut png_bytes = Vec::new();
    image::DynamicImage::ImageLuma8(image)
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .unwrap();

    (payload, png_bytes)
}

/// Parse scanned QR code JSON string.
pub fn parse_qr_payload(qr_data: &str) -> Result<QrPayload, PairingError> {
    let data: QrPayloadJson = serde_json::from_str(qr_data)
        .map_err(|e| PairingError::InvalidQr(e.to_string()))?;

    if data.v != 1 {
        return Err(PairingError::UnsupportedVersion(data.v));
    }

    let key = key_from_base64(&data.key)
        .map_err(|e| PairingError::InvalidQr(e.to_string()))?;

    Ok(QrPayload {
        key,
        telegram_user_id: data.tid,
        display_name: data.name,
        envelope_template: data.env,
    })
}

/// Build encrypted handshake message (joiner sends this).
pub fn build_handshake_message(
    key: &[u8; 32],
    display_name: &str,
    envelope_template: &str,
) -> String {
    let payload = serde_json::to_vec(&HandshakeJson {
        r#type: HANDSHAKE_TYPE.to_string(),
        name: display_name.to_string(),
        env: envelope_template.to_string(),
    }).unwrap();

    let sealed = encrypt(&payload, key);
    base64::engine::general_purpose::URL_SAFE.encode(&sealed)
}

/// Try to parse incoming message as handshake. Returns None if not valid.
pub fn parse_handshake_message(message: &str, key: &[u8; 32]) -> Option<HandshakeJson> {
    let sealed = base64::engine::general_purpose::URL_SAFE.decode(message).ok()?;
    let plaintext = decrypt(&sealed, key).ok()?;
    let data: HandshakeJson = serde_json::from_slice(&plaintext).ok()?;
    if data.r#type == HANDSHAKE_TYPE {
        Some(data)
    } else {
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("invalid QR data: {0}")]
    InvalidQr(String),
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u32),
    #[error("bridge error: {0}")]
    Bridge(String),
}
```

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_payload_roundtrip() { ... }
    #[test]
    fn test_invalid_version_rejected() { ... }
    #[test]
    fn test_handshake_roundtrip() { ... }
    #[test]
    fn test_wrong_key_returns_none() { ... }
    #[test]
    fn test_non_handshake_returns_none() { ... }
    #[test]
    fn test_qr_produces_valid_png() { ... }
}
```

---

## Acceptance Criteria

- [ ] `create_qr_payload` generates key and returns valid PNG bytes
- [ ] `parse_qr_payload` extracts key, user ID, display name, envelope template
- [ ] QR payload roundtrips (create → serialize → parse)
- [ ] Invalid protocol version rejected
- [ ] `build_handshake_message` produces encrypted base64 message
- [ ] `parse_handshake_message` decrypts and validates
- [ ] Wrong key returns `None` (no panic)
- [ ] Non-handshake messages return `None`
- [ ] QR JSON format compatible with Python version (`v`, `key`, `tid`, `name`, `env`)
- [ ] `cargo test` passes all pairing tests
