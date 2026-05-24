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
