use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use grammers_client::client::UpdatesConfiguration;
use grammers_client::update::Update;
use grammers_client::client::{LoginToken, PasswordToken};
use grammers_client::{Client, SenderPool, SignInError};
use grammers_session::Session as _;
use grammers_session::storages::SqliteSession;
use grammers_session::types::{PeerAuth, PeerId, PeerRef};
use grammers_tl_types as tl;
use tokio::sync::{Mutex, mpsc};

use crate::bridges::traits::{Bridge, BridgeError};

/// Message received from Telegram.
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub channel_id: i64,
    pub text: String,
    pub sender_id: i64,
}

/// Internal connected state, created on `connect_unauthenticated()`.
struct ConnectedState {
    client: Client,
    session: Arc<SqliteSession>,
    self_user_id: i64,
}

/// Telegram bridge implementing the [`Bridge`] trait via grammers-client 0.9.
pub struct TelegramBridge {
    api_id: i32,
    api_hash: String,
    session_path: PathBuf,
    state: Option<ConnectedState>,
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
    ) -> Self {
        Self {
            api_id,
            api_hash,
            session_path,
            state: None,
            monitored_channels: Arc::new(Mutex::new(monitored_channels)),
            passthrough: Arc::new(Mutex::new(false)),
            message_tx,
        }
    }

    /// Add a channel to the monitored set.
    pub async fn add_monitored_channel(&self, channel_id: i64) {
        self.monitored_channels.lock().await.insert(channel_id);
    }

    /// Remove a channel from the monitored set.
    pub async fn remove_monitored_channel(&self, channel_id: i64) {
        self.monitored_channels.lock().await.remove(&channel_id);
    }

    /// Spawn a background task that reads from the update stream and forwards
    /// relevant incoming messages through the mpsc channel.
    fn spawn_listener(
        client: Client,
        updates_rx: mpsc::UnboundedReceiver<grammers_session::updates::UpdatesLike>,
        monitored: Arc<Mutex<HashSet<i64>>>,
        passthrough: Arc<Mutex<bool>>,
        self_user_id: i64,
        tx: mpsc::UnboundedSender<IncomingMessage>,
    ) {
        tokio::spawn(async move {
            let mut stream = client
                .stream_updates(
                    updates_rx,
                    UpdatesConfiguration {
                        catch_up: false,
                        ..Default::default()
                    },
                )
                .await;

            loop {
                match stream.next().await {
                    Ok(Update::NewMessage(message)) if !message.outgoing() => {
                        // Filter out our own messages by sender_id.
                        let sender_peer_id = message.sender_id();
                        let sender_id = sender_peer_id
                            .map(|p| p.bare_id())
                            .unwrap_or(0);

                        if sender_id == self_user_id {
                            continue;
                        }

                        let chat_id = message.peer_id().bot_api_dialog_id().abs();
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
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("Telegram update error: {}", e);
                        break;
                    }
                }
            }
        });
    }

    /// Connect to Telegram (open session, start network I/O) without performing
    /// authentication. Returns `true` if already authorized, `false` if sign-in
    /// is required.
    pub async fn connect_unauthenticated(&mut self) -> Result<bool, BridgeError> {
        let session = SqliteSession::open(&self.session_path)
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;
        let session = Arc::new(session);

        let pool = SenderPool::new(Arc::clone(&session), self.api_id);
        // updates_rx is dropped here — we reconnect after auth to get a fresh channel.
        let _updates_rx = pool.updates;
        let runner = pool.runner;
        let client = Client::new(pool.handle);

        tokio::spawn(runner.run());

        let is_authorized = client
            .is_authorized()
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        // Store a temporary connected state (self_user_id = 0 until authorized).
        self.state = Some(ConnectedState {
            client,
            session,
            self_user_id: 0,
        });

        Ok(is_authorized)
    }

    /// Request a Telegram login code for the given phone number.
    /// Must call `connect_unauthenticated()` first.
    pub async fn request_login_code_for_phone(
        &self,
        phone: &str,
    ) -> Result<LoginToken, BridgeError> {
        let state = self.state.as_ref().ok_or(BridgeError::NotConnected)?;
        state
            .client
            .request_login_code(phone, &self.api_hash)
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))
    }

    /// Sign in with the received code. Returns `Ok(None)` on success, or
    /// `Ok(Some(PasswordToken))` if 2FA is required.
    ///
    /// On success the listener is spawned and `self_user_id` is populated.
    pub async fn sign_in_with_code(
        &mut self,
        token: LoginToken,
        code: &str,
    ) -> Result<Option<PasswordToken>, BridgeError> {
        let client = {
            let state = self.state.as_ref().ok_or(BridgeError::NotConnected)?;
            state.client.clone()
        };

        match client.sign_in(&token, code).await {
            Ok(_user) => {
                self.finalize_auth().await?;
                Ok(None)
            }
            Err(SignInError::PasswordRequired(password_token)) => {
                Ok(Some(password_token))
            }
            Err(e) => Err(BridgeError::ConnectionFailed(e.to_string())),
        }
    }

    /// Complete 2FA login with the given password.
    pub async fn check_2fa_password(
        &mut self,
        password_token: PasswordToken,
        password: &str,
    ) -> Result<(), BridgeError> {
        let client = {
            let state = self.state.as_ref().ok_or(BridgeError::NotConnected)?;
            state.client.clone()
        };

        client
            .check_password(password_token, password.as_bytes().to_vec())
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        self.finalize_auth().await
    }

    /// Finalize connection: reconnect with a fresh session (spawns listener).
    /// Call this after `connect_unauthenticated()` succeeds (session already authorised),
    /// or after sign-in. Closes the current connection and re-opens with the listener.
    pub async fn finalize_connection(&mut self) -> Result<(), BridgeError> {
        self.reconnect_after_auth().await
    }

    /// Finalize auth: reconnect with a fresh session (spawns listener).
    async fn finalize_auth(&mut self) -> Result<(), BridgeError> {
        self.reconnect_after_auth().await
    }

    /// Close the current connection and reconnect. After sign-in the session
    /// file is authorised, so this connect will skip auth and spawn the listener.
    async fn reconnect_after_auth(&mut self) -> Result<(), BridgeError> {
        // Disconnect current client.
        if let Some(old_state) = self.state.take() {
            old_state.client.disconnect();
        }

        // Re-open session (now authorised).
        let session = SqliteSession::open(&self.session_path)
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;
        let session = Arc::new(session);

        let pool = SenderPool::new(Arc::clone(&session), self.api_id);
        let updates_rx = pool.updates;
        let runner = pool.runner;
        let client = Client::new(pool.handle);

        tokio::spawn(runner.run());

        let me = client
            .get_me()
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;
        let self_user_id = me.id().bare_id();

        Self::spawn_listener(
            client.clone(),
            updates_rx,
            self.monitored_channels.clone(),
            self.passthrough.clone(),
            self_user_id,
            self.message_tx.clone(),
        );

        self.state = Some(ConnectedState {
            client,
            session,
            self_user_id,
        });

        Ok(())
    }

    /// Resolve a chat ID to a `PeerRef` using the session cache, then send a message.
    ///
    /// The chat_id here is the raw bare ID (positive), not the Bot API dialog ID.
    async fn resolve_peer_and_send(
        client: &Client,
        session: &SqliteSession,
        channel_id: i64,
        text: &str,
    ) -> Result<(), BridgeError> {
        // Telegram channel IDs received from updates are bare positive IDs.
        // We try channel first, then chat.
        let peer_ref = {
            // Try as a channel (supergroup/broadcast).
            let peer_id = PeerId::channel(channel_id)
                .or_else(|| PeerId::user(channel_id))
                .or_else(|| PeerId::chat(channel_id))
                .ok_or_else(|| {
                    BridgeError::SendFailed(format!("Invalid channel_id: {channel_id}"))
                })?;

            let auth = session
                .peer(peer_id)
                .await
                .and_then(|info| info.auth())
                .unwrap_or_default();

            PeerRef { id: peer_id, auth }
        };

        client
            .send_message(peer_ref, text)
            .await
            .map_err(|e| BridgeError::SendFailed(e.to_string()))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Bridge for TelegramBridge {
    /// Connect and authenticate. If a valid session exists, no auth is needed
    /// and the listener is spawned immediately. If auth is required, this returns
    /// an error — use the UI-driven `connect_unauthenticated` / auth methods instead.
    async fn connect(&mut self) -> Result<(), BridgeError> {
        let is_authorized = self.connect_unauthenticated().await?;

        if !is_authorized {
            return Err(BridgeError::ConnectionFailed(
                "Telegram auth required — use UI setup flow".to_string(),
            ));
        }

        self.reconnect_after_auth().await
    }

    async fn disconnect(&mut self) -> Result<(), BridgeError> {
        if let Some(state) = self.state.take() {
            state.client.disconnect();
        }
        Ok(())
    }

    async fn send(&self, channel_id: i64, envelope: &str) -> Result<(), BridgeError> {
        let state = self.state.as_ref().ok_or(BridgeError::NotConnected)?;
        Self::resolve_peer_and_send(&state.client, &state.session, channel_id, envelope)
            .await
    }

    async fn create_channel(&self, user_ids: &[i64], name: &str) -> Result<i64, BridgeError> {
        let state = self.state.as_ref().ok_or(BridgeError::NotConnected)?;
        let client = &state.client;

        // Build InputUser list from the session-cached peer info.
        let mut input_users: Vec<tl::enums::InputUser> = Vec::with_capacity(user_ids.len());
        for &uid in user_ids {
            let peer_id = PeerId::user(uid).ok_or_else(|| {
                BridgeError::ChannelCreationFailed(format!("Invalid user_id: {uid}"))
            })?;
            let auth = state
                .session
                .peer(peer_id)
                .await
                .and_then(|info| info.auth())
                .unwrap_or(PeerAuth::default());

            input_users.push(
                tl::types::InputUser {
                    user_id: uid,
                    access_hash: auth.hash(),
                }
                .into(),
            );
        }

        let result = client
            .invoke(&tl::functions::messages::CreateChat {
                users: input_users,
                title: name.to_string(),
                ttl_period: None,
            })
            .await
            .map_err(|e| BridgeError::ChannelCreationFailed(e.to_string()))?;

        // Extract the chat ID from the Updates response.
        let chat_id = extract_chat_id_from_updates(result)?;
        Ok(chat_id)
    }

    async fn get_self_user_id(&self) -> Result<i64, BridgeError> {
        self.state
            .as_ref()
            .map(|s| s.self_user_id)
            .ok_or(BridgeError::NotConnected)
    }
}

/// Extract the new chat ID from the `messages.InvitedUsers` response returned by `messages.createChat`.
fn extract_chat_id_from_updates(
    invited: tl::enums::messages::InvitedUsers,
) -> Result<i64, BridgeError> {
    let tl::enums::messages::InvitedUsers::Users(users) = invited;
    match users.updates {
        tl::enums::Updates::Updates(tl::types::Updates { chats, .. }) => {
            let chat = chats.into_iter().next().ok_or_else(|| {
                BridgeError::ChannelCreationFailed("no chat in createChat response".to_string())
            })?;
            Ok(chat_id_from_enum(chat)?)
        }
        tl::enums::Updates::UpdateShort(_)
        | tl::enums::Updates::UpdateShortMessage(_)
        | tl::enums::Updates::UpdateShortChatMessage(_)
        | tl::enums::Updates::UpdateShortSentMessage(_)
        | tl::enums::Updates::Combined(_) => Err(BridgeError::ChannelCreationFailed(
            "unexpected updates format from createChat".to_string(),
        )),
        tl::enums::Updates::TooLong => Err(BridgeError::ChannelCreationFailed(
            "updates too long from createChat".to_string(),
        )),
    }
}

fn chat_id_from_enum(chat: tl::enums::Chat) -> Result<i64, BridgeError> {
    match chat {
        tl::enums::Chat::Chat(c) => Ok(c.id),
        tl::enums::Chat::Channel(c) => Ok(c.id),
        tl::enums::Chat::Forbidden(c) => Ok(c.id),
        tl::enums::Chat::ChannelForbidden(c) => Ok(c.id),
        tl::enums::Chat::Empty(c) => Ok(c.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_bridge() -> (TelegramBridge, mpsc::UnboundedReceiver<IncomingMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let bridge = TelegramBridge::new(
            12345,
            "test_hash".to_string(),
            PathBuf::from("/tmp/test.session"),
            HashSet::new(),
            tx,
        );
        (bridge, rx)
    }

    #[tokio::test]
    async fn test_passthrough_default_false() {
        let (bridge, _rx) = make_bridge();
        let passthrough = *bridge.passthrough.lock().await;
        assert!(!passthrough, "passthrough should default to false");
    }

    #[tokio::test]
    async fn test_monitored_channel_add() {
        let (bridge, _rx) = make_bridge();
        bridge.add_monitored_channel(42).await;
        let channels = bridge.monitored_channels.lock().await;
        assert!(channels.contains(&42));
    }

    #[tokio::test]
    async fn test_monitored_channel_remove() {
        let (bridge, _rx) = make_bridge();
        bridge.add_monitored_channel(42).await;
        bridge.remove_monitored_channel(42).await;
        let channels = bridge.monitored_channels.lock().await;
        assert!(!channels.contains(&42));
    }

    #[tokio::test]
    async fn test_initial_monitored_channels() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let initial: HashSet<i64> = [1, 2, 3].iter().cloned().collect();
        let bridge = TelegramBridge::new(
            12345,
            "test_hash".to_string(),
            PathBuf::from("/tmp/test.session"),
            initial.clone(),
            tx,
        );
        let channels = bridge.monitored_channels.lock().await;
        assert_eq!(*channels, initial);
    }

    #[test]
    fn test_incoming_message_struct() {
        let msg = IncomingMessage {
            channel_id: 100,
            text: "hello".to_string(),
            sender_id: 999,
        };
        assert_eq!(msg.channel_id, 100);
        assert_eq!(msg.text, "hello");
        assert_eq!(msg.sender_id, 999);
    }

    #[tokio::test]
    async fn test_get_self_user_id_not_connected() {
        let (bridge, _rx) = make_bridge();
        let result = bridge.get_self_user_id().await;
        assert!(matches!(result, Err(BridgeError::NotConnected)));
    }

    #[tokio::test]
    async fn test_send_not_connected() {
        let (bridge, _rx) = make_bridge();
        let result = bridge.send(123, "hello").await;
        assert!(matches!(result, Err(BridgeError::NotConnected)));
    }

    #[tokio::test]
    async fn test_create_channel_not_connected() {
        let (bridge, _rx) = make_bridge();
        let result = bridge.create_channel(&[1, 2], "test").await;
        assert!(matches!(result, Err(BridgeError::NotConnected)));
    }
}
