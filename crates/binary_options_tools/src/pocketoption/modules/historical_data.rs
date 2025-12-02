use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use serde::Deserialize;
use tokio::{select, time::timeout};
use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::{
    candle::Candle,
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
};

#[derive(Debug)]
pub enum Command {
    GetHistory {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
}

#[derive(Debug)]
pub enum CommandResponse {
    Success { req_id: Uuid, candles: Vec<Candle> },
    Error(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ServerResponse {
    Success(Vec<Candle>),
    Fail(String),
}

pub struct HistoricalDataHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
}

impl Clone for HistoricalDataHandle {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

impl HistoricalDataHandle {
    pub async fn get_history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let id = Uuid::new_v4();
        self.sender
            .send(Command::GetHistory {
                asset: asset.clone(),
                period,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        loop {
            match timeout(Duration::from_secs(10), self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Success { req_id, candles })) => {
                    if req_id == id {
                        return Ok(candles);
                    } else {
                        continue;
                    }
                }
                Ok(Ok(CommandResponse::Error(e))) => return Err(PocketError::General(e)),
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: "get_history".to_string(),
                        context: format!("asset: {}, period: {}", asset, period),
                        duration: Duration::from_secs(10),
                    })
                }
            }
        }
    }
}

pub struct HistoricalDataApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    last_req_id: Option<Uuid>,
}

#[async_trait]
impl ApiModule<State> for HistoricalDataApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = HistoricalDataHandle;

    fn new(
        shared_state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
    ) -> Self {
        Self {
            state: shared_state,
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
            last_req_id: None,
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        HistoricalDataHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::GetHistory { asset, period, req_id } => {
                            if self.last_req_id.is_some() {
                                warn!(target: "HistoricalDataApiModule", "Overwriting a pending request. Concurrent get_history calls are not supported.");
                            }
                            self.last_req_id = Some(req_id);
                            let msg = format!("42[\"getHistory\",{{\"asset\":\"{}\",\"period\":{}}}]", asset, period);
                            self.to_ws_sender.send(Message::text(msg)).await?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    if let Message::Binary(data) = &*msg {
                        if let Ok(response) = serde_json::from_slice::<ServerResponse>(data) {
                            match response {
                                ServerResponse::Success(candles) => {
                                    if let Some(req_id) = self.last_req_id.take() {
                                        self.command_responder.send(CommandResponse::Success {
                                            req_id,
                                            candles,
                                        }).await?;
                                    } else {
                                        warn!(target: "HistoricalDataApiModule", "Received history data but no req_id was pending. Discarding.");
                                    }
                                }
                                ServerResponse::Fail(e) => {
                                    self.last_req_id = None;
                                    self.command_responder.send(CommandResponse::Error(e)).await?;
                                }
                            }
                        } else {
                            let data_as_string = String::from_utf8_lossy(data);
                            warn!(
                                target: "HistoricalDataApiModule",
                                "Failed to deserialize message. Data: {}", data_as_string
                            );
                        }
                    }
                }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec!["updateHistory"]))
    }
}
