# Tauri Phase 4: Identity Store (Rust)

**Status:** Not Started
**Dependencies:** Phase 2 (Crypto Engine)
**Output:** `src-tauri/src/identity/contact.rs`, `src-tauri/src/identity/store.rs`, unit tests

---

## Purpose

Port the identity store to Rust. Manages contacts and their encryption keys, persists them encrypted on disk using Argon2id-derived storage key from a user passphrase.

---

## Dependencies (Cargo.toml)

```toml
argon2 = "0.5"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
```

---

## Files

### `src-tauri/src/identity/contact.rs`

```rust
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

/// Custom serde module for serializing [u8; 32] as base64
mod base64_key {
    use base64::{engine::general_purpose::URL_SAFE, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(key: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&URL_SAFE.encode(key))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let bytes = URL_SAFE.decode(&s).map_err(serde::de::Error::custom)?;
        let mut key = [0u8; 32];
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("invalid key length"));
        }
        key.copy_from_slice(&bytes);
        Ok(key)
    }
}
```

### `src-tauri/src/identity/store.rs`

```rust
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;

use crate::crypto::{encrypt, decrypt, CryptoError};
use crate::identity::contact::Contact;

const SALT_SIZE: usize = 16;
const KEY_SIZE: usize = 32;

pub struct IdentityStore {
    veil_dir: PathBuf,
    keyring_dir: PathBuf,
    storage_key: [u8; KEY_SIZE],
    contacts: HashMap<String, Contact>,
}

impl IdentityStore {
    pub fn new(passphrase: &str, veil_dir: Option<PathBuf>) -> Result<Self, StoreError> {
        let veil_dir = veil_dir.unwrap_or_else(|| {
            dirs::home_dir().unwrap().join(".veil")
        });
        let keyring_dir = veil_dir.join("keyring");

        // Create dirs with restrictive permissions
        ensure_dir(&veil_dir)?;
        ensure_dir(&keyring_dir)?;

        // Derive storage key
        let storage_key = derive_key(passphrase, &veil_dir)?;

        let mut store = Self {
            veil_dir,
            keyring_dir,
            storage_key,
            contacts: HashMap::new(),
        };
        store.load_contacts();
        Ok(store)
    }

    pub fn add_contact(&mut self, contact: Contact) { ... }
    pub fn remove_contact(&mut self, contact_id: &str) { ... }
    pub fn get_contact(&self, contact_id: &str) -> Option<&Contact> { ... }
    pub fn get_contact_by_channel(&self, channel_id: i64) -> Option<&Contact> { ... }
    pub fn list_contacts(&self) -> Vec<&Contact> { ... }
    pub fn update_contact(&mut self, contact: Contact) { ... }

    fn load_contacts(&mut self) { ... }
    fn save_contact(&self, contact: &Contact) -> Result<(), StoreError> { ... }
}

fn derive_key(passphrase: &str, veil_dir: &Path) -> Result<[u8; KEY_SIZE], StoreError> {
    let salt_path = veil_dir.join("salt");
    let salt = if salt_path.exists() {
        fs::read(&salt_path)?
    } else {
        let mut salt = vec![0u8; SALT_SIZE];
        // Use OsRng for salt generation
        use argon2::password_hash::rand_core::RngCore;
        OsRng.fill_bytes(&mut salt);
        fs::write(&salt_path, &salt)?;
        salt
    };

    let mut key = [0u8; KEY_SIZE];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|e| StoreError::KeyDerivation(e.to_string()))?;
    Ok(key)
}

fn ensure_dir(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path)?;
    // Set permissions to 0o700 on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

**Implementation notes:**
- Each contact saved as `keyring/{contact_id}.enc` — `encrypt(serde_json::to_vec(&contact), storage_key)`
- `load_contacts` silently skips files that fail to decrypt (wrong passphrase)
- `derive_key` uses `Argon2::default()` which is Argon2id with reasonable defaults
- Directory permissions set to `0o700` on Unix via `PermissionsExt`

### `src-tauri/src/identity/mod.rs`

```rust
pub mod contact;
pub mod store;
pub mod pairing;  // placeholder for Phase 6

pub use contact::Contact;
pub use store::IdentityStore;
```

---

## Acceptance Criteria

- [ ] `IdentityStore::new()` creates dirs with 0o700 permissions
- [ ] Passphrase derived to storage key via Argon2id
- [ ] Salt generated on first run, persisted at `{veil_dir}/salt`
- [ ] Contacts encrypted as individual `.enc` files
- [ ] Contacts survive close/reopen with same passphrase
- [ ] Wrong passphrase results in empty contact list (no crash)
- [ ] CRUD operations: add, remove, get by ID, get by channel, list, update
- [ ] Contact files on disk are not readable as plaintext
- [ ] `cargo test` passes all identity tests
