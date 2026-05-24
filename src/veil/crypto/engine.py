import nacl.utils
from nacl.secret import SecretBox


def encrypt(plaintext: bytes, key: bytes) -> bytes:
    """
    Encrypt plaintext with XSalsa20-Poly1305.

    Args:
        plaintext: Raw bytes to encrypt.
        key: 32-byte (256-bit) symmetric key.

    Returns:
        Sealed bytes: nonce (24 bytes) + ciphertext + auth tag (16 bytes).
        PyNaCl's SecretBox.encrypt() handles nonce prepending automatically.

    Raises:
        nacl.exceptions.CryptoError: If key is wrong size.
    """
    box = SecretBox(key)
    return box.encrypt(plaintext)


def decrypt(sealed: bytes, key: bytes) -> bytes:
    """
    Decrypt sealed bytes. Verifies authentication tag.

    Args:
        sealed: Output from encrypt() — nonce + ciphertext + auth tag.
        key: Same 32-byte key used for encryption.

    Returns:
        Original plaintext bytes.

    Raises:
        nacl.exceptions.CryptoError: If key is wrong, data is tampered,
                                      or sealed format is invalid.
    """
    box = SecretBox(key)
    return box.decrypt(sealed)
