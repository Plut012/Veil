import json
import os
from pathlib import Path
from datetime import datetime

import nacl.pwhash.argon2id
import nacl.utils
from nacl.exceptions import CryptoError

from veil.crypto import encrypt, decrypt, key_to_base64, key_from_base64
from veil.identity.contact import Contact

VEIL_DIR = Path.home() / ".veil"
KEYRING_DIR = VEIL_DIR / "keyring"
CONFIG_PATH = VEIL_DIR / "config.toml"

_SALT_SIZE = nacl.pwhash.argon2id.SALTBYTES
_KEY_SIZE = 32


class IdentityStore:
    """Manages contacts and their encrypted keys on disk."""

    def __init__(self, passphrase: str, veil_dir: Path = VEIL_DIR):
        self.veil_dir = veil_dir
        self.keyring_dir = veil_dir / "keyring"
        self._ensure_dirs()
        self._derive_storage_key(passphrase)
        self._contacts: dict[str, Contact] = {}
        self._load_contacts()

    def _derive_storage_key(self, passphrase: str) -> None:
        """Derive a 32-byte key from the user's passphrase using Argon2id."""
        salt_path = self.veil_dir / "salt"
        if salt_path.exists():
            salt = salt_path.read_bytes()
        else:
            salt = nacl.utils.random(_SALT_SIZE)
            salt_path.write_bytes(salt)

        self._storage_key = nacl.pwhash.argon2id.kdf(
            size=_KEY_SIZE,
            password=passphrase.encode("utf-8"),
            salt=salt,
            opslimit=nacl.pwhash.argon2id.OPSLIMIT_MODERATE,
            memlimit=nacl.pwhash.argon2id.MEMLIMIT_MODERATE,
        )

    def _ensure_dirs(self) -> None:
        """Create veil_dir and keyring subdirectory if they don't exist."""
        self.veil_dir.mkdir(mode=0o700, exist_ok=True)
        self.keyring_dir.mkdir(mode=0o700, exist_ok=True)

    def _load_contacts(self) -> None:
        """Load all contact files from keyring directory, decrypt each."""
        for enc_file in self.keyring_dir.glob("*.enc"):
            try:
                sealed = enc_file.read_bytes()
                plaintext = decrypt(sealed, self._storage_key)
                contact = self._contact_from_json(plaintext)
                self._contacts[contact.contact_id] = contact
            except (CryptoError, Exception):
                # Wrong passphrase or corrupted file — skip silently
                pass

    def _save_contact(self, contact: Contact) -> None:
        """Encrypt and write a single contact to disk."""
        plaintext = self._contact_to_json(contact)
        sealed = encrypt(plaintext, self._storage_key)
        enc_file = self.keyring_dir / f"{contact.contact_id}.enc"
        enc_file.write_bytes(sealed)

    def _contact_to_json(self, contact: Contact) -> bytes:
        """Serialize contact to JSON bytes (including key as base64)."""
        data = {
            "contact_id": contact.contact_id,
            "display_name": contact.display_name,
            "key": key_to_base64(contact.key),
            "telegram_channel_id": contact.telegram_channel_id,
            "telegram_user_id": contact.telegram_user_id,
            "created_at": contact.created_at.isoformat(),
            "envelope_template": contact.envelope_template,
        }
        return json.dumps(data).encode("utf-8")

    def _contact_from_json(self, data: bytes) -> Contact:
        """Deserialize contact from JSON bytes."""
        obj = json.loads(data.decode("utf-8"))
        return Contact(
            contact_id=obj["contact_id"],
            display_name=obj["display_name"],
            key=key_from_base64(obj["key"]),
            telegram_channel_id=obj["telegram_channel_id"],
            telegram_user_id=obj["telegram_user_id"],
            created_at=datetime.fromisoformat(obj["created_at"]),
            envelope_template=obj.get("envelope_template", ""),
        )

    def add_contact(self, contact: Contact) -> None:
        """Add a new contact and persist to disk."""
        self._contacts[contact.contact_id] = contact
        self._save_contact(contact)

    def remove_contact(self, contact_id: str) -> None:
        """Remove a contact and delete its keyring file."""
        self._contacts.pop(contact_id, None)
        enc_file = self.keyring_dir / f"{contact_id}.enc"
        if enc_file.exists():
            enc_file.unlink()

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
