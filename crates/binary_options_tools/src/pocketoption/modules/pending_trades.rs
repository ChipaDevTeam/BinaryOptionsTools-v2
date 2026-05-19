use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

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
    types::{
        CancelPendingOrder, CancelPendingOrderResult, FailOpenOrder, MultiPatternRule,
        OpenPendingOrder, PendingOrder,
    },
};

const PENDING_ORDER_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MISMATCH_RETRIES: usize = 5;

#[derive(Debug)]
pub enum Command {
    OpenPendingOrder {
        open_type: u32,
        amount: Decimal,
        asset: String,
        open_time: u32,
        open_price: Decimal,
        timeframe: u32,
        min_payout: u32,
        command: u32,
        req_id: Uuid,
    },
    CancelPendingOrder {
        ticket: Uuid,
        req_id: Uuid,
    },
}

#[derive(Debug)]
pub enum CommandResponse {
    Success {
        req_id: Uuid,
        pending_order: Box<PendingOrder>,
    },
    Error {
        req_id: Uuid,
        fail: Box<FailOpenOrder>,
    },
    CancelSuccess {
        req_id: Uuid,
        result: Box<CancelPendingOrderResult>,
    },
    CancelError {
        req_id: Uuid,
        ticket: Uuid,
        error: String,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ServerResponse {
    Success(Box<PendingOrder>),
    Fail(Box<FailOpenOrder>),
    CancelSuccess(serde_json::Value),
    CancelFail(Box<FailCancelPendingOrder>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailCancelPendingOrder {
    pub error: String,
    #[serde(default)]
    pub ticket: Option<Uuid>,
}

#[derive(Debug, Clone, Copy)]
struct PendingCancelRequest {
    req_id: Uuid,
    ticket: Uuid,
}

pub struct PendingTradesHandle {
    pub sender: AsyncSender<Command>,
    pub receiver: AsyncReceiver<CommandResponse>,
    /// Single-threaded bottleneck for pending trade calls.
    /// This intentional design prevents head-of-line blocking issues and ensures
    /// that concurrent requests do not interfere with the platform session state.
    /// If concurrency is required in the future, consider a semaphore instead.
    pub call_lock: Arc<tokio::sync::Mutex<()>>,
    pub response_cache: Arc<tokio::sync::Mutex<HashMap<Uuid, CommandResponse>>>,
}

impl Clone for PendingTradesHandle {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            call_lock: self.call_lock.clone(),
            response_cache: self.response_cache.clone(),
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
            response_cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Sets an external lock for request serialization.
    pub fn with_lock(mut self, lock: Arc<tokio::sync::Mutex<()>>) -> Self {
        self.call_lock = lock;
        self
    }

    /// Opens a pending order on the PocketOption platform.
    ///
    #[allow(clippy::too_many_arguments)]
    pub async fn open_pending_order(
        &self,
        open_type: u32,
        amount: Decimal,
        asset: String,
        open_time: u32,
        open_price: Decimal,
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
        match self
            .wait_for_response(
                id,
                "open_pending_order",
                format!("asset: {}, open_type: {}", asset, open_type),
            )
            .await?
        {
            CommandResponse::Success { pending_order, .. } => Ok(*pending_order),
            CommandResponse::Error { fail, .. } => Err(PocketError::FailOpenOrder {
                error: fail.error,
                amount: fail.amount,
                asset: fail.asset,
            }),
            _ => Err(PocketError::General(
                "unexpected response type for open_pending_order".to_string(),
            )),
        }
    }

    /// Cancels a pending order by its ticket.
    pub async fn cancel_pending_order(
        &self,
        ticket: Uuid,
    ) -> PocketResult<CancelPendingOrderResult> {
        let id = Uuid::new_v4();
        self.sender
            .send(Command::CancelPendingOrder { ticket, req_id: id })
            .await
            .map_err(CoreError::from)?;

        match self
            .wait_for_response(id, "cancel_pending_order", format!("ticket: {}", ticket))
            .await?
        {
            CommandResponse::CancelSuccess { result, .. } => Ok(*result),
            CommandResponse::CancelError { error, ticket, .. } => Err(PocketError::General(
                format!("Failed to cancel ticket {ticket}: {error}"),
            )),
            _ => Err(PocketError::General(
                "unexpected response type for cancel_pending_order".to_string(),
            )),
        }
    }

    /// Cancels multiple pending orders and returns per-ticket status.
    pub async fn cancel_pending_orders(
        &self,
        tickets: Vec<Uuid>,
    ) -> PocketResult<Vec<CancelPendingOrderResult>> {
        let mut results = Vec::with_capacity(tickets.len());
        for ticket in tickets {
            results.push(self.cancel_pending_order(ticket).await?);
        }
        Ok(results)
    }

    async fn wait_for_response(
        &self,
        req_id: Uuid,
        task: &str,
        context: String,
    ) -> PocketResult<CommandResponse> {
        let mut mismatch_count = 0;
        loop {
            if let Some(response) = self.response_cache.lock().await.remove(&req_id) {
                return Ok(response);
            }
            match timeout(PENDING_ORDER_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(response)) => {
                    if response.req_id() == req_id {
                        return Ok(response);
                    }

                    warn!(
                        "Received response for different req_id: {}",
                        response.req_id()
                    );
                    self.response_cache
                        .lock()
                        .await
                        .insert(response.req_id(), response);
                    mismatch_count += 1;
                    if mismatch_count >= MAX_MISMATCH_RETRIES {
                        return Err(PocketError::Timeout {
                            task: task.to_string(),
                            context: format!("{context}, exceeded mismatch retries"),
                            duration: PENDING_ORDER_TIMEOUT,
                        });
                    }
                }
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: task.to_string(),
                        context,
                        duration: PENDING_ORDER_TIMEOUT,
                    });
                }
            };
        }
    }
}

