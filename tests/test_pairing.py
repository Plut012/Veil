"""Tests for pairing ceremony logic. No Telegram connection required."""
import pytest
import json
from veil.identity.pairing import (
    create_qr_payload,
    parse_qr_payload,
    build_handshake_message,
    parse_handshake_message,
)
from veil.crypto import generate_key, key_to_base64


class TestQRPayload:
    def test_create_produces_png(self):
        """QR code output is valid PNG bytes."""
        payload, qr_bytes = create_qr_payload(
            telegram_user_id=12345,
            display_name="Alice",
        )
        assert qr_bytes[:8] == b"\x89PNG\r\n\x1a\n"  # PNG magic bytes

    def test_create_generates_key(self):
        """Each call generates a unique 32-byte key."""
        p1, _ = create_qr_payload(12345, "Alice")
        p2, _ = create_qr_payload(12345, "Alice")
        assert len(p1.key) == 32
        assert p1.key != p2.key

    def test_roundtrip_qr_payload(self):
        """Created QR data can be parsed back."""
        payload, qr_bytes = create_qr_payload(
            telegram_user_id=12345,
            display_name="Alice",
            envelope_template="~~ {ciphertext} ~~",
        )

        # Simulate scanning: extract JSON from QR
        # (In reality, QR scanner returns this string)
        qr_json = json.dumps({
            "v": 1,
            "key": key_to_base64(payload.key),
            "tid": 12345,
            "name": "Alice",
            "env": "~~ {ciphertext} ~~",
        })

        parsed = parse_qr_payload(qr_json)
        assert parsed.key == payload.key
        assert parsed.telegram_user_id == 12345
        assert parsed.display_name == "Alice"
        assert parsed.envelope_template == "~~ {ciphertext} ~~"

    def test_invalid_version_rejected(self):
        data = json.dumps({"v": 99, "key": "x", "tid": 1, "name": "x"})
        with pytest.raises(ValueError, match="Unsupported"):
            parse_qr_payload(data)

    def test_missing_fields_rejected(self):
        data = json.dumps({"v": 1, "key": "x"})  # missing tid, name
        with pytest.raises(KeyError):
            parse_qr_payload(data)


class TestHandshake:
    def test_build_and_parse(self):
        """Handshake roundtrips through encrypt/decrypt."""
        key = generate_key()
        msg = build_handshake_message(key, "Bob", "[ v ] {ciphertext}")
        parsed = parse_handshake_message(msg, key)
        assert parsed is not None
        assert parsed["name"] == "Bob"
        assert parsed["env"] == "[ v ] {ciphertext}"
        assert parsed["type"] == "veil_handshake"

    def test_wrong_key_returns_none(self):
        """Handshake with wrong key returns None (not a crash)."""
        key1 = generate_key()
        key2 = generate_key()
        msg = build_handshake_message(key1, "Bob")
        assert parse_handshake_message(msg, key2) is None

    def test_non_handshake_returns_none(self):
        """Random string is not a handshake."""
        key = generate_key()
        assert parse_handshake_message("just a normal message", key) is None

    def test_empty_envelope_template(self):
        """Handshake with no envelope template works."""
        key = generate_key()
        msg = build_handshake_message(key, "Bob")
        parsed = parse_handshake_message(msg, key)
        assert parsed is not None
        assert parsed.get("env", "") == ""
