# Phase 5: Pairing Ceremony

**Status:** Not Started
**Dependencies:** Phase 1 (Crypto), Phase 2 (Identity), Phase 3 (Envelope), Phase 4 (Bridge)
**Output:** `src/veil/identity/pairing.py`, `tests/test_pairing.py`

---

## Purpose

The pairing ceremony is the moment two people establish encrypted communication. It happens in person. One scan of a QR code bootstraps everything: key exchange, Telegram channel creation, and contact registration.

This module orchestrates the ceremony by wiring together crypto (key generation), identity (contact storage), and bridge (channel creation).

---

## The Ceremony

Two roles: **initiator** (shows QR) and **joiner** (scans QR).

```
INITIATOR (Alice):                         JOINER (Bob):
                                           
1. Taps "New Contact"                      
2. Veil generates 256-bit key              
3. Builds QR payload:                      
   {                                       
     "key": "<base64>",                    
     "telegram_user_id": 12345,            
     "display_name": "Alice",              
     "envelope_template": "~~ {c} ~~"      
   }                                       
4. Displays QR code on screen              
                                           5. Taps "Pair"
                                           6. Scans Alice's QR code
                                           7. Extracts payload
                                           8. Creates Telegram group, adds Alice
                                           9. Sends encrypted handshake message:
                                              encrypt({
                                                "type": "veil_handshake",
                                                "display_name": "Bob",
                                                "envelope_template": "..."
                                              })
                                           10. Saves contact (Alice) to keyring
                                           
11. Detects new group message              
12. Decrypts handshake                     
13. Saves contact (Bob) to keyring         
14. Pairing complete                       15. Pairing complete
```

---

## Files

### `src/veil/identity/pairing.py`

```python
import json
import io
import uuid
import base64
import logging
from dataclasses import dataclass

import qrcode

from veil.crypto import generate_key, key_to_base64, key_from_base64, encrypt, decrypt
from veil.identity.contact import Contact
from veil.identity.store import IdentityStore
from veil.bridges.base import Bridge

logger = logging.getLogger(__name__)

HANDSHAKE_TYPE = "veil_handshake"


@dataclass
class QRPayload:
    """Data encoded in the pairing QR code."""
    key: bytes                  # 256-bit symmetric key
    telegram_user_id: int       # initiator's Telegram user ID
    display_name: str           # initiator's display name
    envelope_template: str      # initiator's envelope template


def create_qr_payload(
    telegram_user_id: int,
    display_name: str,
    envelope_template: str = "",
) -> tuple[QRPayload, bytes]:
    """
    Generate a new QR payload for the initiator role.

    Returns:
        (payload, qr_image_png_bytes)
    """
    key = generate_key()
    payload = QRPayload(
        key=key,
        telegram_user_id=telegram_user_id,
        display_name=display_name,
        envelope_template=envelope_template,
    )

    # Serialize to JSON
    payload_json = json.dumps({
        "v": 1,  # protocol version
        "key": key_to_base64(key),
        "tid": telegram_user_id,
        "name": display_name,
        "env": envelope_template,
    })

    # Generate QR code as PNG bytes
    qr = qrcode.make(payload_json)
    buf = io.BytesIO()
    qr.save(buf, format="PNG")
    qr_bytes = buf.getvalue()

    return payload, qr_bytes


def parse_qr_payload(qr_data: str) -> QRPayload:
    """
    Parse scanned QR code data into a QRPayload.

    Args:
        qr_data: JSON string from the scanned QR code.

    Returns:
        Parsed QRPayload.

    Raises:
        ValueError: If the QR data is invalid or missing fields.
        KeyError: If required fields are missing.
    """
    data = json.loads(qr_data)

    if data.get("v") != 1:
        raise ValueError(f"Unsupported QR protocol version: {data.get('v')}")

    return QRPayload(
        key=key_from_base64(data["key"]),
        telegram_user_id=data["tid"],
        display_name=data["name"],
        envelope_template=data.get("env", ""),
    )


def build_handshake_message(
    key: bytes,
    display_name: str,
    envelope_template: str = "",
) -> str:
    """
    Build the encrypted handshake message sent by the joiner.

    Returns:
        Base64-encoded encrypted handshake (ready to send as envelope).
    """
    payload = json.dumps({
        "type": HANDSHAKE_TYPE,
        "name": display_name,
        "env": envelope_template,
    }).encode("utf-8")

    sealed = encrypt(payload, key)
    return base64.urlsafe_b64encode(sealed).decode("ascii")


def parse_handshake_message(message: str, key: bytes) -> dict | None:
    """
    Try to parse an incoming message as a Veil handshake.

    Returns:
        Parsed handshake dict with 'name' and 'env' fields,
        or None if this isn't a valid handshake.
    """
    try:
        sealed = base64.urlsafe_b64decode(message.encode("ascii"))
        plaintext = decrypt(sealed, key)
        data = json.loads(plaintext)
        if data.get("type") == HANDSHAKE_TYPE:
            return data
    except Exception:
        return None
    return None


async def initiate_pairing(
    store: IdentityStore,
    bridge: Bridge,
    display_name: str,
    envelope_template: str = "",
) -> tuple[QRPayload, bytes]:
    """
    Start the pairing ceremony as the initiator.

    Generates key and QR code. Does NOT save the contact yet —
    that happens when the handshake is received.

    Returns:
        (payload, qr_image_png_bytes) for display to the user.
    """
    telegram_user_id = await bridge.get_self_user_id()
    return create_qr_payload(telegram_user_id, display_name, envelope_template)


async def complete_pairing_as_joiner(
    qr_data: str,
    store: IdentityStore,
    bridge: Bridge,
    display_name: str,
    envelope_template: str = "",
    channel_name: str = "Veil",
) -> Contact:
    """
    Complete pairing as the joiner (the one who scans).

    1. Parse QR payload
    2. Create Telegram channel
    3. Send encrypted handshake
    4. Save contact

    Returns:
        The newly created Contact.
    """
    payload = parse_qr_payload(qr_data)

    # Create dedicated Telegram channel
    channel_id = await bridge.create_channel(
        user_ids=[payload.telegram_user_id],
        name=channel_name,
    )

    # Send handshake
    handshake = build_handshake_message(
        key=payload.key,
        display_name=display_name,
        envelope_template=envelope_template,
    )
    await bridge.send(channel_id, handshake)

    # Save contact
    contact = Contact(
        contact_id=str(uuid.uuid4()),
        display_name=payload.display_name,
        key=payload.key,
        telegram_channel_id=channel_id,
        telegram_user_id=payload.telegram_user_id,
        envelope_template=payload.envelope_template,
    )
    store.add_contact(contact)

    logger.info(f"Paired with {payload.display_name} (joiner role)")
    return contact


async def complete_pairing_as_initiator(
    pending_payload: QRPayload,
    channel_id: int,
    handshake_data: dict,
    store: IdentityStore,
    bridge: Bridge,
) -> Contact:
    """
    Complete pairing as the initiator when the handshake is received.

    Called by the app layer when it detects a valid handshake message
    in a new channel.

    Args:
        pending_payload: The QRPayload generated during initiate_pairing.
        channel_id: The Telegram channel the handshake arrived in.
        handshake_data: Parsed handshake dict (from parse_handshake_message).

    Returns:
        The newly created Contact.
    """
    contact = Contact(
        contact_id=str(uuid.uuid4()),
        display_name=handshake_data["name"],
        key=pending_payload.key,
        telegram_channel_id=channel_id,
        telegram_user_id=0,  # joiner's user ID is not in the handshake (known from Telegram)
        envelope_template=handshake_data.get("env", ""),
    )
    store.add_contact(contact)
    bridge.add_monitored_channel(channel_id)

    logger.info(f"Paired with {handshake_data['name']} (initiator role)")
    return contact
```

