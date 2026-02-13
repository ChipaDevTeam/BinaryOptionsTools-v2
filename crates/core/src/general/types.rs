use std::{collections::HashMap, ops::Deref, sync::Arc};

use async_channel::bounded;
use async_channel::Receiver;
use async_channel::Sender;
use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::constants::MAX_CHANNEL_CAPACITY;
use crate::error::BinaryOptionsResult;
use crate::error::BinaryOptionsToolsError;

use super::config;
use super::send::SenderMessage;
use super::traits::InnerConfig;
use super::traits::WCallback;
use super::traits::{DataHandler, MessageTransfer};

#[derive(Clone)]
pub enum MessageType<Transfer>
where
    Transfer: MessageTransfer,
{
    Info(Transfer::Info),
    Transfer(Transfer),
    Raw(Transfer::Raw),
}

// Type alias to reduce type complexity for pending_requests
type PendingRequests<Transfer> = Arc<
    Mutex<HashMap<<Transfer as MessageTransfer>::Info, (Sender<Transfer>, Receiver<Transfer>)>>,
>;

#[derive(Clone)]
pub struct Data<T, Transfer>
where
    Transfer: MessageTransfer,
    T: DataHandler,
{
    inner: Arc<T>,
    pub pending_requests: PendingRequests<Transfer>,
    pub raw_requests: (Sender<Transfer::Raw>, Receiver<Transfer::Raw>),
}

impl<T: DataHandler + Default, Transfer: MessageTransfer> Default for Data<T, Transfer> {
    fn default() -> Self {
        let raw_requests = bounded(MAX_CHANNEL_CAPACITY);
        Self {
            raw_requests,
            inner: Default::default(),
            pending_requests: Default::default(),
        }
    }
}
#[derive(Clone)]
pub struct Callback<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig> {
    inner: Arc<dyn WCallback<T = T, Transfer = Transfer, U = U>>,
}

pub fn default_validator<Transfer: MessageTransfer>(_val: &Transfer) -> bool {
    false
}

impl<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig> Callback<T, Transfer, U> {
    pub fn new(callback: Arc<dyn WCallback<T = T, Transfer = Transfer, U = U>>) -> Self {
        Self { inner: callback }
    }
}

#[async_trait]
impl<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig> WCallback
    for Callback<T, Transfer, U>
{
    type T = T;
    type Transfer = Transfer;
    type U = U;

    async fn call(
        &self,
        data: Data<Self::T, Self::Transfer>,
        sender: &SenderMessage,
        config: &config::Config<Self::T, Self::Transfer, Self::U>,
    ) -> BinaryOptionsResult<()> {
        self.inner.call(data, sender, config).await
    }
}

impl<T, Transfer> Data<T, Transfer>
where
    Transfer: MessageTransfer,
    T: DataHandler<Transfer = Transfer>,
{
    pub fn new(inner: T) -> Self {
        let raw_requests = bounded(MAX_CHANNEL_CAPACITY);
        Self {
            inner: Arc::new(inner),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            raw_requests,
        }
    }

    pub fn raw_reciever(&self) -> Receiver<Transfer::Raw> {
        self.raw_requests.1.clone()
    }

    pub fn raw_sender(&self) -> Sender<Transfer::Raw> {
        self.raw_requests.0.clone()
    }

    pub async fn add_request(&self, info: Transfer::Info) -> Receiver<Transfer> {
        let mut requests = self.pending_requests.lock().await;
        let (_, r) = requests
            .entry(info)
            .or_insert(bounded(MAX_CHANNEL_CAPACITY));
        r.clone()
    }

    pub async fn sender(&self, info: Transfer::Info) -> Option<Sender<Transfer>> {
        let requests = self.pending_requests.lock().await;
        requests.get(&info).map(|(s, _)| s.clone())
    }

    pub async fn get_sender(&self, message: &Transfer) -> Option<Vec<Sender<Transfer>>> {
        let requests = self.pending_requests.lock().await;
        if let Some(infos) = &message.error_info() {
            return Some(
                infos
                    .iter()
                    .filter_map(|i| requests.get(i).map(|(s, _)| s.to_owned()))
                    .collect(),
            );
        }
        requests
            .get(&message.info())
            .map(|(s, _)| vec![s.to_owned()])
    }

    pub async fn raw_send(&self, msg: Transfer::Raw) -> BinaryOptionsResult<()> {
        let sender = &self.raw_requests.0;
        if sender.receiver_count() > 1 {
            sender
                .send(msg)
                .await
                .map_err(|e| BinaryOptionsToolsError::ChannelRequestSendingError(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn update_data(
        &self,
        message: Transfer,
    ) -> BinaryOptionsResult<Option<Vec<Sender<Transfer>>>> {
        self.inner.update(&message).await?;
        Ok(self.get_sender(&message).await)
    }
}

impl<T, Transfer> Deref for Data<T, Transfer>
where
    Transfer: MessageTransfer,
    T: DataHandler,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
