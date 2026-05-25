use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde::{Deserialize, Serialize};

use crate::crypto::{decrypt, encrypt, generate_key, key_from_base64, key_to_base64};

const HANDSHAKE_TYPE: &str = "veil_handshake";

// ------------------------------------------------------------------
// Public types
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QrPayload {
    pub key: [u8; 32],
    pub telegram_user_id: i64,
    pub display_name: String,
    pub envelope_template: String,
}

/// Data returned from a successfully parsed handshake message.
#[derive(Debug, Clone)]
pub struct HandshakeData {
    pub name: String,
    pub env: String,
}

// ------------------------------------------------------------------
// Internal serde structs
// ------------------------------------------------------------------

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
    #[serde(rename = "type")]
    handshake_type: String,
    name: String,
    #[serde(default)]
    env: String,
}

// ------------------------------------------------------------------
// Public API
// ------------------------------------------------------------------

/// Generate a random pairing key, build the QR payload struct, and render a
/// QR code PNG.  Returns `(QrPayload, png_bytes)`.
pub fn create_qr_payload(
    telegram_user_id: i64,
    display_name: &str,
    envelope_template: &str,
) -> Result<(QrPayload, Vec<u8>), PairingError> {
    let key = generate_key();

    let json = serde_json::to_string(&QrPayloadJson {
        v: 1,
        key: key_to_base64(&key),
        tid: telegram_user_id,
        name: display_name.to_string(),
        env: envelope_template.to_string(),
    })
    .map_err(|e| PairingError::Internal(e.to_string()))?;

    let qr = qrcode::QrCode::new(json.as_bytes())
        .map_err(|e| PairingError::QrGeneration(e.to_string()))?;

    let img = qr.render::<image::Luma<u8>>().build();

    let mut png_bytes: Vec<u8> = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| PairingError::QrGeneration(e.to_string()))?;

    let payload = QrPayload {
        key,
        telegram_user_id,
        display_name: display_name.to_string(),
        envelope_template: envelope_template.to_string(),
    };

    Ok((payload, png_bytes))
}

/// Parse the JSON string obtained by scanning a QR code into a `QrPayload`.
/// Validates that the protocol version is 1.
pub fn parse_qr_payload(qr_data: &str) -> Result<QrPayload, PairingError> {
    let data: QrPayloadJson = serde_json::from_str(qr_data)
        .map_err(|e| PairingError::InvalidQr(e.to_string()))?;

    if data.v != 1 {
        return Err(PairingError::UnsupportedVersion(data.v));
    }

    let key = key_from_base64(&data.key).map_err(|e| PairingError::InvalidQr(e.to_string()))?;

    Ok(QrPayload {
        key,
        telegram_user_id: data.tid,
        display_name: data.name,
        envelope_template: data.env,
    })
}

/// Encrypt the joiner's handshake JSON with the shared key and return a
/// URL-safe base64 string ready to send over Telegram.
pub fn build_handshake_message(
    key: &[u8; 32],
    display_name: &str,
    envelope_template: &str,
) -> Result<String, PairingError> {
    let payload = serde_json::to_vec(&HandshakeJson {
        handshake_type: HANDSHAKE_TYPE.to_string(),
        name: display_name.to_string(),
        env: envelope_template.to_string(),
    })
    .map_err(|e| PairingError::Internal(e.to_string()))?;

    let sealed = encrypt(&payload, key);
    Ok(URL_SAFE.encode(&sealed))
}

/// Try to decode, decrypt, and validate a message as a handshake.
///
/// Returns `Some(HandshakeData)` on success, `None` on any failure (wrong key,
/// bad format, wrong type, etc.).  Never panics.
pub fn parse_handshake_message(message: &str, key: &[u8; 32]) -> Option<HandshakeData> {
    let sealed = URL_SAFE.decode(message).ok()?;
    let plaintext = decrypt(&sealed, key).ok()?;
    let data: HandshakeJson = serde_json::from_slice(&plaintext).ok()?;
    if data.handshake_type != HANDSHAKE_TYPE {
        return None;
    }
    Some(HandshakeData {
        name: data.name,
        env: data.env,
    })
}

