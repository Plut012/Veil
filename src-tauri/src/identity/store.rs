use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use argon2::Argon2;

use crate::crypto::{decrypt, encrypt, CryptoError};
use crate::identity::contact::Contact;

const SALT_SIZE: usize = 16;
const KEY_SIZE: usize = 32;

pub struct IdentityStore {
    keyring_dir: PathBuf,
    storage_key: [u8; KEY_SIZE],
    contacts: HashMap<String, Contact>,
}

impl IdentityStore {
    /// Create or open an identity store.
    ///
    /// `passphrase` is used to derive the storage key via Argon2id.
    /// `veil_dir` defaults to `~/.veil` if `None`.
    pub fn new(passphrase: &str, veil_dir: Option<PathBuf>) -> Result<Self, StoreError> {
        let veil_dir = match veil_dir {
            Some(d) => d,
            None => dirs::home_dir()
                .ok_or_else(|| StoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine home directory",
                )))?
                .join(".veil"),
        };
        let keyring_dir = veil_dir.join("keyring");

        ensure_dir(&veil_dir)?;
        ensure_dir(&keyring_dir)?;

        let storage_key = derive_key(passphrase, &veil_dir)?;

        let mut store = Self {
            keyring_dir,
            storage_key,
            contacts: HashMap::new(),
        };
        store.load_contacts();
        Ok(store)
    }

    /// Add a contact and persist it to disk.
    pub fn add_contact(&mut self, contact: Contact) -> Result<(), StoreError> {
        self.save_contact(&contact)?;
        self.contacts.insert(contact.contact_id.clone(), contact);
        Ok(())
    }

    /// Remove a contact from memory and disk.
    pub fn remove_contact(&mut self, contact_id: &str) -> Result<(), StoreError> {
        self.contacts.remove(contact_id);
        let path = self.contact_path(contact_id);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Look up a contact by its ID.
    pub fn get_contact(&self, contact_id: &str) -> Option<&Contact> {
        self.contacts.get(contact_id)
    }

    /// Look up a contact by Telegram channel ID.
    pub fn get_contact_by_channel(&self, channel_id: i64) -> Option<&Contact> {
        self.contacts
            .values()
            .find(|c| c.telegram_channel_id == channel_id)
    }

    /// List all contacts.
    pub fn list_contacts(&self) -> Vec<&Contact> {
        self.contacts.values().collect()
    }

    /// Update an existing contact (replaces by contact_id).
    pub fn update_contact(&mut self, contact: Contact) -> Result<(), StoreError> {
        self.save_contact(&contact)?;
        self.contacts.insert(contact.contact_id.clone(), contact);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn contact_path(&self, contact_id: &str) -> PathBuf {
        self.keyring_dir.join(format!("{contact_id}.enc"))
    }

    fn save_contact(&self, contact: &Contact) -> Result<(), StoreError> {
        let json = serde_json::to_vec(contact)?;
        let sealed = encrypt(&json, &self.storage_key);
        let path = self.contact_path(&contact.contact_id);
        fs::write(&path, &sealed)?;
        Ok(())
    }

    /// Load all `.enc` files from the keyring directory.
    /// Files that fail to decrypt (e.g., wrong passphrase) are silently skipped.
    fn load_contacts(&mut self) {
        let entries = match fs::read_dir(&self.keyring_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("enc") {
                continue;
            }

            let sealed = match fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let plaintext = match decrypt(&sealed, &self.storage_key) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let contact: Contact = match serde_json::from_slice(&plaintext) {
                Ok(c) => c,
                Err(_) => continue,
            };

            self.contacts.insert(contact.contact_id.clone(), contact);
        }
    }
}

// ------------------------------------------------------------------
// Key derivation
// ------------------------------------------------------------------

fn derive_key(passphrase: &str, veil_dir: &Path) -> Result<[u8; KEY_SIZE], StoreError> {
    let salt_path = veil_dir.join("salt");
    let salt = if salt_path.exists() {
        let bytes = fs::read(&salt_path)?;
        if bytes.len() < 8 {
            return Err(StoreError::KeyDerivation(
                "persisted salt is too short (< 8 bytes)".to_string(),
            ));
        }
        bytes
    } else {
        // Generate a fresh random salt using sodiumoxide (already a project dependency).
        let salt = sodiumoxide::randombytes::randombytes(SALT_SIZE);
        fs::write(&salt_path, &salt)?;
        salt
    };

    let mut key = [0u8; KEY_SIZE];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|e| StoreError::KeyDerivation(e.to_string()))?;
    Ok(key)
}

// ------------------------------------------------------------------
// Directory helpers
// ------------------------------------------------------------------

