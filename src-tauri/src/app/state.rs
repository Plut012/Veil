use std::path::PathBuf;

use grammers_client::client::{LoginToken, PasswordToken};
use tokio::sync::{mpsc, Mutex};

use crate::app::config::VeilConfig;
use crate::bridges::{IncomingMessage, TelegramBridge};
use crate::identity::IdentityStore;
use crate::identity::pairing::QrPayload;

/// Shared application state, managed by Tauri's state system.
///
/// All fields are `Mutex`-wrapped for async access from multiple Tauri command handlers.
/// `store` and `bridge` are `Option` — they are `None` until setup completes.
pub struct VeilState {
    pub config: Mutex<VeilConfig>,
    pub config_path: PathBuf,
    pub veil_dir: PathBuf,
    /// Initialized after passphrase is submitted.
    pub store: Mutex<Option<IdentityStore>>,
    /// Initialized after passphrase is submitted.
    pub bridge: Mutex<Option<TelegramBridge>>,
    /// Set when we are in the "Show QR" / initiator pairing role.
    /// Cleared after the handshake is received and processed.
    pub pending_pairing: Mutex<Option<QrPayload>>,
    /// Channel for forwarding incoming messages from the bridge listener task.
    pub message_tx: mpsc::UnboundedSender<IncomingMessage>,
    /// Intermediate auth state: stored between request_telegram_code and submit_telegram_code.
    pub login_token: Mutex<Option<LoginToken>>,
    /// Intermediate auth state: stored between submit_telegram_code (2FA) and submit_2fa_password.
    pub password_token: Mutex<Option<PasswordToken>>,
}

impl VeilState {
    pub fn new(
        config: VeilConfig,
        config_path: PathBuf,
        veil_dir: PathBuf,
        message_tx: mpsc::UnboundedSender<IncomingMessage>,
    ) -> Self {
        Self {
            config: Mutex::new(config),
            config_path,
            veil_dir,
            store: Mutex::new(None),
            bridge: Mutex::new(None),
            pending_pairing: Mutex::new(None),
            message_tx,
            login_token: Mutex::new(None),
            password_token: Mutex::new(None),
        }
    }
}
