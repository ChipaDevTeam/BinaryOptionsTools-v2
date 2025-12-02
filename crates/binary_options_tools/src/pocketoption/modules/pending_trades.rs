use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use serde::Deserialize;
use tokio::{select, time::timeout};
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
    types::{FailOpenOrder, MultiPatternRule, OpenPendingOrder, PendingOrder},
};

#[derive(Debug)]
pub enum Command {
    OpenPendingOrder {
        open_type: u32,
        amount: f64,
        asset: String,
        open_time: u32,
        open_price: f64,
        timeframe: u32,
        min_payout: u32,
        command: u32,
        req_id: Uuid,
    },
}

#[derive(Debug)]
pub enum CommandResponse {
    Success {
        req_id: Uuid,
        pending_order: Box<PendingOrder>,
    },
    Error(Box<FailOpenOrder>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ServerResponse {
    Success(Box<PendingOrder>),
    Fail(Box<FailOpenOrder>),
}

pub struct PendingTradesHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
}

impl Clone for PendingTradesHandle {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

impl PendingTradesHandle {
    pub async fn open_pending_order(
        &self,
        open_type: u32,
        amount: f64,
        asset: String,
        open_time: u32,
        open_price: f64,
        timeframe: u32,
        min_payout: u32,
        command: u32,
    ) -> PocketResult<PendingOrder> {
        let id = Uuid::new_v4();
        self.sender
            .send(Command::OpenPendingOrder {
                open_type,
                amount,
                asset: asset.clone(),
                open_time,
                open_price,
                timeframe,
                min_payout,
                command,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        loop {
            match timeout(Duration::from_secs(10), self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Success {
                    req_id,
                    pending_order,
                })) => {
                    if req_id == id {
                        return Ok(*pending_order);
                    } else {
                        warn!("Received response for unknown req_id: {}", req_id);
                        continue;
                    }
                }
                Ok(Ok(CommandResponse::Error(fail))) => {
                    return Err(PocketError::FailOpenOrder {
                        error: fail.error,
                        amount: fail.amount,
                        asset: fail.asset,
                    });
                }
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: "open_pending_order".to_string(),
                        context: format!("asset: {}, open_type: {}", asset, open_type),
                        duration: Duration::from_secs(10),
                    })
                }
            }
        }
    }
}

pub struct PendingTradesApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    last_req_id: Option<Uuid>,
}

#[async_trait]
impl ApiModule<State> for PendingTradesApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = PendingTradesHandle;

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
        PendingTradesHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::OpenPendingOrder { open_type, amount, asset, open_time, open_price, timeframe, min_payout, command, req_id } => {
                            if self.last_req_id.is_some() {
                                warn!(target: "PendingTradesApiModule", "Overwriting a pending request. Concurrent open_pending_order calls are not supported.");
                            }
                            self.last_req_id = Some(req_id);
                            let order = OpenPendingOrder::new(open_type, amount, asset, open_time, open_price, timeframe, min_payout, command);
                            self.to_ws_sender.send(Message::text(order.to_string())).await?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    if let Message::Binary(data) = &*msg {
                        if let Ok(response) = serde_json::from_slice::<ServerResponse>(data) {
                            match response {
                                ServerResponse::Success(pending_order) => {
                                    self.state.trade_state.add_pending_deal(*pending_order.clone()).await;
                                    info!(target: "PendingTradesApiModule", "Pending trade opened: {}", pending_order.ticket);
                                    let response_req_id = self.last_req_id.take().unwrap_or_else(|| {
                                        warn!(target: "PendingTradesApiModule", "Received successopenPendingOrder but no req_id was pending. Using ticket as fallback req_id.");
                                        pending_order.ticket
                                    });
                                    self.command_responder.send(CommandResponse::Success {
                                        req_id: response_req_id,
                                        pending_order,
                                    }).await?;
                                }
                                ServerResponse::Fail(fail) => {
                                    self.last_req_id = None;
                                    self.command_responder.send(CommandResponse::Error(fail)).await?;
                                }
                            }
                        } else {
                            let data_as_string = String::from_utf8_lossy(data);
                            warn!(
                                target: "PendingTradesApiModule",
                                "Failed to deserialize message. Data: {}", data_as_string
                            );
                        }
                    }
                }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec![
            "successopenPendingOrder",
            "failopenPendingOrder",
        ]))
    }
}