**Implementation notes:**

**QR payload format:**
- JSON string with version field (`"v": 1`) for future compatibility.
- Fields are abbreviated (`tid`, `env`) to keep QR data compact — large payloads produce denser QR codes that are harder to scan.
- The key is base64-encoded for JSON serialization.

**Handshake message:**
- The joiner sends the first message in the new channel, encrypted with the shared key.
- The message includes `"type": "veil_handshake"` so the initiator's Veil can distinguish it from normal messages.
- The handshake carries the joiner's display name and envelope template.

**Pending pairing state:**
- The initiator generates a QR and waits. The app layer must hold the `QRPayload` in memory until the handshake arrives.
- If the handshake never arrives (user cancels, scan fails), the pending state is discarded. No cleanup needed — nothing was persisted.

**QR code scanning:**
- Scanning is a frontend concern. The frontend uses the device camera (or a webcam on desktop), decodes the QR, and sends the JSON string to the backend via WebSocket.
- The `pyzbar` or similar library can be used server-side if needed, but browser-based scanning via `jsQR` or `html5-qrcode` is simpler for the frontend.

### `tests/test_pairing.py`

```python
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
```

---

## Acceptance Criteria

- [ ] `create_qr_payload` generates a 256-bit key and returns valid PNG QR code bytes
- [ ] `parse_qr_payload` correctly extracts key, user ID, display name, and envelope template
- [ ] QR payload roundtrips correctly (create → serialize → parse)
- [ ] Invalid QR protocol versions are rejected
- [ ] `build_handshake_message` produces an encrypted, base64-encoded message
- [ ] `parse_handshake_message` decrypts and validates the handshake
- [ ] Wrong key during handshake parse returns `None` (no crash)
- [ ] Non-handshake messages return `None`
- [ ] `complete_pairing_as_joiner` creates channel, sends handshake, saves contact
- [ ] `complete_pairing_as_initiator` saves contact when handshake is received
- [ ] All tests pass: `uv run pytest tests/test_pairing.py -v`
