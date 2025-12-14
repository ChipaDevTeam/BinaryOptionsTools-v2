use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use serde::Deserialize;
use tokio::{select, sync::Mutex, time::timeout};
use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::{
    candle::Candle,
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
};

const HISTORICAL_DATA_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MISMATCH_RETRIES: usize = 5;

#[derive(Debug, Clone)]
pub enum Command {
    GetHistory {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct HistoricalDataHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
    call_lock: Arc<Mutex<()>>,
}

impl HistoricalDataHandle {
    /// Retrieves historical candle data for a specific asset and period.
    ///
    /// **Concurrency Warning:**
    /// This method uses a shared internal lock (`call_lock`) to serialize requests across
    /// all clones of this handle. This ensures that only one `get_history` request is
    /// in-flight at a time for the underlying client actor, matching the actor's single-request
    /// limitation. Concurrent calls will be queued and executed sequentially.
    pub async fn get_history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        // Acquire lock to serialize requests.
        // This ensures that we don't have concurrent get_history calls racing for responses
        // on the shared receiver, matching the single-request limitation of the actor.
        let _guard = self.call_lock.lock().await;

        let id = Uuid::new_v4();
        self.sender
            .send(Command::GetHistory {
                asset: asset.clone(),
                period,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        let mut mismatch_count = 0;
        loop {
            // Mismatched `req_id` cases are typically only expected if other consumers
            // are unexpectedly sharing the same response receiver, or if messages
            // arrive severely out of order (though less likely with serialized requests).
            match timeout(HISTORICAL_DATA_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Success { req_id, candles })) => {
                    if req_id == id {
                        return Ok(candles);
                    } else {
                        warn!("Received response for unknown req_id: {}", req_id);
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: "get_history".to_string(),
                                context: format!(
                                    "asset: {}, period: {}, exceeded mismatch retries",
                                    asset, period
                                ),
                                duration: HISTORICAL_DATA_TIMEOUT,
                            });
                        }
                        continue;
                    }
                }
                Ok(Ok(CommandResponse::Error(e))) => return Err(PocketError::General(e)),
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: "get_history".to_string(),
                        context: format!("asset: {}, period: {}", asset, period),
                        duration: HISTORICAL_DATA_TIMEOUT,
                    })
                }
            }
        }
    }
}

/// This API module handles historical data requests.
///
/// **Concurrency Notes:**
/// - Only one `get_history` request is supported at a time by this module's actor.
/// - The `last_req_id` field is purely for client-side bookkeeping to correlate responses;
///   the PocketOption server protocol does not echo this `req_id` in its responses.
/// - `MAX_MISMATCH_RETRIES` exists to guard against potential misrouted `CommandResponse` messages
///   if the `AsyncReceiver` is shared with other consumers, or if messages arrive out of order
///   due to network conditions or client-side issues.
#[allow(dead_code)] // The state field is not directly read in the module's run logic, but used indirectly by the rule.
pub struct HistoricalDataApiModule {
    _state: Arc<State>, // Prefix with _ to mark as intentionally unused
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
            _state: shared_state, // Prefix with _ to mark as intentionally unused
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
        HistoricalDataHandle {
            sender,
            receiver,
            call_lock: Arc::new(Mutex::new(())),
        }
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
