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

    payload_json = json.dumps({
        "v": 1,
        "key": key_to_base64(key),
        "tid": telegram_user_id,
        "name": display_name,
        "env": envelope_template,
    })

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
        ValueError: If the QR data is invalid or the version is unsupported.
        KeyError: If required fields are missing.
    """
    data = json.loads(qr_data)

    if data.get("v") != 1:
        raise ValueError(f"Unsupported QR protocol version: {data.get('v')}")

    # Access all required fields first (raises KeyError if any are missing)
    key_b64 = data["key"]
    telegram_user_id = data["tid"]
    display_name = data["name"]

    return QRPayload(
        key=key_from_base64(key_b64),
        telegram_user_id=telegram_user_id,
        display_name=display_name,
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

    channel_id = await bridge.create_channel(
        user_ids=[payload.telegram_user_id],
        name=channel_name,
    )

    handshake = build_handshake_message(
        key=payload.key,
        display_name=display_name,
        envelope_template=envelope_template,
    )
    await bridge.send(channel_id, handshake)

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
        telegram_user_id=0,  # joiner's user ID is not in the handshake (known from Telegram event)
        envelope_template=handshake_data.get("env", ""),
    )
    store.add_contact(contact)
    bridge.add_monitored_channel(channel_id)

    logger.info(f"Paired with {handshake_data['name']} (initiator role)")
    return contact
