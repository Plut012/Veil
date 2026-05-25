use tokio::sync::Mutex;

use crate::app::config::VeilConfig;
use crate::bridges::TelegramBridge;
use crate::identity::IdentityStore;
use crate::identity::pairing::QrPayload;

/// Shared application state, managed by Tauri's state system.
///
/// All fields are `Mutex`-wrapped for async access from multiple Tauri command handlers.
pub struct VeilState {
    pub config: Mutex<VeilConfig>,
    pub store: Mutex<IdentityStore>,
    pub bridge: Mutex<TelegramBridge>,
    /// Set when we are in the "Show QR" / initiator pairing role.
    /// Cleared after the handshake is received and processed.
    pub pending_pairing: Mutex<Option<QrPayload>>,
    /// Path to the config file so commands can persist changes.
    pub config_path: std::path::PathBuf,
}

impl VeilState {
    pub fn new(
        config: VeilConfig,
        store: IdentityStore,
        bridge: TelegramBridge,
        config_path: std::path::PathBuf,
    ) -> Self {
        Self {
            config: Mutex::new(config),
            store: Mutex::new(store),
            bridge: Mutex::new(bridge),
            pending_pairing: Mutex::new(None),
            config_path,
        }
    }
}