// ------------------------------------------------------------------
// Error type
// ------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("invalid QR data: {0}")]
    InvalidQr(String),
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u32),
    #[error("QR generation failed: {0}")]
    QrGeneration(String),
    #[error("internal error: {0}")]
    Internal(String),
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn init_sodium() {
        sodiumoxide::init().expect("sodiumoxide::init() failed");
    }

    // --- QR payload roundtrip ---

    #[test]
    fn test_qr_payload_roundtrip() {
        init_sodium();
        let (payload, _png) =
            create_qr_payload(123456789, "Alice", "v1").expect("create_qr_payload failed");

        // Serialize back to the JSON format a scanner would see.
        let json = serde_json::to_string(&QrPayloadJson {
            v: 1,
            key: key_to_base64(&payload.key),
            tid: payload.telegram_user_id,
            name: payload.display_name.clone(),
            env: payload.envelope_template.clone(),
        })
        .unwrap();

        let parsed = parse_qr_payload(&json).expect("parse_qr_payload failed");

        assert_eq!(parsed.key, payload.key);
        assert_eq!(parsed.telegram_user_id, 123456789);
        assert_eq!(parsed.display_name, "Alice");
        assert_eq!(parsed.envelope_template, "v1");
    }

    // --- Invalid version rejected ---

    #[test]
    fn test_invalid_version_rejected() {
        init_sodium();
        let (payload, _png) =
            create_qr_payload(1, "Bob", "v1").expect("create_qr_payload failed");

        let json = serde_json::to_string(&QrPayloadJson {
            v: 99,
            key: key_to_base64(&payload.key),
            tid: payload.telegram_user_id,
            name: payload.display_name.clone(),
            env: payload.envelope_template.clone(),
        })
        .unwrap();

        let err = parse_qr_payload(&json).unwrap_err();
        assert!(
            matches!(err, PairingError::UnsupportedVersion(99)),
            "expected UnsupportedVersion(99), got: {:?}",
            err
        );
    }

    // --- Missing / invalid fields rejected ---

    #[test]
    fn test_missing_fields_rejected() {
        // Missing required fields → serde parse error.
        let err = parse_qr_payload(r#"{"v": 1}"#).unwrap_err();
        assert!(
            matches!(err, PairingError::InvalidQr(_)),
            "expected InvalidQr, got: {:?}",
            err
        );
    }

    #[test]
    fn test_invalid_json_rejected() {
        let err = parse_qr_payload("not json at all").unwrap_err();
        assert!(
            matches!(err, PairingError::InvalidQr(_)),
            "expected InvalidQr, got: {:?}",
            err
        );
    }

    // --- Handshake roundtrip ---

    #[test]
    fn test_handshake_roundtrip() {
        init_sodium();
        let key = generate_key();
        let msg = build_handshake_message(&key, "Charlie", "v2").expect("build failed");
        let result = parse_handshake_message(&msg, &key).expect("parse returned None");
        assert_eq!(result.name, "Charlie");
        assert_eq!(result.env, "v2");
    }

    // --- Wrong key returns None ---

    #[test]
    fn test_wrong_key_returns_none() {
        init_sodium();
        let key1 = generate_key();
        let key2 = generate_key();
        let msg = build_handshake_message(&key1, "Dana", "v1").expect("build failed");
        let result = parse_handshake_message(&msg, &key2);
        assert!(result.is_none(), "wrong key should return None, not panic");
    }

    // --- Non-handshake message returns None ---

    #[test]
    fn test_non_handshake_message_returns_none() {
        init_sodium();
        let key = generate_key();

        // Encrypt a valid JSON object whose `type` is not "veil_handshake".
        let payload = serde_json::to_vec(&serde_json::json!({
            "type": "something_else",
            "name": "Eve",
            "env": "v1"
        }))
        .unwrap();
        let sealed = encrypt(&payload, &key);
        let msg = URL_SAFE.encode(&sealed);

        let result = parse_handshake_message(&msg, &key);
        assert!(
            result.is_none(),
            "message with wrong type should return None"
        );
    }

    // --- Garbage base64 returns None ---

    #[test]
    fn test_garbage_input_returns_none() {
        init_sodium();
        let key = generate_key();
        assert!(parse_handshake_message("not-valid-base64!!!", &key).is_none());
        assert!(parse_handshake_message("", &key).is_none());
    }

    // --- QR PNG magic bytes ---

    #[test]
    fn test_qr_produces_valid_png() {
        init_sodium();
        let (_payload, png) =
            create_qr_payload(42, "Frank", "v1").expect("create_qr_payload failed");

        // PNG files always begin with the 8-byte magic signature.
        assert!(
            png.len() > 8,
            "PNG bytes should be more than 8 bytes, got {}",
            png.len()
        );
        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "first 8 bytes must be PNG magic signature"
        );
    }
}
