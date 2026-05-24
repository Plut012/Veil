"""Tests for envelope wrap/unwrap."""
import pytest
from veil.envelope import wrap, unwrap


class TestWrap:
    def test_simple_template(self):
        assert wrap("abc123", "~~ {ciphertext} ~~") == "~~ abc123 ~~"

    def test_prefix_template(self):
        assert wrap("abc123", "[ veil ] {ciphertext}") == "[ veil ] abc123"

    def test_empty_template(self):
        """Empty template returns raw ciphertext."""
        assert wrap("abc123", "") == "abc123"

    def test_no_placeholder_in_template(self):
        """Template without placeholder returns the template as-is (edge case)."""
        assert wrap("abc123", "no placeholder here") == "no placeholder here"

    def test_emoji_template(self):
        assert wrap("abc123", "🌿 {ciphertext} 🌿") == "🌿 abc123 🌿"


class TestUnwrap:
    def test_match_simple(self):
        msg = "~~ abc123 ~~"
        assert unwrap(msg, ["~~ {ciphertext} ~~"]) == "abc123"

    def test_match_prefix(self):
        msg = "[ veil ] abc123def"
        assert unwrap(msg, ["[ veil ] {ciphertext}"]) == "abc123def"

    def test_match_empty_template(self):
        """Raw ciphertext (no wrapping) is matched by empty template."""
        msg = "abc123XYZ"
        assert unwrap(msg, [""]) == "abc123XYZ"

    def test_no_match_returns_none(self):
        """Non-Veil message returns None."""
        msg = "hey what's up"
        assert unwrap(msg, ["~~ {ciphertext} ~~"]) is None

    def test_multiple_templates_first_match(self):
        """First matching template wins."""
        msg = "~~ abc123 ~~"
        result = unwrap(msg, ["[ veil ] {ciphertext}", "~~ {ciphertext} ~~"])
        assert result == "abc123"

    def test_emoji_template_match(self):
        msg = "🌿 abc123 🌿"
        assert unwrap(msg, ["🌿 {ciphertext} 🌿"]) == "abc123"

    def test_real_base64_ciphertext(self):
        """Realistic base64 ciphertext extracts correctly."""
        import base64
        ciphertext_b64 = base64.urlsafe_b64encode(b"x" * 40).decode()
        msg = wrap(ciphertext_b64, ":: {ciphertext} ::")
        assert unwrap(msg, [":: {ciphertext} ::"]) == ciphertext_b64

    def test_partial_match_not_confused(self):
        """Message that looks similar but doesn't match template returns None."""
        msg = "~~ not base64!!! ~~"
        # Contains spaces and '!' which aren't in the base64 charset
        assert unwrap(msg, ["~~ {ciphertext} ~~"]) is None