fn ensure_dir(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

// ------------------------------------------------------------------
// Error type
// ------------------------------------------------------------------

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

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn init_sodium() {
        sodiumoxide::init().expect("sodiumoxide::init() failed");
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("veil_test_{}", Uuid::new_v4()));
        dir
    }

    fn make_contact(channel_id: i64) -> Contact {
        Contact {
            contact_id: Uuid::new_v4().to_string(),
            display_name: "Test User".to_string(),
            key: [7u8; 32],
            telegram_channel_id: channel_id,
            telegram_user_id: 999,
            created_at: Utc::now(),
            envelope_template: "v1".to_string(),
        }
    }

    // ------------------------------------------------------------------

    #[test]
    fn test_add_and_get_contact() {
        init_sodium();
        let dir = temp_dir();
        let mut store = IdentityStore::new("passphrase", Some(dir)).unwrap();
        let contact = make_contact(1001);
        let id = contact.contact_id.clone();
        store.add_contact(contact).unwrap();
        let found = store.get_contact(&id);
        assert!(found.is_some(), "contact should be retrievable by ID");
        assert_eq!(found.unwrap().contact_id, id);
    }

    #[test]
    fn test_persistence_across_instances() {
        init_sodium();
        let dir = temp_dir();
        let contact = make_contact(1002);
        let id = contact.contact_id.clone();

        {
            let mut store = IdentityStore::new("correct-pass", Some(dir.clone())).unwrap();
            store.add_contact(contact).unwrap();
        }

        let store2 = IdentityStore::new("correct-pass", Some(dir)).unwrap();
        let found = store2.get_contact(&id);
        assert!(found.is_some(), "contact should persist across store instances");
        assert_eq!(found.unwrap().contact_id, id);
    }

    #[test]
    fn test_wrong_passphrase_yields_empty_list() {
        init_sodium();
        let dir = temp_dir();
        let contact = make_contact(1003);

        {
            let mut store = IdentityStore::new("correct-pass", Some(dir.clone())).unwrap();
            store.add_contact(contact).unwrap();
        }

        let store2 = IdentityStore::new("wrong-pass", Some(dir)).unwrap();
        assert!(
            store2.list_contacts().is_empty(),
            "wrong passphrase should yield empty contact list, not a crash"
        );
    }

    #[test]
    fn test_get_contact_by_channel() {
        init_sodium();
        let dir = temp_dir();
        let mut store = IdentityStore::new("passphrase", Some(dir)).unwrap();
        let contact = make_contact(5555);
        let id = contact.contact_id.clone();
        store.add_contact(contact).unwrap();

        let found = store.get_contact_by_channel(5555);
        assert!(found.is_some(), "should find contact by channel ID");
        assert_eq!(found.unwrap().contact_id, id);

        assert!(
            store.get_contact_by_channel(9999).is_none(),
            "unknown channel should return None"
        );
    }

    #[test]
    fn test_remove_contact() {
        init_sodium();
        let dir = temp_dir();
        let mut store = IdentityStore::new("passphrase", Some(dir.clone())).unwrap();
        let contact = make_contact(2001);
        let id = contact.contact_id.clone();
        store.add_contact(contact).unwrap();

        // Verify it exists on disk before removal
        let enc_path = dir.join("keyring").join(format!("{id}.enc"));
        assert!(enc_path.exists(), "contact file should exist before removal");

        store.remove_contact(&id).unwrap();
        assert!(store.get_contact(&id).is_none(), "contact should be gone from memory");
        assert!(!enc_path.exists(), "contact file should be removed from disk");
    }

    #[test]
    fn test_list_contacts() {
        init_sodium();
        let dir = temp_dir();
        let mut store = IdentityStore::new("passphrase", Some(dir)).unwrap();

        assert!(store.list_contacts().is_empty(), "store starts empty");

        let c1 = make_contact(3001);
        let c2 = make_contact(3002);
        let c3 = make_contact(3003);
        store.add_contact(c1).unwrap();
        store.add_contact(c2).unwrap();
        store.add_contact(c3).unwrap();

        assert_eq!(store.list_contacts().len(), 3, "should list all 3 contacts");
    }

    #[test]
    fn test_update_contact() {
        init_sodium();
        let dir = temp_dir();
        let mut store = IdentityStore::new("passphrase", Some(dir.clone())).unwrap();
        let contact = make_contact(4001);
        let id = contact.contact_id.clone();
        store.add_contact(contact).unwrap();

        let mut updated = store.get_contact(&id).unwrap().clone();
        updated.display_name = "Updated Name".to_string();
        store.update_contact(updated).unwrap();

        assert_eq!(
            store.get_contact(&id).unwrap().display_name,
            "Updated Name",
            "contact should reflect the update in memory"
        );

        // Verify the update persisted to disk
        let store2 = IdentityStore::new("passphrase", Some(dir)).unwrap();
        assert_eq!(
            store2.get_contact(&id).unwrap().display_name,
            "Updated Name",
            "contact update should persist across store instances"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_directory_permissions_are_0700() {
        use std::os::unix::fs::PermissionsExt;
        init_sodium();
        let dir = temp_dir();
        IdentityStore::new("passphrase", Some(dir.clone())).unwrap();

        let veil_meta = fs::metadata(&dir).unwrap();
        let veil_mode = veil_meta.permissions().mode() & 0o777;
        assert_eq!(veil_mode, 0o700, "veil_dir should have 0700 permissions");

        let keyring_dir = dir.join("keyring");
        let keyring_meta = fs::metadata(&keyring_dir).unwrap();
        let keyring_mode = keyring_meta.permissions().mode() & 0o777;
        assert_eq!(keyring_mode, 0o700, "keyring_dir should have 0700 permissions");
    }

    #[test]
    fn test_contact_files_are_encrypted() {
        init_sodium();
        let dir = temp_dir();
        let mut store = IdentityStore::new("passphrase", Some(dir.clone())).unwrap();
        let contact = make_contact(6001);
        let id = contact.contact_id.clone();
        store.add_contact(contact).unwrap();

        let enc_path = dir.join("keyring").join(format!("{id}.enc"));
        let raw = fs::read(&enc_path).unwrap();

        // The file must not be valid JSON (it's encrypted binary data)
        assert!(
            serde_json::from_slice::<serde_json::Value>(&raw).is_err(),
            "contact file on disk should not be readable as plaintext JSON"
        );
    }
}
