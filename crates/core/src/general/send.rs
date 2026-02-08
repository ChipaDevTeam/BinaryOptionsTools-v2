use std::time::Duration;

use async_channel::{bounded, Receiver, RecvError, Sender};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::{
    error::{BinaryOptionsResult, BinaryOptionsToolsError},
    general::validate::validate,
    utils::time::timeout,
};

use super::{
    stream::FilteredRecieverStream,
    traits::{DataHandler, MessageTransfer, RawMessage, ValidatorTrait},
    types::Data,
};

#[derive(Clone)]
pub struct SenderMessage {
    sender: Sender<Message>,
    sender_priority: Sender<Message>,
}

impl SenderMessage {
    pub fn new(cap: usize) -> (Self, (Receiver<Message>, Receiver<Message>)) {
        let (s, r) = bounded(cap);
        let (sp, rp) = bounded(cap);

        (
            Self {
                sender: s,
                sender_priority: sp,
            },
            (r, rp),
        )
    }
    // pub fn new(sender: Sender<Transfer>) -> Self {
    //     Self { sender }
    // }
    async fn reciever<Transfer: MessageTransfer, T: DataHandler<Transfer = Transfer>>(
        &self,
        data: &Data<T, Transfer>,
        msg: Transfer,
        response_type: Transfer::Info,
    ) -> BinaryOptionsResult<Receiver<Transfer>> {
        let reciever = data.add_request(response_type).await;

        self.send(msg)
            .await
            .map_err(|e| BinaryOptionsToolsError::GeneralMessageSendingError(e.to_string()))?;
        Ok(reciever)
    }

    async fn raw_reciever<Transfer: MessageTransfer, T: DataHandler<Transfer = Transfer>>(
        &self,
        data: &Data<T, Transfer>,
        msg: Transfer::Raw,
    ) -> BinaryOptionsResult<Receiver<Transfer::Raw>> {
        let reciever = data.raw_reciever();

        self.raw_send::<Transfer>(msg)
            .await
            .map_err(|e| BinaryOptionsToolsError::GeneralMessageSendingError(e.to_string()))?;

        Ok(reciever)
    }

    pub async fn raw_send<Transfer: MessageTransfer>(
        &self,
        msg: Transfer::Raw,
    ) -> BinaryOptionsResult<()> {
        self.sender
            .send(msg.message())
            .await
            .map_err(|e| BinaryOptionsToolsError::ChannelRequestSendingError(e.to_string()))
    }

    pub async fn send<Transfer: MessageTransfer>(&self, msg: Transfer) -> BinaryOptionsResult<()> {
        self.sender
            .send(msg.into())
            .await
            .map_err(|e| BinaryOptionsToolsError::ChannelRequestSendingError(e.to_string()))
    }

    pub async fn priority_send(&self, msg: Message) -> BinaryOptionsResult<()> {
        self.sender_priority
            .send(msg)
            .await
            .map_err(|e| BinaryOptionsToolsError::ChannelRequestSendingError(e.to_string()))?;
        Ok(())
    }

