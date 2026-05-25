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
