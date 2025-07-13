#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("Channel receiver error: {0}")]
    ChannelReceiver(#[from] kanal::ReceiveError),
    #[error("Channel sender error: {0}")]
    ChannelSender(#[from] kanal::SendError),
    #[error("Connection error: {0}")]
    Connection(#[from] super::connector::ConnectorError),
    #[error("Failed to join task: {0}")]
    JoinTask(#[from] tokio::task::JoinError),
    /// Error for when a module is not found.
    #[error("Module '{0}' not found.")]
    ModuleNotFound(String),

    #[error("Failed to parse ssid: {0}")]
    SsidParsing(String),
    #[error("HTTP request error: {0}")]
    HttpRequest(String),

    #[error("Lightweight [{0} Module] loop exited unexpectedly.")]
    LightweightModuleLoop(String),

    #[error("Api [{0} Module] loop exited unexpectedly.")]
    ApiModuleLoop(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Poison error: {0}")]
    Poison(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type CoreResult<T> = std::result::Result<T, CoreError>;
