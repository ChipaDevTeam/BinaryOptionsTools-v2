use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use serde::Deserialize;
use tokio::select;
use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
    candle::Candle,
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
    Success {
        req_id: Uuid,
        candles: Vec<Candle>,
    },
    Error(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ServerResponse {
    Success(Vec<Candle>),
    Fail(String),
}

#[derive(Clone)]
pub struct HistoricalDataHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
}

impl HistoricalDataHandle {
    pub async fn get_history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let id = Uuid::new_v4();
        self.sender
            .send(Command::GetHistory {
                asset,
                period,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::Success { req_id, candles }) => {
                    if req_id == id {
                        return Ok(candles);
                    } else {
                        continue;
                    }
                }
                Ok(CommandResponse::Error(e)) => return Err(PocketError::General(e)),
                Err(e) => return Err(CoreError::from(e).into()),
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
                                    if let Some(candle) = candles.first() {
                                        let req_id = Uuid::new_v4(); // This is a placeholder, as the server does not return a request id for this message.
                                        self.command_responder.send(CommandResponse::Success {
                                            req_id,
                                            candles,
                                        }).await?;
                                    }
                                }
                                ServerResponse::Fail(e) => {
                                    self.command_responder.send(CommandResponse::Error(e)).await?;
                                }
                            }
                        } else {
                            warn!(target: "HistoricalDataApiModule", "Received unrecognized message: {:?}", msg);
                        }
                    }
                }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec![
            "451-[\"updateHistory\"]",
        ]))
    }
}