use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule, RunnerCommand},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::{select, time::timeout};
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
    types::{FailOpenOrder, MultiPatternRule, OpenPendingOrder, PendingOrder},
    utils::SocketIoFrame,
};

const PENDING_ORDER_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MISMATCH_RETRIES: usize = 5;

#[derive(Debug)]
pub enum Command {
    OpenPendingOrder {
        open_type: u32,
        amount: Decimal,
        asset: String,
        open_time: String,
        open_price: Decimal,
        timeframe: u32,
        min_payout: u32,
        command: u32,
        req_id: Uuid,
    },
    CancelPendingOrder {
        ticket: String,
        req_id: Uuid,
    },
    CancelPendingOrders {
        tickets: Vec<String>,
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
    CancelSuccess {
        req_id: Uuid,
        ticket: String,
    },
    BatchCancelSuccess {
        req_id: Uuid,
        cancelled: Vec<String>,
    },
    CancelError {
        req_id: Uuid,
        error: String,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ServerResponse {
    Success(Box<PendingOrder>),
    Fail(Box<FailOpenOrder>),
}

/// Server response for pending order cancellation
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum CancelServerResponse {
    /// Single order cancellation success
    SingleSuccess {
        #[serde(rename = "ticket")]
        ticket: String,
    },
    /// Batch cancellation success
    BatchSuccess {
        #[serde(rename = "cancelled")]
        cancelled: Vec<String>,
    },
    /// Placeholder success response seen in some cases
    Placeholder {
        #[serde(rename = "_placeholder")]
        placeholder: bool,
        #[serde(rename = "num")]
        num: u32,
    },
    /// Cancellation failure
    Error {
        #[serde(rename = "error")]
        error: String,
    },
}

pub struct PendingTradesHandle {
    pub sender: AsyncSender<Command>,
    pub receiver: AsyncReceiver<CommandResponse>,
    /// Single-threaded bottleneck for pending trade calls.
    /// This intentional design prevents head-of-line blocking issues and ensures
    /// that concurrent requests do not interfere with the platform session state.
    /// If concurrency is required in the future, consider a semaphore instead.
    pub call_lock: Arc<tokio::sync::Mutex<()>>,
}

impl Clone for PendingTradesHandle {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            call_lock: self.call_lock.clone(),
        }
    }
}

impl PendingTradesHandle {
    /// Creates a new handle with the given channels.
    pub fn new(sender: AsyncSender<Command>, receiver: AsyncReceiver<CommandResponse>) -> Self {
        Self {
            sender,
            receiver,
            call_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Sets an external lock for request serialization.
    pub fn with_lock(mut self, lock: Arc<tokio::sync::Mutex<()>>) -> Self {
        self.call_lock = lock;
        self
    }

    /// Cancels a pending order by its ticket identifier.
    ///
    /// # Arguments
    /// * `ticket` - The unique ticket string identifying the pending order to cancel.
    ///
    /// # Returns
    /// * `Ok(String)` - The ticket of the successfully cancelled order.
    /// * `Err(PocketError)` - If cancellation fails, the order is not found, or a timeout occurs.
    ///
    /// # Errors
    /// * `PocketError::Timeout` - If the server does not respond within the pending order timeout.
    /// * `PocketError::DealNotFound` - If no pending order with the given ticket exists.
    pub async fn cancel_pending_order(&self, ticket: String) -> PocketResult<String> {
        let _lock = self.call_lock.lock().await;

        // Drain the receiver of any stale responses from previous timed-out requests
        while let Ok(msg) = self.receiver.try_recv() {
            warn!("Drained stale response from PendingTradesHandle: {:?}", msg);
        }

        let id = Uuid::new_v4();
        self.sender
            .send(Command::CancelPendingOrder {
                ticket: ticket.clone(),
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;

        match timeout(PENDING_ORDER_TIMEOUT, self.receiver.recv()).await {
            Ok(Ok(CommandResponse::CancelSuccess {
                req_id,
                ticket: cancelled_ticket,
            })) => {
                if req_id == id {
                    Ok(cancelled_ticket)
                } else {
                    Err(PocketError::Timeout {
                        task: "cancel_pending_order".to_string(),
                        context: format!("Received mismatched req_id for ticket: {}", ticket),
                        duration: PENDING_ORDER_TIMEOUT,
                    })
                }
            }
            Ok(Ok(CommandResponse::CancelError { req_id: _, error })) => {
                Err(PocketError::FailOpenOrder {
                    error,
                    amount: Decimal::ZERO,
                    asset: String::new(),
                })
            }
            Ok(Err(e)) => Err(CoreError::from(e).into()),
            Err(_) => Err(PocketError::Timeout {
                task: "cancel_pending_order".to_string(),
                context: format!("ticket: {}", ticket),
                duration: PENDING_ORDER_TIMEOUT,
            }),
            _ => Err(PocketError::Timeout {
                task: "cancel_pending_order".to_string(),
                context: format!("Unexpected response for ticket: {}", ticket),
                duration: PENDING_ORDER_TIMEOUT,
            }),
        }
    }

    /// Cancels multiple pending orders in a single batch operation.
    ///
    /// # Arguments
    /// * `tickets` - A vector of ticket strings identifying the pending orders to cancel.
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - A vector of tickets that were successfully cancelled.
    /// * `Err(PocketError)` - If the batch cancellation fails or a timeout occurs.
    ///
    /// # Note
    /// Partial success is possible: some orders may be cancelled while others fail.
    /// The returned vector contains only the successfully cancelled tickets.
    pub async fn cancel_pending_orders(&self, tickets: Vec<String>) -> PocketResult<Vec<String>> {
        let _lock = self.call_lock.lock().await;

        // Drain the receiver of any stale responses from previous timed-out requests
        while let Ok(msg) = self.receiver.try_recv() {
            warn!("Drained stale response from PendingTradesHandle: {:?}", msg);
        }

        let id = Uuid::new_v4();
        self.sender
            .send(Command::CancelPendingOrders {
                tickets: tickets.clone(),
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;

        match timeout(PENDING_ORDER_TIMEOUT, self.receiver.recv()).await {
            Ok(Ok(CommandResponse::BatchCancelSuccess { req_id, cancelled })) => {
                if req_id == id {
                    Ok(cancelled)
                } else {
                    Err(PocketError::Timeout {
                        task: "cancel_pending_orders".to_string(),
                        context: "Received mismatched req_id".to_string(),
                        duration: PENDING_ORDER_TIMEOUT,
                    })
                }
            }
            Ok(Ok(CommandResponse::CancelError { req_id: _, error })) => {
                Err(PocketError::FailOpenOrder {
                    error,
                    amount: Decimal::ZERO,
                    asset: String::new(),
                })
            }
            Ok(Err(e)) => Err(CoreError::from(e).into()),
            Err(_) => Err(PocketError::Timeout {
                task: "cancel_pending_orders".to_string(),
                context: format!("tickets: {:?}", tickets),
                duration: PENDING_ORDER_TIMEOUT,
            }),
            _ => Err(PocketError::Timeout {
                task: "cancel_pending_orders".to_string(),
                context: "Unexpected response".to_string(),
                duration: PENDING_ORDER_TIMEOUT,
            }),
        }
    }

    /// Opens a pending order on the PocketOption platform.
    ///
    /// This method is now thread-safe and will serialize requests to prevent
    /// concurrent access issues.
    #[allow(clippy::too_many_arguments)]
    pub async fn open_pending_order(
        &self,
        open_type: u32,
        amount: Decimal,
        asset: String,
        open_time: String,
        open_price: Decimal,
        timeframe: u32,
        min_payout: u32,
        command: u32,
    ) -> PocketResult<PendingOrder> {
        let _lock = self.call_lock.lock().await;

        // Drain the receiver of any stale responses from previous timed-out requests
        while let Ok(msg) = self.receiver.try_recv() {
            warn!("Drained stale response from PendingTradesHandle: {:?}", msg);
        }

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
                Ok(Ok(other)) => {
                    warn!(
                        "Received unexpected response type in open_pending_order: {:?}",
                        other
                    );
                    mismatch_count += 1;
                    if mismatch_count >= MAX_MISMATCH_RETRIES {
                        return Err(PocketError::Timeout {
                            task: "open_pending_order".to_string(),
                            context: format!(
                                "asset: {}, open_type: {}, exceeded mismatch retries (unexpected response)",
                                asset, open_type
                            ),
                            duration: PENDING_ORDER_TIMEOUT,
                        });
                    }
                    continue;
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
    pending_requests: std::collections::VecDeque<Uuid>,
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
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self {
            state: shared_state,
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
            pending_requests: std::collections::VecDeque::new(),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        PendingTradesHandle::new(sender, receiver)
    }
    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                Ok(cmd) = self.command_receiver.recv() => {
                    tracing::debug!(target: "PendingTradesApiModule", "Received command: {:?}, pending_requests before: {:?}", cmd, self.pending_requests);
                    match cmd {
                        Command::OpenPendingOrder { open_type, amount, asset, open_time, open_price, timeframe, min_payout, command, req_id } => {
                            self.pending_requests.push_back(req_id);
                            tracing::debug!(target: "PendingTradesApiModule", "Added req_id to queue: {:?}. Queue size: {}", req_id, self.pending_requests.len());
                            let order = OpenPendingOrder::new(open_type, amount, asset, open_time, open_price, timeframe, min_payout, command);
                            self.to_ws_sender.send(Message::text(order.to_string())).await?;
                        }
                        Command::CancelPendingOrder { ticket, req_id } => {
                            self.pending_requests.push_back(req_id);
                            tracing::debug!(target: "PendingTradesApiModule", "Added cancel req_id to queue: {:?}. Queue size: {}", req_id, self.pending_requests.len());
                            // Send cancel pending order message to WebSocket
                            let cancel_msg = serde_json::json!(["cancelPendingOrder", {
                                "ticket": ticket
                            }]);
                            self.to_ws_sender.send(Message::text(format!("42{}", cancel_msg))).await?;
                        }
                        Command::CancelPendingOrders { tickets, req_id } => {
                            self.pending_requests.push_back(req_id);
                            tracing::debug!(target: "PendingTradesApiModule", "Added batch cancel req_id to queue: {:?}. Queue size: {}", req_id, self.pending_requests.len());
                            // Send batch cancel pending orders message to WebSocket
                            let cancel_msg = serde_json::json!(["cancelPendingOrders", {
                                "tickets": tickets
                            }]);
                            self.to_ws_sender.send(Message::text(format!("42{}", cancel_msg))).await?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    tracing::debug!(target: "PendingTradesApiModule", "Received message: {:?}, pending_requests: {:?}", msg, self.pending_requests);
                    let response_result = match msg.as_ref() {
                        Message::Binary(data) => serde_json::from_slice::<ServerResponse>(data).map_err(|e| e.to_string()),
                        Message::Text(text) => {
                            if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                                Ok(res)
                            } else if let Some(frame) = SocketIoFrame::parse(text) {
                                if let Some((event, payload)) = frame.extract_event() {
                                    if event == "successopenPendingOrder" || event == "failopenPendingOrder" {
                                        serde_json::from_value::<ServerResponse>(payload).map_err(|e| e.to_string())
                                    } else if event == "successcancelPendingOrder" || event == "failcancelPendingOrder" || 
                                              event == "successcancelPendingOrders" || event == "failcancelPendingOrders" {
                                        // Handle cancellation responses
                                        match serde_json::from_value::<CancelServerResponse>(payload) {
                                            Ok(cancel_res) => {
                                                if let Some(req_id) = self.pending_requests.pop_front() {
                                                    let resp = match cancel_res {
                                                        CancelServerResponse::SingleSuccess { ticket } => CommandResponse::CancelSuccess { req_id, ticket },
                                                        CancelServerResponse::BatchSuccess { cancelled } => CommandResponse::BatchCancelSuccess { req_id, cancelled },
                                                        CancelServerResponse::Placeholder { .. } => CommandResponse::CancelSuccess { req_id, ticket: String::new() },
                                                        CancelServerResponse::Error { error } => CommandResponse::CancelError { req_id, error },
                                                    };
                                                    if let Err(e) = self.command_responder.send(resp).await {
                                                        warn!(target: "PendingTradesApiModule", "Failed to send cancel response: {}", e);
                                                    }
                                                }
                                                // Return a dummy Success to satisfy the outer match, but we already sent the command response
                                                Ok(ServerResponse::Success(Box::new(PendingOrder::default())))
                                            }
                                            Err(e) => Err(e.to_string()),
                                        }
                                    } else {
                                        serde_json::from_str::<ServerResponse>(text).map_err(|e| e.to_string())
                                    }
                                } else {
                                    serde_json::from_str::<ServerResponse>(text).map_err(|e| e.to_string())
                                }
                            } else {
                                serde_json::from_str::<ServerResponse>(text).map_err(|e| e.to_string())
                            }
                        },
                        _ => continue,
                    };

                    match response_result {
                        Ok(response) => {
                            match response {
                                ServerResponse::Success(pending_order) => {
                                    self.state.trade_state.add_pending_deal(*pending_order.clone()).await;
                                    info!(target: "PendingTradesApiModule", "Pending trade opened: {}", pending_order.ticket);
                                    tracing::debug!(target: "PendingTradesApiModule", "Success response, pending_requests: {:?}", self.pending_requests);
                                    if let Some(req_id) = self.pending_requests.pop_front() {
                                        tracing::debug!(target: "PendingTradesApiModule", "Sending Success response with req_id: {}", req_id);
                                        if let Err(e) = self.command_responder.send(CommandResponse::Success {
                                            req_id,
                                            pending_order,
                                        }).await {
                                            warn!(target: "PendingTradesApiModule", "Failed to send Success response: {}", e);
                                        }
                                    } else {
                                        tracing::debug!(target: "PendingTradesApiModule", "No req_id pending, dropping response");
                                        warn!(target: "PendingTradesApiModule", "Received successopenPendingOrder but no req_id was pending. Dropping response to avoid ambiguity.");
                                    }
                                }
                                ServerResponse::Fail(fail) => {
                                    if let Some(req_id) = self.pending_requests.pop_front() {
                                        tracing::debug!(target: "PendingTradesApiModule", "Forwarding failure for req_id: {}", req_id);
                                        if let Err(e) = self.command_responder.send(CommandResponse::Error(fail)).await {
                                            warn!(target: "PendingTradesApiModule", "Failed to send Error response: {}", e);
                                        }
                                    } else {
                                        tracing::debug!(target: "PendingTradesApiModule", "No req_id pending, dropping failure response");
                                        warn!(target: "PendingTradesApiModule", "Received failopenPendingOrder but no req_id was pending. Dropping response.");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                target: "PendingTradesApiModule",
                                "Failed to deserialize message. Error: {}", e
                            );
                        }
                    }
                }
                else => {
                    info!(target: "PendingTradesApiModule", "Channels closed, shutting down module.");
                    break;
                }
            }
            tracing::debug!(target: "PendingTradesApiModule", "Loop iteration completed");
        }
        Ok(())
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec![
            "successopenPendingOrder",
            "failopenPendingOrder",
            "successcancelPendingOrder",
            "failcancelPendingOrder",
            "successcancelPendingOrders",
            "failcancelPendingOrders",
        ]))
    }
}

impl Drop for PendingTradesApiModule {
    fn drop(&mut self) {
        tracing::debug!(target: "PendingTradesApiModule", "PendingTradesApiModule dropped");
    }
}
