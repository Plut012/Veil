import nacl.utils
import base64

KEY_SIZE = 32  # 256 bits


def generate_key() -> bytes:
    """Generate a cryptographically random 256-bit key."""
    return nacl.utils.random(KEY_SIZE)


def key_to_base64(key: bytes) -> str:
    """Encode key as URL-safe base64 string (for QR codes, config)."""
    return base64.urlsafe_b64encode(key).decode("ascii")


def key_from_base64(encoded: str) -> bytes:
    """Decode key from URL-safe base64 string."""
    key = base64.urlsafe_b64decode(encoded.encode("ascii"))
    if len(key) != KEY_SIZE:
        raise ValueError(f"Invalid key size: expected {KEY_SIZE}, got {len(key)}")
    return key
