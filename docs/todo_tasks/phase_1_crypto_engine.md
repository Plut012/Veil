# Phase 1: Crypto Engine

**Status:** Not Started
**Dependencies:** None (first module — zero external dependencies within Veil)
**Output:** `src/veil/crypto/engine.py`, `src/veil/crypto/keys.py`, `tests/test_crypto.py`

---

## Purpose

The crypto engine is Veil's core. It provides symmetric encryption/decryption using XChaCha20-Poly1305 via PyNaCl (libsodium bindings). This module is pure — no side effects, no platform awareness, no file I/O. It receives bytes and returns bytes.

---

## Files

### `src/veil/crypto/engine.py`

Three functions. Nothing else.

```python
import nacl.utils
from nacl.secret import SecretBox


def encrypt(plaintext: bytes, key: bytes) -> bytes:
    """
    Encrypt plaintext with XChaCha20-Poly1305.

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
```

**Implementation notes:**
- PyNaCl's `SecretBox` uses XSalsa20-Poly1305 by default (24-byte nonce). This is the libsodium `crypto_secretbox` primitive — proven, audited, and the nonce is large enough that random generation is safe (no collision risk).
- `SecretBox.encrypt()` automatically generates a random nonce via `nacl.utils.random()` and prepends it to the output. No manual nonce handling needed.
- The auth tag is appended automatically. `decrypt()` verifies it before returning plaintext — any tampering raises `CryptoError`.
- We do NOT catch `CryptoError` — let it propagate. The caller decides how to handle decryption failures.

### `src/veil/crypto/keys.py`

Key generation and encoding utilities.

```python
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
```

**Implementation notes:**
- URL-safe base64 (uses `-` and `_` instead of `+` and `/`) because the key will be embedded in QR code payloads as JSON.
- `key_from_base64` validates length. A wrong-size key would fail at the `SecretBox` level anyway, but failing early with a clear message is better.

### `src/veil/crypto/__init__.py`

Clean public API — re-export the three core functions and the encoding utilities.

```python
from veil.crypto.engine import encrypt, decrypt
from veil.crypto.keys import generate_key, key_to_base64, key_from_base64
```

### `tests/test_crypto.py`

Real crypto operations. No mocking.

```python
"""Tests for the crypto engine. All tests use real encryption — no mocks."""
import pytest
from nacl.exceptions import CryptoError
from veil.crypto import encrypt, decrypt, generate_key, key_to_base64, key_from_base64


class TestEncryptDecrypt:
    def test_roundtrip(self):
        """Encrypt then decrypt returns original plaintext."""
        key = generate_key()
        plaintext = b"hello veil"
        sealed = encrypt(plaintext, key)
        assert decrypt(sealed, key) == plaintext

    def test_empty_message(self):
        """Empty plaintext encrypts and decrypts correctly."""
        key = generate_key()
        sealed = encrypt(b"", key)
        assert decrypt(sealed, key) == b""

    def test_unicode_message(self):
        """UTF-8 encoded text survives roundtrip."""
        key = generate_key()
        plaintext = "encrypted with love 🔒".encode("utf-8")
        sealed = encrypt(plaintext, key)
        assert decrypt(sealed, key) == plaintext

    def test_large_message(self):
        """Large payloads encrypt correctly."""
        key = generate_key()
        plaintext = b"x" * 100_000
        sealed = encrypt(plaintext, key)
        assert decrypt(sealed, key) == plaintext

    def test_wrong_key_fails(self):
        """Decrypting with wrong key raises CryptoError."""
        key1 = generate_key()
        key2 = generate_key()
        sealed = encrypt(b"secret", key1)
        with pytest.raises(CryptoError):
            decrypt(sealed, key2)

    def test_tampered_ciphertext_fails(self):
        """Modifying sealed bytes raises CryptoError."""
        key = generate_key()
        sealed = encrypt(b"secret", key)
        tampered = bytearray(sealed)
        tampered[-1] ^= 0xFF  # flip last byte (in auth tag)
        with pytest.raises(CryptoError):
            decrypt(bytes(tampered), key)

    def test_truncated_ciphertext_fails(self):
        """Truncated sealed bytes raises CryptoError."""
        key = generate_key()
        sealed = encrypt(b"secret", key)
        with pytest.raises(CryptoError):
            decrypt(sealed[:10], key)

    def test_unique_nonces(self):
        """Each encryption produces different output (random nonce)."""
        key = generate_key()
        plaintext = b"same message"
        sealed1 = encrypt(plaintext, key)
        sealed2 = encrypt(plaintext, key)
        assert sealed1 != sealed2  # different nonces
        assert decrypt(sealed1, key) == decrypt(sealed2, key)  # same plaintext

    def test_sealed_format_size(self):
        """Sealed output is nonce (24) + plaintext + MAC (16) bytes."""
        key = generate_key()
        plaintext = b"hello"
        sealed = encrypt(plaintext, key)
        expected_size = 24 + len(plaintext) + 16
        assert len(sealed) == expected_size


class TestKeyGeneration:
    def test_key_length(self):
        """Generated key is 32 bytes (256 bits)."""
        key = generate_key()
        assert len(key) == 32

    def test_keys_are_unique(self):
        """Each generated key is different."""
        keys = {generate_key() for _ in range(100)}
        assert len(keys) == 100


class TestKeyEncoding:
    def test_roundtrip(self):
        """Key survives base64 encode/decode."""
        key = generate_key()
        encoded = key_to_base64(key)
        assert key_from_base64(encoded) == key

    def test_encoded_is_string(self):
        """Encoded key is an ASCII string."""
        encoded = key_to_base64(generate_key())
        assert isinstance(encoded, str)
        encoded.encode("ascii")  # should not raise

    def test_invalid_length_rejected(self):
        """Base64 string that decodes to wrong length is rejected."""
        import base64
        bad = base64.urlsafe_b64encode(b"too short").decode()
        with pytest.raises(ValueError, match="Invalid key size"):
            key_from_base64(bad)
```

---

## Acceptance Criteria

- [ ] `encrypt(plaintext, key)` returns sealed bytes (nonce + ciphertext + auth tag)
- [ ] `decrypt(sealed, key)` returns original plaintext
- [ ] Wrong key raises `CryptoError`
- [ ] Tampered ciphertext raises `CryptoError`
- [ ] Each encryption of the same plaintext produces different output (random nonce)
- [ ] `generate_key()` returns 32 random bytes
- [ ] Keys roundtrip through base64 encoding
- [ ] All tests pass: `uv run pytest tests/test_crypto.py -v`
- [ ] No file I/O, no side effects, no platform awareness in the crypto module
