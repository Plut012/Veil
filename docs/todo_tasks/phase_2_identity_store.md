# Phase 2: Identity Store

**Status:** Not Started
**Dependencies:** Phase 1 (Crypto Engine)
**Output:** `src/veil/identity/contact.py`, `src/veil/identity/store.py`, `tests/test_identity.py`

---

## Purpose

The identity store manages contacts and their encryption keys. It handles persisting keys to disk encrypted under a user passphrase, loading them back, and providing a clean interface for the rest of the app to look up contacts and their keys.

This module owns the `~/.veil/` directory structure.

---

## Files

### `src/veil/identity/contact.py`

The contact model. A contact is someone you've paired with.

```python
from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class Contact:
    contact_id: str              # UUID — unique identifier
    display_name: str            # human-readable name (from pairing)
    key: bytes                   # 32-byte symmetric encryption key
    telegram_channel_id: int     # Telegram group/chat ID for this contact
    telegram_user_id: int        # contact's Telegram user ID
    created_at: datetime = field(default_factory=datetime.utcnow)
    envelope_template: str = ""  # contact's envelope template (for parsing their messages)
```

**Notes:**
- `contact_id` is a UUID4 string generated at pairing time.
- `key` is the shared symmetric key — never serialized to disk in plaintext.
- `envelope_template` stores the template the contact uses so we can parse their messages. Empty string means use the raw ciphertext format (no wrapping).
- `telegram_channel_id` is set during pairing when the Telegram group is created. It may be `0` temporarily during pairing before the channel exists.

### `src/veil/identity/store.py`

Encrypted local storage for contacts. Uses the crypto engine to encrypt the keyring at rest.

```python
import json
import os
import uuid
from pathlib import Path
from datetime import datetime
from hashlib import sha256

from veil.crypto import encrypt, decrypt, key_to_base64, key_from_base64
from veil.identity.contact import Contact

VEIL_DIR = Path.home() / ".veil"
KEYRING_DIR = VEIL_DIR / "keyring"
CONFIG_PATH = VEIL_DIR / "config.toml"


class IdentityStore:
    """Manages contacts and their encrypted keys on disk."""

    def __init__(self, passphrase: str, veil_dir: Path = VEIL_DIR):
        self.veil_dir = veil_dir
        self.keyring_dir = veil_dir / "keyring"
        self._derive_storage_key(passphrase)
        self._contacts: dict[str, Contact] = {}
        self._ensure_dirs()
        self._load_contacts()

    def _derive_storage_key(self, passphrase: str) -> None:
        """Derive a 32-byte key from the user's passphrase using Argon2id."""
        # Use PyNaCl's password hashing (Argon2id via libsodium)
        # The salt is fixed per installation (stored in veil_dir)
        ...

    def _ensure_dirs(self) -> None:
        """Create ~/.veil/ and ~/.veil/keyring/ if they don't exist."""
        self.veil_dir.mkdir(mode=0o700, exist_ok=True)
        self.keyring_dir.mkdir(mode=0o700, exist_ok=True)

    def _load_contacts(self) -> None:
        """Load all contact files from keyring directory, decrypt each."""
        ...

    def _save_contact(self, contact: Contact) -> None:
        """Encrypt and write a single contact to disk."""
        ...

    def _contact_to_json(self, contact: Contact) -> bytes:
        """Serialize contact to JSON bytes (including key as base64)."""
        ...

    def _contact_from_json(self, data: bytes) -> Contact:
        """Deserialize contact from JSON bytes."""
        ...

    def add_contact(self, contact: Contact) -> None:
        """Add a new contact and persist to disk."""
        self._contacts[contact.contact_id] = contact
        self._save_contact(contact)

    def remove_contact(self, contact_id: str) -> None:
        """Remove a contact and delete its keyring file."""
        ...

    def get_contact(self, contact_id: str) -> Contact | None:
        """Look up a contact by ID."""
        return self._contacts.get(contact_id)

    def get_contact_by_channel(self, channel_id: int) -> Contact | None:
        """Look up a contact by their Telegram channel ID."""
        for contact in self._contacts.values():
            if contact.telegram_channel_id == channel_id:
                return contact
        return None

    def list_contacts(self) -> list[Contact]:
        """Return all contacts."""
        return list(self._contacts.values())

    def update_contact(self, contact: Contact) -> None:
        """Update an existing contact and re-persist."""
        self._contacts[contact.contact_id] = contact
        self._save_contact(contact)
```

**Implementation details:**

**Passphrase → storage key derivation:**
- Use `nacl.pwhash` (Argon2id via libsodium) to derive a 32-byte key from the passphrase.
- A random salt is generated on first run and stored at `~/.veil/salt` (16 bytes, not secret — standard practice).
- Argon2id parameters: use `nacl.pwhash.argon2id.OPSLIMIT_MODERATE` and `MEMLIMIT_MODERATE` for a reasonable security/speed tradeoff.

**Contact file format:**
- Each contact is stored as a separate file: `~/.veil/keyring/{contact_id}.enc`
- File contents: `encrypt(contact_json_bytes, storage_key)`
- One file per contact means adding/removing contacts doesn't require rewriting the entire keyring.