    pub async fn send_message<Transfer: MessageTransfer, T: DataHandler<Transfer = Transfer>>(
        &self,
        data: &Data<T, Transfer>,
        msg: Transfer,
        response_type: Transfer::Info,
        validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    ) -> BinaryOptionsResult<Transfer> {
        let reciever = self.reciever(data, msg, response_type).await?;

        while let Ok(msg) = reciever.recv().await {
            if let Some(msg) =
                validate(validator, msg).inspect_err(|e| warn!("Failed to place trade {e}"))?
            {
                return Ok(msg);
            }
        }
        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
            RecvError,
        ))
    }

    pub async fn send_raw_message<
        Transfer: MessageTransfer,
        T: DataHandler<Transfer = Transfer>,
    >(
        &self,
        data: &Data<T, Transfer>,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<Transfer::Raw> {
        let reciever = self.raw_reciever(data, msg).await?;

        while let Ok(msg) = reciever.recv().await {
            if validator.validate(&msg) {
                return Ok(msg);
            }
        }
        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
            RecvError,
        ))
    }

    pub async fn send_message_with_timout<
        Transfer: MessageTransfer,
        T: DataHandler<Transfer = Transfer>,
    >(
        &self,
        time: Duration,
        task: impl ToString,
        data: &Data<T, Transfer>,
        msg: Transfer,
        response_type: Transfer::Info,
        validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    ) -> BinaryOptionsResult<Transfer> {
        let reciever = self.reciever(data, msg, response_type).await?;

        timeout(
            time,
            async {
                while let Ok(msg) = reciever.recv().await {
                    if let Some(msg) = validate(validator, msg)
                        .inspect_err(|e| warn!("Failed to place trade {e}"))?
                    {
                        return Ok(msg);
                    }
                }
                Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                    RecvError,
                ))
            },
            task.to_string(),
        )
        .await
    }
    pub async fn send_raw_message_with_timout<
        Transfer: MessageTransfer,
        T: DataHandler<Transfer = Transfer>,
    >(
        &self,
        time: Duration,
        task: impl ToString,
        data: &Data<T, Transfer>,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<Transfer::Raw> {
        let reciever = self.raw_reciever(data, msg).await?;

        timeout(
            time,
            async {
                while let Ok(msg) = reciever.recv().await {
                    if validator.validate(&msg) {
                        return Ok(msg);
                    }
                }
                Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                    RecvError,
                ))
            },
            task.to_string(),
        )
        .await
    }

    pub async fn send_message_with_timeout_and_retry<
        Transfer: MessageTransfer,
        T: DataHandler<Transfer = Transfer>,
    >(
        &self,
        time: Duration,
        task: impl ToString,
        data: &Data<T, Transfer>,
        msg: Transfer,
        response_type: Transfer::Info,
        validator: &(dyn ValidatorTrait<Transfer> + Send + Sync),
    ) -> BinaryOptionsResult<Transfer> {
        let reciever = self
            .reciever(data, msg.clone(), response_type.clone())
            .await?;

        let call1 = timeout(
            time,
            async {
                while let Ok(msg) = reciever.recv().await {
                    if let Some(msg) = validate(validator, msg)
                        .inspect_err(|e| warn!("Failed to place trade {e}"))?
                    {
                        return Ok(msg);
                    }
                }
                Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                    RecvError,
                ))
            },
            task.to_string(),
        )
        .await;
        match call1 {
            Ok(res) => Ok(res),
            Err(_) => {
                info!("Failded once trying again");
                let reciever = self.reciever(data, msg, response_type).await?;
                timeout(
                    time,
                    async {
                        while let Ok(msg) = reciever.recv().await {
                            if let Some(msg) = validate(validator, msg)
                                .inspect_err(|e| warn!("Failed to place trade {e}"))?
                            {
                                return Ok(msg);
                            }
                        }
                        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                            RecvError,
                        ))
                    },
                    task.to_string(),
                )
                .await
            }
        }
    }

    pub async fn send_raw_message_with_timeout_and_retry<
        Transfer: MessageTransfer,
        T: DataHandler<Transfer = Transfer>,
    >(
        &self,
        time: Duration,
        task: impl ToString,
        data: &Data<T, Transfer>,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<Transfer::Raw> {
        let reciever = self.raw_reciever(data, msg.clone()).await?;

        let call1 = timeout(
            time,
            async {
                while let Ok(msg) = reciever.recv().await {
                    if validator.validate(&msg) {
                        return Ok(msg);
                    }
                }
                Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                    RecvError,
                ))
            },
            task.to_string(),
        )
        .await;
        match call1 {
            Ok(res) => Ok(res),
            Err(_) => {
                info!("Failded once trying again");
                let reciever = self.raw_reciever(data, msg).await?;
                timeout(
                    time,
                    async {
                        while let Ok(msg) = reciever.recv().await {
                            if validator.validate(&msg) {
                                return Ok(msg);
                            }
                        }
                        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(
                            RecvError,
                        ))
                    },
                    task.to_string(),
                )
                .await
            }
        }
    }

    pub async fn send_raw_message_iterator<
        Transfer: MessageTransfer,
        T: DataHandler<Transfer = Transfer>,
    >(
        &self,
        timeout: Option<Duration>,
        data: &Data<T, Transfer>,
        msg: Transfer::Raw,
        validator: Box<dyn ValidatorTrait<Transfer::Raw> + Send + Sync>,
    ) -> BinaryOptionsResult<FilteredRecieverStream<Transfer::Raw>> {
        let reciever = self.raw_reciever(data, msg).await?;
        info!("Created new RawStreamIterator");
        Ok(FilteredRecieverStream::new(reciever, timeout, validator))
    }
}