impl CommandResponse {
    fn req_id(&self) -> Uuid {
        match self {
            CommandResponse::Success { req_id, .. } => *req_id,
            CommandResponse::Error { req_id, .. } => *req_id,
            CommandResponse::CancelSuccess { req_id, .. } => *req_id,
            CommandResponse::CancelError { req_id, .. } => *req_id,
        }
    }
}

/// This API module handles the creation of pending trade orders.
///
pub struct PendingTradesApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    pending_open_req_ids: VecDeque<Uuid>,
    pending_cancel_requests: VecDeque<PendingCancelRequest>,
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
            pending_open_req_ids: VecDeque::new(),
            pending_cancel_requests: VecDeque::new(),
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
                    tracing::debug!(
                        target: "PendingTradesApiModule",
                        "Received command: {:?}, pending_open_count: {}, pending_cancel_count: {}",
                        cmd,
                        self.pending_open_req_ids.len(),
                        self.pending_cancel_requests.len()
                    );
                    match cmd {
                        Command::OpenPendingOrder { open_type, amount, asset, open_time, open_price, timeframe, min_payout, command, req_id } => {
                            self.pending_open_req_ids.push_back(req_id);
                            let order = OpenPendingOrder::new(open_type, amount, asset, open_time, open_price, timeframe, min_payout, command);
                            self.to_ws_sender.send(Message::text(order.to_string())).await?;
                        }
                        Command::CancelPendingOrder { ticket, req_id } => {
                            self.pending_cancel_requests
                                .push_back(PendingCancelRequest { req_id, ticket });
                            let order = CancelPendingOrder::new(ticket);
                            self.to_ws_sender.send(Message::text(order.to_string())).await?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    let response_result = parse_server_response(msg.as_ref());

                    match response_result {
                        Ok(response) => {
                            match response {
                                ServerResponse::Success(pending_order) => {
                                    self.state.trade_state.add_pending_deal(*pending_order.clone()).await;
                                    info!(target: "PendingTradesApiModule", "Pending trade opened: {}", pending_order.ticket);
                                    if let Some(req_id) = self.pending_open_req_ids.pop_front() {
                                        tracing::debug!(target: "PendingTradesApiModule", "Sending Success response with req_id: {}", req_id);
                                        self.command_responder.send(CommandResponse::Success {
                                            req_id,
                                            pending_order,
                                        }).await?;
                                    } else {
                                        warn!(target: "PendingTradesApiModule", "Received success openPendingOrder but no open req_id was pending. Dropping response to avoid ambiguity.");
                                    }
                                }
                                ServerResponse::Fail(fail) => {
                                    if let Some(req_id) = self.pending_open_req_ids.pop_front() {
                                        tracing::debug!(target: "PendingTradesApiModule", "Forwarding failure for req_id: {}", req_id);
                                        self.command_responder.send(CommandResponse::Error { req_id, fail }).await?;
                                    } else {
                                        warn!(target: "PendingTradesApiModule", "Received failopenPendingOrder but no req_id was pending. Dropping response.");
                                    }
                                }
                                ServerResponse::CancelSuccess(value) => {
                                    if let Some(request) = self.pending_cancel_requests.pop_front() {
                                        let ticket = extract_ticket_from_value(&value).unwrap_or(request.ticket);
                                        let _ = self.state.trade_state.remove_pending_deal(&ticket).await;
                                        self.command_responder.send(CommandResponse::CancelSuccess {
                                            req_id: request.req_id,
                                            result: Box::new(CancelPendingOrderResult {
                                                ticket,
                                                status: "cancelled".to_string(),
                                            }),
                                        }).await?;
                                    } else {
                                        warn!(target: "PendingTradesApiModule", "Received successcancelPendingOrder but no cancellation request was pending.");
                                    }
                                }
                                ServerResponse::CancelFail(fail) => {
                                    if let Some(request) = self.pending_cancel_requests.pop_front() {
                                        self.command_responder.send(CommandResponse::CancelError {
                                            req_id: request.req_id,
                                            ticket: fail.ticket.unwrap_or(request.ticket),
                                            error: fail.error,
                                        }).await?;
                                    } else {
                                        warn!(target: "PendingTradesApiModule", "Received failcancelPendingOrder but no cancellation request was pending.");
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
        ]))
    }
}

fn parse_server_response(msg: &Message) -> Result<ServerResponse, String> {
    match msg {
        Message::Binary(data) => {
            serde_json::from_slice::<ServerResponse>(data).map_err(|e| e.to_string())
        }
        Message::Text(text) => {
            if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                return Ok(res);
            }

            if let Some(start) = text.find('[') {
                if let Ok(serde_json::Value::Array(arr)) =
                    serde_json::from_str::<serde_json::Value>(&text[start..])
                {
                    if arr.len() >= 2 {
                        let event = arr[0].as_str().unwrap_or_default();
                        let payload = arr[1].clone();
                        return match event {
                            "successopenPendingOrder" => {
                                serde_json::from_value::<PendingOrder>(payload)
                                    .map(|order| ServerResponse::Success(Box::new(order)))
                                    .map_err(|e| e.to_string())
                            }
                            "failopenPendingOrder" => {
                                serde_json::from_value::<FailOpenOrder>(payload)
                                    .map(|fail| ServerResponse::Fail(Box::new(fail)))
                                    .map_err(|e| e.to_string())
                            }
                            "successcancelPendingOrder" => {
                                Ok(ServerResponse::CancelSuccess(payload))
                            }
                            "failcancelPendingOrder" => {
                                serde_json::from_value::<FailCancelPendingOrder>(payload)
                                    .map(|fail| ServerResponse::CancelFail(Box::new(fail)))
                                    .map_err(|e| e.to_string())
                            }
                            _ => serde_json::from_str::<ServerResponse>(text)
                                .map_err(|e| e.to_string()),
                        };
                    }
                }
            }

            serde_json::from_str::<ServerResponse>(text).map_err(|e| e.to_string())
        }
        _ => Err("unsupported message type".to_string()),
    }
}

fn extract_ticket_from_value(value: &serde_json::Value) -> Option<Uuid> {
    if let Some(ticket) = value.get("ticket").and_then(|v| v.as_str()) {
        return Uuid::parse_str(ticket).ok();
    }
    if let Some(ticket) = value.as_str() {
        return Uuid::parse_str(ticket).ok();
    }
    None
}

impl Drop for PendingTradesApiModule {
    fn drop(&mut self) {
        tracing::debug!(target: "PendingTradesApiModule", "PendingTradesApiModule dropped");
    }
}
