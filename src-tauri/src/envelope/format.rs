use regex::Regex;

const PLACEHOLDER: &str = "{ciphertext}";

/// Wrap base64-encoded ciphertext in a user-defined template.
/// Empty template returns raw ciphertext unchanged.
pub fn wrap(ciphertext_b64: &str, template: &str) -> String {
    if template.is_empty() {
        return ciphertext_b64.to_string();
    }
    template.replace(PLACEHOLDER, ciphertext_b64)
}

/// Build a regex pattern from a template for ciphertext extraction.
fn build_pattern(template: &str) -> Regex {
    if template.is_empty() {
        return Regex::new(r"^([A-Za-z0-9_\-=+/]+)$").expect("hardcoded pattern is valid");
    }
    let parts: Vec<&str> = template.split(PLACEHOLDER).collect();
    let escaped: Vec<String> = parts.iter().map(|p| regex::escape(p)).collect();
    let pattern = escaped.join(r"([A-Za-z0-9_\-=+/]+)");
    Regex::new(&pattern).expect("generated pattern is valid")
}

/// Try to extract ciphertext from a message using known templates.
/// Returns the ciphertext from the first matching template, or None if no match.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_simple() {
        let result = wrap("abc123", "Encrypted: {ciphertext}");
        assert_eq!(result, "Encrypted: abc123");
    }

    #[test]
    fn test_wrap_prefix_template() {
        let result = wrap("XYZ789", "[veil]{ciphertext}[/veil]");
        assert_eq!(result, "[veil]XYZ789[/veil]");
    }

    #[test]
    fn test_wrap_empty_template() {
        let result = wrap("abc123", "");
        assert_eq!(result, "abc123");
    }

    #[test]
    fn test_unwrap_match() {
        let message = "Encrypted: abc123";
        let templates = ["Encrypted: {ciphertext}"];
        let result = unwrap(message, &templates);
        assert_eq!(result, Some("abc123".to_string()));
    }

    #[test]
    fn test_unwrap_no_match_returns_none() {
        let message = "Hello world";
        let templates = ["Encrypted: {ciphertext}"];
        let result = unwrap(message, &templates);
        assert_eq!(result, None);
    }

    #[test]
    fn test_unwrap_multiple_templates_first_match_wins() {
        let ciphertext = "abc123";
        let message = format!("[veil]{ciphertext}[/veil]");
        let templates = ["Encrypted: {ciphertext}", "[veil]{ciphertext}[/veil]"];
        let result = unwrap(&message, &templates);
        assert_eq!(result, Some(ciphertext.to_string()));
    }

    #[test]
    fn test_emoji_template_wrap() {
        let result = wrap("abc123", "🔒 {ciphertext} 🔒");
        assert_eq!(result, "🔒 abc123 🔒");
    }

    #[test]
    fn test_emoji_template_unwrap() {
        let message = "🔒 abc123 🔒";
        let templates = ["🔒 {ciphertext} 🔒"];
        let result = unwrap(message, &templates);
        assert_eq!(result, Some("abc123".to_string()));
    }

    #[test]
    fn test_roundtrip() {
        let ciphertext = "base64EncodedCiphertext+/=";
        let template = "🔐 Secret: {ciphertext} :end";
        let wrapped = wrap(ciphertext, template);
        let recovered = unwrap(&wrapped, &[template]);
        assert_eq!(recovered, Some(ciphertext.to_string()));
    }

    #[test]
    fn test_regex_special_chars_in_template() {
        // Template contains brackets and dots which are regex special chars
        let template = "[msg]{ciphertext}...done";
        let ciphertext = "abc123XYZ";
        let wrapped = wrap(ciphertext, template);
        assert_eq!(wrapped, "[msg]abc123XYZ...done");
        let recovered = unwrap(&wrapped, &[template]);
        assert_eq!(recovered, Some(ciphertext.to_string()));
    }

    #[test]
    fn test_unwrap_empty_template_raw_ciphertext() {
        let ciphertext = "abc123XYZ";
        let result = unwrap(ciphertext, &[""]);
        assert_eq!(result, Some(ciphertext.to_string()));
    }
}
