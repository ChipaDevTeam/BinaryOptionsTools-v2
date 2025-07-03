use async_trait::async_trait;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("Connection timeout")]
    Timeout,
    #[error("Maximum reconnection attempts reached")]
    MaxReconnectAttemptsReached,
    #[error("Connection is closed")]
    ConnectionClosed,
    #[error("Custom: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, ConnectorError>;
pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[async_trait]
pub trait Connector: Send + Sync {
    /// Connect to the WebSocket server and return the stream
    async fn connect(&self) -> Result<WsStream>;

    /// Disconnect from the WebSocket server
    async fn disconnect(&self) -> Result<()>;

    /// Reconnect to the WebSocket server with automatic retry logic and return the stream
    async fn reconnect(&self) -> Result<WsStream> {
        self.disconnect().await?;

        // Retry logic can be implemented here if needed
        self.connect().await
    }
}
