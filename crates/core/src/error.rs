use std::time::Duration;

use thiserror::Error;

use tokio_tungstenite::tungstenite::{http, Error as TungsteniteError, Message};

use crate::general::traits::MessageTransfer;

#[derive(Error, Debug)]
pub enum BinaryOptionsToolsError {
    #[error("Failed to parse recieved data: {0}")]
    SerdeGeneralParsingError(#[from] serde_json::Error),
    #[error("Url parsing failed: {0}")]
    UrlParsingError(#[from] url::ParseError),
    #[error("{platform} Error, {error}")]
    BinaryOptionsTradingError { platform: String, error: String },
    #[error("Error sending request, {0}")]
    WebsocketMessageSendingError(String),
    #[error("Failed to recieve data from websocket server: {0}")]
    WebsocketRecievingConnectionError(String),
    #[error("Websocket connection was closed by the server, {0}")]
    WebsocketConnectionClosed(String),
    #[error("Failed to connect to websocket server: {0}")]
    WebsocketConnectionError(Box<TungsteniteError>),
    #[error("Failed to send message to websocket sender, {0}")]
    MessageSendingError(#[from] async_channel::SendError<Message>),
    #[error("Failed to send message using asynchronous channel, {0}")]
    GeneralMessageSendingError(String),
    #[error(
        "Failed to reconnect '{0}' times, maximum allowed number of reconnections was reached, breaking"
    )]
    MaxReconnectAttemptsReached(u32),
    #[error(
        "Failed to reconnect '{number}' times, maximum allowed number of reconnections is `{max}`"
    )]
    ReconnectionAttemptFailure { number: u32, max: u32 },
    #[error("Failed to recieve message from separate thread, {0}")]
    OneShotRecieverError(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("Failed to recieve message from request channel, {0}")]
    ChannelRequestRecievingError(#[from] async_channel::RecvError),
    #[error("Failed to send message to request channel, {0}")]
    ChannelRequestSendingError(String),
    #[error("Error recieving response from server, {0}")]
    WebSocketMessageError(String),
    #[error("Failed to parse data: {0}")]
    GeneralParsingError(String),
    #[error("Error making http request: {0}")]
    HTTPError(#[from] http::Error),
    #[error("Unallowed operation, {0}")]
    Unallowed(String),
    #[error("Failed to join thread, {0}")]
    TaskJoinError(#[from] tokio::task::JoinError),
    #[error("Failed to execute '{task}' task before the maximum allowed time of '{duration:?}'")]
    TimeoutError { task: String, duration: Duration },
    #[error("Failed to parse duration, error {0}")]
    ChronoDurationParsingError(#[from] chrono::OutOfRangeError),
    #[error("Unknown error during execution, error {0}")]
    UnknownError(#[from] anyhow::Error),
}

pub type BinaryOptionsResult<T> = Result<T, BinaryOptionsToolsError>;

impl From<TungsteniteError> for BinaryOptionsToolsError {
    fn from(e: TungsteniteError) -> Self {
        Self::WebsocketConnectionError(Box::new(e))
    }
}

impl<Transfer> From<Transfer> for BinaryOptionsToolsError
where
    Transfer: MessageTransfer,
{
    fn from(value: Transfer) -> Self {
        let error = value.to_error();
        Self::WebsocketMessageSendingError(error.to_string())
    }
}
