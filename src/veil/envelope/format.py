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
