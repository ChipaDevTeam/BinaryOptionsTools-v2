use async_trait::async_trait;
use core::{error, fmt, hash};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::error::BinaryOptionsResult;

use super::{
    config::Config,
    send::SenderMessage,
    types::{Data, MessageType},
};

/// This trait makes sure that the struct passed to the `WebsocketClient` can be cloned, sended through multiple threads, and serialized and deserialized using serde
pub trait Credentials: Clone + Send + Sync + Serialize + DeserializeOwned {}

/// This trait is used to allow users to pass their own config struct to the `WebsocketClient`
pub trait InnerConfig: DeserializeOwned + Clone + Send {}

/// This trait allows users to pass their own way of storing and updating recieved data from the `websocket` connection
#[async_trait]
pub trait DataHandler: Clone + Send + Sync {
    type Transfer: MessageTransfer;

    async fn update(&self, message: &Self::Transfer) -> BinaryOptionsResult<()>;
}

/// Allows users to add a callback that will be called when the websocket connection is established after being disconnected, you will have access to the `Data` struct providing access to any required information stored during execution
#[async_trait]
pub trait WCallback: Send + Sync {
    type T: DataHandler;
    type Transfer: MessageTransfer;
    type U: InnerConfig;

    async fn call(
        &self,
        data: Data<Self::T, Self::Transfer>,
        sender: &SenderMessage,
        config: &Config<Self::T, Self::Transfer, Self::U>,
    ) -> BinaryOptionsResult<()>;
}

/// Main entry point for the `WebsocketClient` struct, this trait is used by the client to handle incoming messages, return data to user and a lot more things
pub trait MessageTransfer:
    DeserializeOwned + Clone + Into<Message> + Send + Sync + error::Error + fmt::Debug + fmt::Display
{
    type Error: Into<Self> + Clone + error::Error;
    type TransferError: error::Error;
    type Info: MessageInformation;
    type Raw: RawMessage;

    fn info(&self) -> Self::Info;

    fn error(&self) -> Option<Self::Error>;

    fn to_error(&self) -> Self::TransferError;

    fn error_info(&self) -> Option<Vec<Self::Info>>;
}

pub trait MessageInformation:
    Serialize + DeserializeOwned + Clone + Send + Sync + Eq + hash::Hash + fmt::Debug + fmt::Display
{
}

pub trait RawMessage:
    Serialize + DeserializeOwned + Clone + Send + Sync + fmt::Debug + fmt::Display
{
    fn message(&self) -> Message {
        Message::text(self.to_string())
    }
}

#[async_trait]
/// Every struct that implements MessageHandler will recieve a message and should return
pub trait MessageHandler: Clone + Send + Sync {
    type Transfer: MessageTransfer;

    async fn process_message(
        &self,
        message: &Message,
        previous: &Option<<<Self as MessageHandler>::Transfer as MessageTransfer>::Info>,
        sender: &SenderMessage,
    ) -> BinaryOptionsResult<(Option<MessageType<Self::Transfer>>, bool)>;
}

#[async_trait]
pub trait Connect: Clone + Send + Sync {
    type Creds: Credentials;
    // type Uris: Iterator<Item = String>;

    async fn connect<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig>(
        &self,
        creds: Self::Creds,
        config: &Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<WebSocketStream<MaybeTlsStream<TcpStream>>>;
}

pub trait ValidatorTrait<T> {
    fn validate(&self, message: &T) -> bool;
}

impl<F, T> ValidatorTrait<T> for F
where
    F: Fn(&T) -> bool + Send + Sync,
{
    fn validate(&self, message: &T) -> bool {
        self(message)
    }
}

impl<T> InnerConfig for T where T: DeserializeOwned + Clone + Send {}
