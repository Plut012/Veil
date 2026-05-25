use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub session_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeilConfig {
    pub display_name: String,
    pub envelope_template: String,
    pub theme: String,
    pub telegram: TelegramConfig,
}

impl VeilConfig {
    /// Load config from a TOML file. Returns `Default` if the file doesn't exist.
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to a TOML file, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        std::fs::write(path, contents)
    }
}

impl Default for VeilConfig {
    fn default() -> Self {
        Self {
            display_name: "Veil User".into(),
            envelope_template: String::new(),
            theme: "art-nouveau".into(),
            telegram: TelegramConfig {
                api_id: 0,
                api_hash: String::new(),
                session_path: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".veil/telegram.session")
                    .to_string_lossy()
                    .into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "veil_config_test_{}.toml",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ))
    }

    #[test]
    fn test_default_config() {
        let cfg = VeilConfig::default();
        assert_eq!(cfg.display_name, "Veil User");
        assert_eq!(cfg.theme, "art-nouveau");
        assert_eq!(cfg.telegram.api_id, 0);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let path = tmp_path();
        let mut cfg = VeilConfig::default();
        cfg.display_name = "Alice".into();
        cfg.theme = "art-nouveau".into();
        cfg.telegram.api_id = 12345;
        cfg.telegram.api_hash = "abc123".into();

        cfg.save(&path).expect("save failed");
        assert!(path.exists());

        let loaded = VeilConfig::load(&path);
        assert_eq!(loaded.display_name, "Alice");
        assert_eq!(loaded.telegram.api_id, 12345);
        assert_eq!(loaded.telegram.api_hash, "abc123");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent_veil_config_xyz.toml");
        let cfg = VeilConfig::load(&path);
        assert_eq!(cfg.display_name, "Veil User");
    }
}
