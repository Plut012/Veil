# Tauri Phase 5: Telegram Bridge (Rust)

**Status:** Not Started
**Dependencies:** Phase 1 (project compiles)
**Output:** `src-tauri/src/bridges/traits.rs`, `src-tauri/src/bridges/telegram.rs`, unit tests

---

## Purpose

Implement the Telegram bridge in Rust using `grammers-client`. Same interface as the Python version: connect, send, receive, create channels. The bridge never sees plaintext — it moves opaque envelope strings.

---

## Dependencies (Cargo.toml)

```toml
grammers-client = "0.7"
grammers-session = "0.6"
grammers-tl-types = "0.7"
log = "0.4"
```

**Note:** Check crates.io for the latest compatible versions of the grammers family. They must all be from the same release cycle.

---

## Files

### `src-tauri/src/bridges/traits.rs`

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Bridge: Send + Sync {
    async fn connect(&mut self) -> Result<(), BridgeError>;
    async fn disconnect(&mut self) -> Result<(), BridgeError>;
    async fn send(&self, channel_id: i64, envelope: &str) -> Result<(), BridgeError>;
    async fn create_channel(&self, user_ids: &[i64], name: &str) -> Result<i64, BridgeError>;
    async fn get_self_user_id(&self) -> Result<i64, BridgeError>;
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("not connected")]
    NotConnected,
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("send failed: {0}")]
    SendFailed(String),
    #[error("channel creation failed: {0}")]
    ChannelCreationFailed(String),
}
```

**Note:** `on_receive` is handled differently in Rust — instead of a callback, we use a `tokio::sync::mpsc` channel or Tauri events. The bridge spawns a background task that listens for incoming messages and forwards them through a channel.

### `src-tauri/src/bridges/telegram.rs`

```rust
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use grammers_client::{Client, Config, InitParams, Update};
use grammers_session::Session;

use crate::bridges::traits::{Bridge, BridgeError};

/// Message received from Telegram
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub channel_id: i64,
    pub text: String,
    pub sender_id: i64,
}

pub struct TelegramBridge {
    api_id: i32,
    api_hash: String,
    session_path: PathBuf,
    client: Option<Client>,
    self_user_id: Option<i64>,
    pub monitored_channels: Arc<Mutex<HashSet<i64>>>,
    pub passthrough: Arc<Mutex<bool>>,
    message_tx: mpsc::UnboundedSender<IncomingMessage>,
}

impl TelegramBridge {
    pub fn new(
        api_id: i32,
        api_hash: String,
        session_path: PathBuf,
        monitored_channels: HashSet<i64>,
        message_tx: mpsc::UnboundedSender<IncomingMessage>,
    ) -> Self { ... }

    /// Spawn a background task that listens for incoming messages
    pub fn spawn_listener(&self) {
        let client = self.client.clone().expect("must be connected");
        let monitored = self.monitored_channels.clone();
        let passthrough = self.passthrough.clone();
        let self_id = self.self_user_id.unwrap();
        let tx = self.message_tx.clone();

        tokio::spawn(async move {
            loop {
                match client.next_update().await {
                    Ok(Update::NewMessage(message)) if !message.outgoing() => {
                        let sender_id = message.sender().map(|s| s.id()).unwrap_or(0);
                        if sender_id == self_id {
                            continue; // skip own messages
                        }

                        let chat_id = message.chat().id();
                        let passthrough_on = *passthrough.lock().await;
                        let is_monitored = monitored.lock().await.contains(&chat_id);

                        if passthrough_on || is_monitored {
                            let _ = tx.send(IncomingMessage {
                                channel_id: chat_id,
                                text: message.text().to_string(),
                                sender_id,
                            });
                        }
                    }
                    Ok(_) => {} // ignore other updates
                    Err(e) => {
                        log::error!("Telegram update error: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

#[async_trait::async_trait]
impl Bridge for TelegramBridge {
    async fn connect(&mut self) -> Result<(), BridgeError> {
        // Load or create session
        // Connect to Telegram
        // If not authorized, handle sign-in flow
        // Store self_user_id
        // Call spawn_listener()
        ...
    }

    async fn disconnect(&mut self) -> Result<(), BridgeError> { ... }

    async fn send(&self, channel_id: i64, envelope: &str) -> Result<(), BridgeError> {
        let client = self.client.as_ref().ok_or(BridgeError::NotConnected)?;
        // Resolve chat by ID and send message
        ...
    }

    async fn create_channel(&self, user_ids: &[i64], name: &str) -> Result<i64, BridgeError> {
        // Create group chat, add users, return chat ID
        ...
    }

    async fn get_self_user_id(&self) -> Result<i64, BridgeError> {
        self.self_user_id.ok_or(BridgeError::NotConnected)
    }
}
```

**Implementation notes:**

- **Message receiving** uses `mpsc::UnboundedSender` instead of a callback. The bridge spawns a listener task that pushes `IncomingMessage` structs through the channel. The app layer reads from the receiver.
- **Self-message filtering** is built in: `message.outgoing()` check + `sender_id == self_id` check.
- **Passthrough** flag uses `Arc<Mutex<bool>>` so the app layer can toggle it from another task.
- **Authentication** on first connect requires interactive input (phone number + code). Use `std::io::stdin` for terminal input during the first-run flow. Tauri can later provide a UI for this.
- **grammers session** is saved to disk at `session_path`. Subsequent connections are automatic.

### `src-tauri/src/bridges/mod.rs`

```rust
pub mod traits;
pub mod telegram;

pub use traits::{Bridge, BridgeError};
pub use telegram::{TelegramBridge, IncomingMessage};
```

### Tests

Unit tests for non-connection logic only:

```rust
#[cfg(test)]
mod tests {
    // Test monitored channel set management
    // Test passthrough flag
    // Test that IncomingMessage struct is correct
    // Cannot test actual Telegram connection without credentials
}
```

---

## grammers Authentication Flow

```rust
async fn connect(&mut self) -> Result<(), BridgeError> {
    let session = Session::load_file_or_create(&self.session_path)
        .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

    let client = Client::connect(Config {
        session,
        api_id: self.api_id,
        api_hash: self.api_hash.clone(),
        params: InitParams::default(),
    }).await.map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

    if !client.is_authorized().await.unwrap_or(false) {
        // Interactive sign-in
        println!("Telegram sign-in required.");
        // ... prompt for phone, request code, sign in
    }

    let me = client.get_me().await
        .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;
    self.self_user_id = Some(me.id());

    client.session().save_to_file(&self.session_path)
        .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

    self.client = Some(client);
    self.spawn_listener();

    Ok(())
}
```

---

## Acceptance Criteria

- [ ] `Bridge` trait defined with all required methods
- [ ] `TelegramBridge` implements `Bridge` trait
- [ ] `connect()` handles session load/create and authentication
- [ ] Self-message filtering works (outgoing + sender_id check)
- [ ] Passthrough flag toggleable from outside the bridge
- [ ] Monitored channel set management (add/remove)
- [ ] Message listener spawns as background task, sends through mpsc channel
- [ ] Operations on disconnected bridge return `BridgeError::NotConnected`
- [ ] `cargo test` passes all bridge unit tests
