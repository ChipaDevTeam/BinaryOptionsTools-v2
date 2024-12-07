use std::string::FromUtf8Error;

use thiserror::Error;
use tokio_tungstenite::tungstenite::{http, Message};

#[derive(Error, Debug)]
pub enum PocketOptionError {
    #[error("Failed to parse SSID: {0}")]
    SsidParsingError(String),
    #[error("Failed to parse data: {0}")]
    GeneralParsingError(String),
    #[error("Error making http request: {0}")]
    HTTPError(#[from] http::Error),
    #[error("TLS Certificate error, {0}")]
    TLSError(#[from] native_tls::Error),
    #[error("Failed to connect to websocket server: {0}")]
    WebsocketConnectionError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Failed to parse recieved data to Message: {0}")]
    WebSocketMessageParsingError(#[from] serde_json::Error),
    #[error("Failed to process recieved Message: {0}")]
    WebSocketMessageProcessingError(#[from] anyhow::Error),
    #[error("Failed to convert bytes to string, {0}")]
    WebSocketMessageByteSerializationError(#[from] FromUtf8Error),
    #[error("Failed to send message to websocket sender, {0}")]
    MessageSendingError(#[from] tokio::sync::mpsc::error::SendError<Message>),
    #[error("Failed to make request, {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("If you are having this error please contact the developpers, {0}")]
    UnreachableError(String),
    #[error("Unallowed operation, {0}")]
    Unallowed(String)
}

pub type PocketResult<T> = Result<T, PocketOptionError>;