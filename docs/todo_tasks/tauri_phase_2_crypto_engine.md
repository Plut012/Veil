# Tauri Phase 2: Crypto Engine (Rust)

**Status:** Not Started
**Dependencies:** Phase 1 (project compiles)
**Output:** `src-tauri/src/crypto/engine.rs`, `src-tauri/src/crypto/keys.rs`, unit tests

---

## Purpose

Port the crypto engine to Rust. Same interface as Python: `encrypt(plaintext, key) -> sealed`, `decrypt(sealed, key) -> plaintext`, `generate_key() -> key`. Uses `sodiumoxide` (libsodium bindings) for XSalsa20-Poly1305 — same algorithm as the Python version, ciphertext-compatible.

---

## Dependencies (Cargo.toml)

```toml
sodiumoxide = "0.2"
base64 = "0.22"
```

---

## Files

### `src-tauri/src/crypto/engine.rs`

```rust
use sodiumoxide::crypto::secretbox;

/// Encrypt plaintext with XSalsa20-Poly1305.
/// Returns sealed bytes: nonce (24) + ciphertext + MAC (16).
pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let key = secretbox::Key::from_slice(key).expect("invalid key");
    let nonce = secretbox::gen_nonce();
    let ciphertext = secretbox::seal(plaintext, &nonce, &key);
    
    let mut sealed = Vec::with_capacity(24 + ciphertext.len());
    sealed.extend_from_slice(nonce.as_ref());
    sealed.extend_from_slice(&ciphertext);
    sealed
}

/// Decrypt sealed bytes. Verifies MAC.
/// Returns plaintext or error on tamper/wrong key.
pub fn decrypt(sealed: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    if sealed.len() < 24 + 16 {
        return Err(CryptoError::InvalidLength);
    }
    
    let key = secretbox::Key::from_slice(key).expect("invalid key");
    let nonce = secretbox::Nonce::from_slice(&sealed[..24])
        .ok_or(CryptoError::InvalidNonce)?;
    let ciphertext = &sealed[24..];
    
    secretbox::open(ciphertext, &nonce, &key)
        .map_err(|_| CryptoError::DecryptionFailed)
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
```

**Note:** `sodiumoxide::crypto::secretbox::seal` appends the MAC to the ciphertext. The sealed format must match PyNaCl exactly: `nonce (24) || ciphertext || MAC (16)`. Verify this in tests by cross-checking with known PyNaCl output.

### `src-tauri/src/crypto/keys.rs`

```rust
use sodiumoxide::crypto::secretbox;
use base64::{engine::general_purpose::URL_SAFE, Engine};

pub const KEY_SIZE: usize = 32;

/// Generate a cryptographically random 256-bit key.
pub fn generate_key() -> [u8; KEY_SIZE] {
    let key = secretbox::gen_key();
    let mut bytes = [0u8; KEY_SIZE];
    bytes.copy_from_slice(key.as_ref());
    bytes
}

/// Encode key as URL-safe base64 string.
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
```

### `src-tauri/src/crypto/mod.rs`

```rust
pub mod engine;
pub mod keys;

pub use engine::{encrypt, decrypt, CryptoError};
pub use keys::{generate_key, key_to_base64, key_from_base64, KeyError, KEY_SIZE};
```

### Tests

Tests live in each module file using `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() { ... }
    #[test]
    fn test_wrong_key_fails() { ... }
    #[test]
    fn test_tampered_ciphertext_fails() { ... }
    #[test]
    fn test_unique_nonces() { ... }
    #[test]
    fn test_sealed_format_size() { ... }
    #[test]
    fn test_key_length() { ... }
    #[test]
    fn test_key_base64_roundtrip() { ... }
    #[test]
    fn test_invalid_base64_rejected() { ... }
}
```

---

## Critical: Ciphertext Compatibility

The sealed format MUST match PyNaCl's `SecretBox.encrypt()` output:
- `nonce (24 bytes) || ciphertext+MAC`

`sodiumoxide::secretbox::seal()` returns `ciphertext+MAC` (no nonce). We prepend the nonce manually. PyNaCl prepends the nonce automatically. The result should be identical.

Write a test that encrypts with known key+nonce and verifies the output matches what PyNaCl would produce. Or: generate a sealed message with the Python version, hardcode it as a test vector, and verify Rust can decrypt it.

---

## Acceptance Criteria

- [ ] `encrypt(plaintext, key)` returns sealed bytes (nonce + ciphertext + MAC)
- [ ] `decrypt(sealed, key)` returns original plaintext
- [ ] Wrong key returns `Err(CryptoError::DecryptionFailed)`
- [ ] Tampered ciphertext returns error
- [ ] Each encryption produces different output (random nonce)
- [ ] `generate_key()` returns 32 random bytes
- [ ] Keys roundtrip through base64 encoding
- [ ] `cargo test` passes all crypto tests
- [ ] Ciphertext format is compatible with PyNaCl output
