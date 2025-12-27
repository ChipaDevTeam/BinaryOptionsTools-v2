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

const PENDING_ORDER_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MISMATCH_RETRIES: usize = 5;

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
    /// Opens a pending order on the PocketOption platform.
    ///
    /// **Warning:** Concurrent calls to this method on the same `PendingTradesHandle`
    /// are *not supported*. Executing multiple `open_pending_order` calls concurrently
    /// will lead to warnings (due to overwriting the module's internal `last_req_id`)
    /// and a high likelihood of one or more calls timing out due to response mismatches.
    /// It is critical to ensure that calls to this method are serialized or awaited
    /// sequentially by the caller. Unmatched or late responses are dropped by the module
    /// to prevent ambiguous correlation, which will result in a timeout for the caller.
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
        let mut mismatch_count = 0;
        loop {
            match timeout(PENDING_ORDER_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Success {
                    req_id,
                    pending_order,
                })) => {
                    if req_id == id {
                        return Ok(*pending_order);
                    } else {
                        warn!("Received response for unknown req_id: {}", req_id);
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: "open_pending_order".to_string(),
                                context: format!(
                                    "asset: {}, open_type: {}, exceeded mismatch retries",
                                    asset, open_type
                                ),
                                duration: PENDING_ORDER_TIMEOUT,
                            });
                        }
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
                        duration: PENDING_ORDER_TIMEOUT,
                    });
                }
            }
        }
    }
}

/// This API module handles the creation of pending trade orders.
///
/// **Concurrency and Correlation Notes:**
/// - This module is designed with the assumption that at most one `OpenPendingOrder`
///   request is in-flight at any given time per client. Concurrent calls to
///   `open_pending_order` on the `PendingTradesHandle` will lead to warnings
///   and potential timeouts, as the module's internal state (`last_req_id`)
///   can only track a single pending request.
/// - The `last_req_id` is a purely client-managed correlation ID. The PocketOption
///   server protocol for pending orders does not echo this `req_id` in its responses.
/// - When a success response for a pending order arrives and no `last_req_id` is currently
///   pending, the module drops the response and logs a warning. This avoids returning
///   ambiguous or incorrect correlation IDs to the caller.
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
                                    if let Some(req_id) = self.last_req_id.take() {
                                        self.command_responder.send(CommandResponse::Success {
                                            req_id,
                                            pending_order,
                                        }).await?;
                                    } else {
                                        warn!(target: "PendingTradesApiModule", "Received successopenPendingOrder but no req_id was pending. Dropping response to avoid ambiguity.");
                                    }
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
