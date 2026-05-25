# Tauri Phase 3: Envelope (Rust)

**Status:** Not Started
**Dependencies:** Phase 1 (project compiles)
**Output:** `src-tauri/src/envelope/format.rs`, unit tests

---

## Purpose

Port the envelope module to Rust. Wraps ciphertext in user-configurable templates, extracts ciphertext from incoming messages by matching against known templates.

---

## Dependencies (Cargo.toml)

```toml
regex = "1"
```

---

## Files

### `src-tauri/src/envelope/format.rs`

```rust
use regex::Regex;

const PLACEHOLDER: &str = "{ciphertext}";

/// Wrap base64-encoded ciphertext in a user-defined template.
pub fn wrap(ciphertext_b64: &str, template: &str) -> String {
    if template.is_empty() {
        return ciphertext_b64.to_string();
    }
    template.replace(PLACEHOLDER, ciphertext_b64)
}

/// Build a regex pattern from a template for ciphertext extraction.
fn build_pattern(template: &str) -> Regex {
    if template.is_empty() {
        return Regex::new(r"^([A-Za-z0-9_\-=+/]+)$").unwrap();
    }
    let parts: Vec<&str> = template.split(PLACEHOLDER).collect();
    let escaped: Vec<String> = parts.iter().map(|p| regex::escape(p)).collect();
    let pattern = escaped.join(r"([A-Za-z0-9_\-=+/]+)");
    Regex::new(&pattern).unwrap()
}

/// Try to extract ciphertext from a message using known templates.
/// Returns None if no template matches (not a Veil message).
pub fn unwrap(message: &str, templates: &[&str]) -> Option<String> {
    for template in templates {
        let pattern = build_pattern(template);
        if let Some(caps) = pattern.captures(message) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}
```

### `src-tauri/src/envelope/mod.rs`

```rust
pub mod format;
pub use format::{wrap, unwrap};
```

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_simple() { ... }
    #[test]
    fn test_wrap_empty_template() { ... }
    #[test]
    fn test_unwrap_match() { ... }
    #[test]
    fn test_unwrap_no_match_returns_none() { ... }
    #[test]
    fn test_unwrap_multiple_templates() { ... }
    #[test]
    fn test_roundtrip() { ... }
    #[test]
    fn test_emoji_template() { ... }
}
```

---

## Acceptance Criteria

- [ ] `wrap(ciphertext, template)` substitutes placeholder and returns formatted string
- [ ] Empty template returns raw ciphertext
- [ ] `unwrap(message, templates)` extracts ciphertext from first matching template
- [ ] `unwrap` returns `None` for non-Veil messages
- [ ] Templates with special regex characters work correctly (brackets, dots)
- [ ] `cargo test` passes all envelope tests
