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
