use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::traits::AppState;

#[derive(Error, Debug)]
pub enum ConnectorError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("Connection timeout")]
    Timeout,
    #[error(
        "Could not connect to the server after multiple attempts: {attempts} attempts on platform {platform}"
    )]
    MultipleAttemptsConnection { attempts: usize, platform: String },
    #[error("Connection is closed")]
    ConnectionClosed,
    #[error("Custom: {0}")]
    Custom(String),
    #[error("Tls error: {0}")]
    Tls(String),
    #[error("Url parsing error, {0} is not a valid url")]
    UrlParsing(String),
    #[error("Failed to build http request: {0}")]
    HttpRequestBuild(String),
    #[error("Core error: {0}")]
    Core(String),
}

pub type ConnectorResult<T> = std::result::Result<T, ConnectorError>;
pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[async_trait]
pub trait Connector<S: AppState>: Send + Sync {
    /// Connect to the WebSocket server and return the stream
    async fn connect(&self, state: Arc<S>) -> ConnectorResult<WsStream>;

    /// Disconnect from the WebSocket server
    async fn disconnect(&self) -> ConnectorResult<()>;

    /// Reconnect to the WebSocket server with automatic retry logic and return the stream
    async fn reconnect(&self, state: Arc<S>) -> ConnectorResult<WsStream> {
        self.disconnect().await?;

        // Retry logic can be implemented here if needed
        self.connect(state).await
    }
}
