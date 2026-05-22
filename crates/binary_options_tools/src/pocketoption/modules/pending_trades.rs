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
use tracing::warn;
use uuid::Uuid;
use tokio::sync::Mutex;

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
    /// The module has stopped and cannot fulfill the request.
    Shutdown {
        req_id: Uuid,
    },
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum CancelServerResponse {
    SingleSuccess {
        ticket: String,
    },
    BatchSuccess {
        cancelled: Vec<String>,
    },
    Placeholder {
        id: u32,
        success: bool,
    },
    Error {
        error: String,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ServerResponse {
    Success(Box<PendingOrder>),
    Fail(Box<FailOpenOrder>),
}

#[derive(Clone)]
pub struct PendingTradesHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
    call_lock: Arc<Mutex<()>>,
}

impl PendingTradesHandle {
    pub fn new(sender: AsyncSender<Command>, receiver: AsyncReceiver<CommandResponse>) -> Self {
        Self {
            sender,
            receiver,
            call_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Sets the lock used for serializing requests.
    pub fn with_lock(mut self, lock: Arc<Mutex<()>>) -> Self {
        self.call_lock = lock;
        self
    }

    /// Creates a new pending order on the PocketOption platform.
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

        // Drain the receiver of any stale responses
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
                        warn!("Received mismatched req_id in open_pending_order: expected {}, got {}", id, req_id);
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: "open_pending_order".to_string(),
                                context: format!("asset: {}, exceeded mismatch retries", asset),
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
                Ok(Ok(CommandResponse::Shutdown { .. })) => {
                    return Err(PocketError::ModuleStopped {
                        module_name: "PendingTradesApiModule".to_string(),
                        context: "PendingTradesApiModule stopped during request".to_string(),
                    });
                }
                Ok(Ok(other)) => {
                    warn!("Received unexpected response type in open_pending_order: {:?}", other);
                    mismatch_count += 1;
                    if mismatch_count >= MAX_MISMATCH_RETRIES {
                        return Err(PocketError::Timeout {
                            task: "open_pending_order".to_string(),
                            context: format!("asset: {}, exceeded mismatch retries (unexpected response)", asset),
                            duration: PENDING_ORDER_TIMEOUT,
                        });
                    }
                    continue;
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

    /// Cancels a specific pending order by its ticket ID.
    pub async fn cancel_pending_order(&self, ticket: String) -> PocketResult<String> {
        let _lock = self.call_lock.lock().await;

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
            Ok(Ok(CommandResponse::CancelSuccess { req_id, ticket: cancelled_ticket })) => {
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
            Ok(Ok(CommandResponse::Shutdown { .. })) => Err(PocketError::ModuleStopped {
                module_name: "PendingTradesApiModule".to_string(),
                context: "PendingTradesApiModule stopped during request".to_string(),
            }),
            Ok(Ok(other)) => {
                warn!("Received unexpected response in cancel_pending_order: {:?}", other);
                Err(PocketError::Timeout {
                    task: "cancel_pending_order".to_string(),
                    context: format!("Unexpected response type for ticket: {}", ticket),
                    duration: PENDING_ORDER_TIMEOUT,
                })
            }
            Ok(Err(e)) => Err(CoreError::from(e).into()),
            Err(_) => Err(PocketError::Timeout {
                task: "cancel_pending_order".to_string(),
                context: format!("ticket: {}", ticket),
                duration: PENDING_ORDER_TIMEOUT,
            }),
        }
    }

    /// Cancels multiple pending orders in a single batch operation.
    pub async fn cancel_pending_orders(&self, tickets: Vec<String>) -> PocketResult<Vec<String>> {
        let _lock = self.call_lock.lock().await;

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
            Ok(Ok(CommandResponse::Shutdown { .. })) => Err(PocketError::ModuleStopped {
                module_name: "PendingTradesApiModule".to_string(),
                context: "PendingTradesApiModule stopped during request".to_string(),
            }),
            Ok(Ok(other)) => {
                warn!("Received unexpected response in cancel_pending_orders: {:?}", other);
                Err(PocketError::Timeout {
                    task: "cancel_pending_orders".to_string(),
                    context: "Unexpected response type".to_string(),
                    duration: PENDING_ORDER_TIMEOUT,
                })
            }
            Ok(Err(e)) => Err(CoreError::from(e).into()),
            Err(_) => Err(PocketError::Timeout {
                task: "cancel_pending_orders".to_string(),
                context: format!("tickets: {:?}", tickets),
                duration: PENDING_ORDER_TIMEOUT,
            }),
        }
    }
}

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
                biased;
                cmd_res = self.command_receiver.recv() => {
                    match cmd_res {
                        Ok(cmd) => {
                            match cmd {
                                Command::OpenPendingOrder { open_type, amount, asset, open_time, open_price, timeframe, min_payout, command, req_id } => {
                                    self.pending_requests.push_back(req_id);
                                    let order = OpenPendingOrder::new(open_type, amount, asset, open_time, open_price, timeframe, min_payout, command);
                                    if let Err(e) = self.to_ws_sender.send(Message::text(order.to_string())).await {
                                        warn!(target: "PendingTradesApiModule", "Failed to send order to WS: {}", e);
                                        self.notify_waiters_module_stopped().await;
                                        return Err(e.into());
                                    }
                                }
                                Command::CancelPendingOrder { ticket, req_id } => {
                                    self.pending_requests.push_back(req_id);
                                    let cancel_msg = serde_json::json!(["cancelPendingOrder", { "ticket": ticket }]);
                                    if let Err(e) = self.to_ws_sender.send(Message::text(format!("42{}", cancel_msg))).await {
                                        warn!(target: "PendingTradesApiModule", "Failed to send cancel order to WS: {}", e);
                                        self.notify_waiters_module_stopped().await;
                                        return Err(e.into());
                                    }
                                }
                                Command::CancelPendingOrders { tickets, req_id } => {
                                    self.pending_requests.push_back(req_id);
                                    let cancel_msg = serde_json::json!(["cancelPendingOrders", { "tickets": tickets }]);
                                    if let Err(e) = self.to_ws_sender.send(Message::text(format!("42{}", cancel_msg))).await {
                                        warn!(target: "PendingTradesApiModule", "Failed to send batch cancel to WS: {}", e);
                                        self.notify_waiters_module_stopped().await;
                                        return Err(e.into());
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            break;
                        }
                    }
                },
                msg_res = self.message_receiver.recv() => {
                    match msg_res {
                        Ok(msg) => {
                            match msg.as_ref() {
                                Message::Text(text) => {
                                    if let Some(frame) = SocketIoFrame::parse(text) {
                                        if let Some((event, payload)) = frame.extract_event() {
                                            match event.as_str() {
                                                "successopenPendingOrder" | "failopenPendingOrder" => {
                                                    if let Ok(response) = serde_json::from_value::<ServerResponse>(payload) {
                                                        if let ServerResponse::Success(ref pending_order) = response {
                                                            self.state.trade_state.add_pending_deal(*pending_order.clone()).await;
                                                        }
                                                        if let Some(req_id) = self.pending_requests.pop_front() {
                                                            match response {
                                                                ServerResponse::Success(pending_order) => {
                                                                    let _ = self.command_responder.send(CommandResponse::Success { req_id, pending_order }).await;
                                                                }
                                                                ServerResponse::Fail(fail) => {
                                                                    let _ = self.command_responder.send(CommandResponse::Error(fail)).await;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    continue;
                                                }
                                                "successcancelPendingOrder" | "failcancelPendingOrder" | 
                                                "successcancelPendingOrders" | "failcancelPendingOrders" => {
                                                    if let Ok(cancel_res) = serde_json::from_value::<CancelServerResponse>(payload) {
                                                        if let Some(req_id) = self.pending_requests.pop_front() {
                                                            let resp = match cancel_res {
                                                                CancelServerResponse::SingleSuccess { ticket } => CommandResponse::CancelSuccess { req_id, ticket },
                                                                CancelServerResponse::BatchSuccess { cancelled } => CommandResponse::BatchCancelSuccess { req_id, cancelled },
                                                                CancelServerResponse::Placeholder { .. } => CommandResponse::CancelSuccess { req_id, ticket: String::new() },
                                                                CancelServerResponse::Error { error } => CommandResponse::CancelError { req_id, error },
                                                            };
                                                            let _ = self.command_responder.send(resp).await;
                                                        }
                                                    }
                                                    continue;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                Message::Binary(data) => {
                                    if let Ok(response) = serde_json::from_slice::<ServerResponse>(data) {
                                        if let ServerResponse::Success(ref pending_order) = response {
                                            self.state.trade_state.add_pending_deal(*pending_order.clone()).await;
                                        }
                                        if let Some(req_id) = self.pending_requests.pop_front() {
                                            match response {
                                                ServerResponse::Success(pending_order) => {
                                                    let _ = self.command_responder.send(CommandResponse::Success { req_id, pending_order }).await;
                                                }
                                                ServerResponse::Fail(fail) => {
                                                    let _ = self.command_responder.send(CommandResponse::Error(fail)).await;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            break;
                        }
                    }
                }
            }
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

impl PendingTradesApiModule {
    async fn notify_waiters_module_stopped(&mut self) {
        let waiters = std::mem::take(&mut self.pending_requests);
        for req_id in waiters {
            let _ = self.command_responder.send(CommandResponse::Shutdown { req_id }).await;
        }
    }
}

impl Drop for PendingTradesApiModule {
    fn drop(&mut self) {
        tracing::debug!(target: "PendingTradesApiModule", "PendingTradesApiModule dropped");
    }
}
