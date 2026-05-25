use sodiumoxide::crypto::secretbox;

/// Encrypt plaintext with XSalsa20-Poly1305.
/// Returns sealed bytes: nonce (24) + ciphertext + MAC (16).
pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let key = secretbox::Key::from_slice(key).expect("key slice is always 32 bytes");
    let nonce = secretbox::gen_nonce();
    let ciphertext = secretbox::seal(plaintext, &nonce, &key);

    let mut sealed = Vec::with_capacity(24 + ciphertext.len());
    sealed.extend_from_slice(nonce.as_ref());
    sealed.extend_from_slice(&ciphertext);
    sealed
}

/// Decrypt sealed bytes. Verifies MAC.
/// Returns plaintext or error on tamper/wrong key/invalid input.
pub fn decrypt(sealed: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    // Minimum: 24 (nonce) + 16 (MAC) = 40 bytes for an empty message
    if sealed.len() < 24 + 16 {
        return Err(CryptoError::InvalidLength);
    }

    let key = secretbox::Key::from_slice(key).expect("key slice is always 32 bytes");
    let nonce = secretbox::Nonce::from_slice(&sealed[..24]).ok_or(CryptoError::InvalidNonce)?;
    let ciphertext = &sealed[24..];

    secretbox::open(ciphertext, &nonce, &key).map_err(|_| CryptoError::DecryptionFailed)
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("sealed data too short")]
    InvalidLength,
    #[error("invalid nonce")]
    InvalidNonce,
    #[error("decryption failed: wrong key or tampered data")]
    DecryptionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_key;

    fn init() {
        sodiumoxide::init().expect("sodiumoxide::init() failed");
    }

    #[test]
    fn test_roundtrip() {
        init();
        let key = generate_key();
        let plaintext = b"hello, veil!";
        let sealed = encrypt(plaintext, &key);
        let decrypted = decrypt(&sealed, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_empty_message_roundtrip() {
        init();
        let key = generate_key();
        let plaintext = b"";
        let sealed = encrypt(plaintext, &key);
        let decrypted = decrypt(&sealed, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        init();
        let key1 = generate_key();
        let key2 = generate_key();
        let sealed = encrypt(b"secret message", &key1);
        let result = decrypt(&sealed, &key2);
        assert!(
            matches!(result, Err(CryptoError::DecryptionFailed)),
            "expected DecryptionFailed, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        init();
        let key = generate_key();
        let mut sealed = encrypt(b"important data", &key);
        // Flip a byte in the ciphertext region (after nonce)
        let tamper_idx = 30;
        sealed[tamper_idx] ^= 0xFF;
        let result = decrypt(&sealed, &key);
        assert!(
            matches!(result, Err(CryptoError::DecryptionFailed)),
            "expected DecryptionFailed after tampering, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_ciphertext_fails() {
        init();
        let key = generate_key();
        let sealed = encrypt(b"some data", &key);
        // Provide only the first 20 bytes — too short for nonce + MAC
        let truncated = &sealed[..20];
        let result = decrypt(truncated, &key);
        assert!(
            matches!(result, Err(CryptoError::InvalidLength)),
            "expected InvalidLength for truncated input, got: {:?}",
            result
        );
    }

    #[test]
    fn test_unique_nonces() {
        init();
        let key = generate_key();
        let plaintext = b"same message";
        let sealed1 = encrypt(plaintext, &key);
        let sealed2 = encrypt(plaintext, &key);
        // Nonces are the first 24 bytes; they must differ
        assert_ne!(
            &sealed1[..24],
            &sealed2[..24],
            "two encryptions of the same plaintext must produce different nonces"
        );
        // Ciphertexts must differ too
        assert_ne!(sealed1, sealed2);
    }

    #[test]
    fn test_sealed_format_size() {
        init();
        let key = generate_key();
        let plaintext = b"test";
        let sealed = encrypt(plaintext, &key);
        // Format: 24 (nonce) + plaintext_len + 16 (MAC)
        let expected_len = 24 + plaintext.len() + 16;
        assert_eq!(
            sealed.len(),
            expected_len,
            "sealed length should be 24 + plaintext_len + 16"
        );
    }
}