**Directory permissions:**
- `~/.veil/` and `~/.veil/keyring/` are created with `0o700` (owner-only).

**Salt file:**
- `~/.veil/salt` — 16 random bytes, created on first `IdentityStore` initialization.
- If the salt file doesn't exist, generate it. If it does, read it.

### `src/veil/identity/__init__.py`

```python
from veil.identity.contact import Contact
from veil.identity.store import IdentityStore
```

### `tests/test_identity.py`

```python
"""Tests for identity store. Uses a temporary directory instead of ~/.veil/."""
import pytest
from pathlib import Path
from veil.identity import Contact, IdentityStore
from veil.crypto import generate_key


@pytest.fixture
def store(tmp_path):
    """Create an IdentityStore with a temp directory."""
    return IdentityStore("test-passphrase", veil_dir=tmp_path)


@pytest.fixture
def sample_contact():
    return Contact(
        contact_id="test-uuid-1234",
        display_name="Alice",
        key=generate_key(),
        telegram_channel_id=123456,
        telegram_user_id=789,
    )


class TestIdentityStore:
    def test_add_and_retrieve(self, store, sample_contact):
        """Added contact is retrievable by ID."""
        store.add_contact(sample_contact)
        retrieved = store.get_contact(sample_contact.contact_id)
        assert retrieved is not None
        assert retrieved.display_name == "Alice"
        assert retrieved.key == sample_contact.key

    def test_persistence_across_instances(self, tmp_path, sample_contact):
        """Contact survives store close and reopen with same passphrase."""
        store1 = IdentityStore("my-passphrase", veil_dir=tmp_path)
        store1.add_contact(sample_contact)

        store2 = IdentityStore("my-passphrase", veil_dir=tmp_path)
        retrieved = store2.get_contact(sample_contact.contact_id)
        assert retrieved is not None
        assert retrieved.key == sample_contact.key

    def test_wrong_passphrase_fails(self, tmp_path, sample_contact):
        """Wrong passphrase cannot decrypt stored contacts."""
        store1 = IdentityStore("correct-passphrase", veil_dir=tmp_path)
        store1.add_contact(sample_contact)

        # Opening with wrong passphrase should fail to load contacts
        # (CryptoError during decryption — store should handle gracefully)
        store2 = IdentityStore("wrong-passphrase", veil_dir=tmp_path)
        assert store2.get_contact(sample_contact.contact_id) is None

    def test_lookup_by_channel(self, store, sample_contact):
        """Contact is findable by Telegram channel ID."""
        store.add_contact(sample_contact)
        retrieved = store.get_contact_by_channel(123456)
        assert retrieved is not None
        assert retrieved.contact_id == sample_contact.contact_id

    def test_remove_contact(self, store, sample_contact):
        """Removed contact is gone from memory and disk."""
        store.add_contact(sample_contact)
        store.remove_contact(sample_contact.contact_id)
        assert store.get_contact(sample_contact.contact_id) is None

    def test_list_contacts(self, store):
        """list_contacts returns all added contacts."""
        key = generate_key()
        for i in range(3):
            store.add_contact(Contact(
                contact_id=f"id-{i}",
                display_name=f"Contact {i}",
                key=key,
                telegram_channel_id=i,
                telegram_user_id=i + 100,
            ))
        assert len(store.list_contacts()) == 3

    def test_update_contact(self, store, sample_contact):
        """Updated contact reflects changes."""
        store.add_contact(sample_contact)
        sample_contact.display_name = "Alice Updated"
        store.update_contact(sample_contact)
        retrieved = store.get_contact(sample_contact.contact_id)
        assert retrieved.display_name == "Alice Updated"

    def test_directory_permissions(self, tmp_path):
        """Veil directory is created with owner-only permissions."""
        veil_dir = tmp_path / "veil_test"
        IdentityStore("passphrase", veil_dir=veil_dir)
        assert oct(veil_dir.stat().st_mode)[-3:] == "700"

    def test_encrypted_on_disk(self, tmp_path, sample_contact):
        """Contact files on disk are encrypted (not readable as JSON)."""
        store = IdentityStore("passphrase", veil_dir=tmp_path)
        store.add_contact(sample_contact)

        keyring = tmp_path / "keyring"
        files = list(keyring.glob("*.enc"))
        assert len(files) == 1

        raw = files[0].read_bytes()
        # Should not be valid JSON (it's encrypted)
        with pytest.raises(Exception):
            import json
            json.loads(raw)
```

---

## Acceptance Criteria

- [ ] `IdentityStore` creates `~/.veil/` and `~/.veil/keyring/` with `0o700` permissions
- [ ] Passphrase is derived to a storage key using Argon2id (via `nacl.pwhash`)
- [ ] Salt is generated on first run and persisted at `~/.veil/salt`
- [ ] Contacts are serialized to JSON, encrypted with the storage key, and saved as `.enc` files
- [ ] Contacts survive close/reopen with the same passphrase
- [ ] Wrong passphrase cannot read contacts
- [ ] Contacts are retrievable by ID and by Telegram channel ID
- [ ] Contacts can be added, removed, updated, and listed
- [ ] Contact files on disk are not readable as plaintext
- [ ] All tests pass: `uv run pytest tests/test_identity.py -v`
