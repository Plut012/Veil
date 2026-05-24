# Phase 3: Envelope

**Status:** Not Started
**Dependencies:** None (standalone module, but tested after Phase 1 for base64 utilities)
**Output:** `src/veil/envelope/format.py`, `tests/test_envelope.py`

---

## Purpose

The envelope module wraps ciphertext in user-configurable templates before sending, and extracts ciphertext from incoming messages by matching against known templates. This gives encrypted messages personality — your ciphertext shows up however you want it to.

---

## Files

### `src/veil/envelope/format.py`

```python
import re

CIPHERTEXT_PLACEHOLDER = "{ciphertext}"


def wrap(ciphertext_b64: str, template: str) -> str:
    """
    Wrap base64-encoded ciphertext in a user-defined template.

    Args:
        ciphertext_b64: Base64-encoded sealed bytes.
        template: Template string containing {ciphertext} placeholder.
                  If template is empty, returns raw ciphertext.

    Returns:
        Formatted envelope string.

    Examples:
        wrap("abc123", "~~ {ciphertext} ~~")  → "~~ abc123 ~~"
        wrap("abc123", "[ veil ] {ciphertext}") → "[ veil ] abc123"
        wrap("abc123", "")  → "abc123"
    """
    if not template:
        return ciphertext_b64
    return template.replace(CIPHERTEXT_PLACEHOLDER, ciphertext_b64)


def build_pattern(template: str) -> re.Pattern:
    """
    Convert a template into a regex pattern for extraction.

    The {ciphertext} placeholder becomes a capture group matching
    URL-safe base64 characters (A-Za-z0-9_-=+/).

    Args:
        template: Template string containing {ciphertext}.

    Returns:
        Compiled regex with one capture group for the ciphertext.
    """
    if not template:
        # No template — the entire message is the ciphertext
        return re.compile(r"^([A-Za-z0-9_\-=+/]+)$")

    # Escape everything except the placeholder
    parts = template.split(CIPHERTEXT_PLACEHOLDER)
    escaped = [re.escape(p) for p in parts]
    pattern = r"([A-Za-z0-9_\-=+/]+)".join(escaped)
    return re.compile(pattern)


def unwrap(message: str, templates: list[str]) -> str | None:
    """
    Try to extract ciphertext from a message using known templates.

    Args:
        message: Raw message text received from the platform.
        templates: List of known envelope templates (from paired contacts).
                   Includes empty string for contacts with no template.

    Returns:
        Extracted base64 ciphertext string, or None if no template matched.
        None means this message is not a Veil message — ignore it.
    """
    for template in templates:
        pattern = build_pattern(template)
        match = pattern.search(message)
        if match:
            return match.group(1)
    return None
```

**Implementation notes:**
- Templates are simple string substitution — no Jinja, no special syntax. Just `{ciphertext}` as a placeholder.
- `unwrap` tries each known template in order. First match wins. This is fine for small contact lists. If ambiguity becomes an issue, templates should be made more distinctive — but that's a user concern, not an engineering one.
- Base64 character class: `[A-Za-z0-9_\-=+/]` covers both standard and URL-safe base64.
- Empty template means "no wrapping" — the entire message is treated as potential ciphertext.

### `src/veil/envelope/__init__.py`

```python
from veil.envelope.format import wrap, unwrap
```

### `tests/test_envelope.py`

```python
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
```

---

## Acceptance Criteria

- [ ] `wrap(ciphertext, template)` substitutes `{ciphertext}` placeholder and returns formatted string
- [ ] `wrap` with empty template returns raw ciphertext
- [ ] `unwrap(message, templates)` extracts ciphertext from first matching template
- [ ] `unwrap` returns `None` for non-Veil messages
- [ ] Templates with emojis and special characters work correctly
- [ ] Base64 ciphertext roundtrips through wrap/unwrap
- [ ] All tests pass: `uv run pytest tests/test_envelope.py -v`
