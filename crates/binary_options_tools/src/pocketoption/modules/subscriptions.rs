use async_trait::async_trait;
use binary_options_tools_core::error::CoreError;
use binary_options_tools_core::reimports::bounded_async;
use binary_options_tools_core::traits::ReconnectCallback;
use binary_options_tools_core::{
    error::CoreResult,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule, RunnerCommand},
};
use core::fmt;
use futures_util::{future::join_all, stream::unfold};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::oneshot;
use tokio::sync::Mutex as TokioMutex;

use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::candle::{
    compile_candles_from_ticks, BaseCandle, HistoryItem, SubscriptionType,
};
use crate::pocketoption::error::PocketError;
use crate::pocketoption::types::{MultiPatternRule, StreamBatch, SubscriptionEvent};
use crate::pocketoption::utils::SocketIoFrame;
use crate::pocketoption::{
    candle::Candle, // Assuming this exists in your types
    error::PocketResult,
    state::State,
};

/// Default maximum cached subscriptions, mirrors [`State`] default `max_subscriptions`.
const DEFAULT_CACHED_MAX: usize = 4;

/// Internal router to distribute command responses to multiple waiters.
pub struct ResponseRouter {
    pending: TokioMutex<HashMap<Uuid, oneshot::Sender<CommandResponse>>>,
}

impl ResponseRouter {
    pub fn new(receiver: AsyncReceiver<CommandResponse>) -> Arc<Self> {
        let router = Arc::new(Self {
            pending: TokioMutex::new(HashMap::new()),
        });
        let router_clone = router.clone();
        tokio::spawn(async move {
            while let Ok(resp) = receiver.recv().await {
                if let Some(id) = get_command_id(&resp) {
                    let mut pending = router_clone.pending.lock().await;
                    if let Some(tx) = pending.remove(&id) {
                        if tx.send(resp).is_err() {
                            tracing::trace!(target: "ResponseRouter", "Failed to route response: receiver dropped");
                        }
                    }
                }
            }
            let mut pending = router_clone.pending.lock().await;
            for (id, tx) in pending.drain() {
                if tx
                    .send(CommandResponse::Shutdown { command_id: id })
                    .is_err()
                {
                    tracing::trace!(target: "ResponseRouter", "Failed to send shutdown notification: receiver dropped");
                }
            }
        });
        router
    }

    pub async fn wait_for(&self, id: Uuid) -> PocketResult<CommandResponse> {
        let rx = self.register(id).await;
        rx.await.map_err(|_| PocketError::ModuleStopped {
            module_name: "SubscriptionsApiModule".to_string(),
            context: "Response router channel closed".to_string(),
        })
    }

    pub async fn register(&self, id: Uuid) -> oneshot::Receiver<CommandResponse> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        rx
    }
}

fn get_command_id(resp: &CommandResponse) -> Option<Uuid> {
    match resp {
        CommandResponse::SubscriptionSuccess { command_id, .. } => Some(*command_id),
        CommandResponse::SubscriptionFailed { command_id, .. } => Some(*command_id),
        CommandResponse::History { command_id, .. } => Some(*command_id),
        CommandResponse::UnsubscriptionSuccess { command_id } => Some(*command_id),
        CommandResponse::UnsubscriptionFailed { command_id, .. } => Some(*command_id),
        CommandResponse::SubscriptionCount { command_id, .. } => Some(*command_id),
        CommandResponse::HistoryFailed { command_id, .. } => Some(*command_id),
        CommandResponse::Shutdown { command_id } => Some(*command_id),
    }
}

#[derive(Serialize)]
pub struct ChangeSymbol {
    // Making it public as it may be used somewhere else
    pub asset: String,
    pub period: i64,
}

#[derive(Deserialize)]
pub struct History {
    pub asset: String,
    pub period: u32,
    #[serde(default)]
    pub candles: Option<Vec<BaseCandle>>,
    #[serde(default)]
    pub history: Option<Vec<HistoryItem>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ServerResponse {
    History(History),
    Candle(StreamBatch),
}

impl fmt::Display for ChangeSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "42[\"changeSymbol\",{}]",
            serde_json::to_string(&self).map_err(|_| fmt::Error)?
        )
    }
}

/// Per-stream buffer. Ticks for a symbol arrive every few hundred milliseconds
/// and aggregated subscriptions (e.g. 60s+ candles) only drain the buffer when
/// the consumer polls, so this must tolerate a consumer that is busy elsewhere.
const MAX_CHANNEL_CAPACITY: usize = 1024;
/// Symbols the server will stream at the same time, as measured on the live demo
/// servers (2026-10-30): the `changeSymbol` chart symbol plus one watchlist symbol.
///
/// The connection is not refused past this point - the extra subscriptions simply
/// never receive ticks, which used to surface as "Subscription stream timed out
/// after 60s" with no hint about why. Subscribing past it now warns once.
pub const OBSERVED_CONCURRENT_STREAM_LIMIT: usize = 2;

const RECONNECT_INITIAL_DELAY: Duration = Duration::from_secs(2);
const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_RECEIVE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Maximum subscriptions limit reached")]
    MaxSubscriptionsReached,
    #[error("Subscription already exists")]
    SubscriptionAlreadyExists,
}

/// Command enum for the `SubscriptionsApiModule`.
#[derive(Debug, Clone)]
pub enum Command {
    /// Subscribe to an asset's stream
    Subscribe {
        asset: String,
        sub_type: SubscriptionType,
        command_id: Uuid,
    },
    /// Unsubscribe from an asset's stream
    /// If subscription_id is None, removes all subscriptions for the asset (legacy behavior).
    /// If Some(id), removes only the specific subscription with that ID.
    Unsubscribe {
        asset: String,
        subscription_id: Option<Uuid>,
        command_id: Uuid,
    },
    /// History
    History {
        asset: String,
        period: u32,
        command_id: Uuid,
    },
    /// Cancels the pending [`Command::History`] identified by `command_id`.
    ///
    /// Sent when the caller is dropped before its history response arrives, so
    /// the module does not keep rejecting later requests as duplicates. The id
    /// is what makes this safe: a late cancellation from one caller must not
    /// remove a newer request for the same `(asset, period)`.
    CancelHistory {
        asset: String,
        period: u32,
        command_id: Uuid,
    },
    /// Requests the number of active subscriptions
    SubscriptionCount { command_id: Uuid },
}

/// Response enum for subscription commands
#[derive(Debug)]
pub enum CommandResponse {
    /// Successful subscription with stream receiver
    SubscriptionSuccess {
        command_id: Uuid,
        subscription_id: Uuid,
        stream_receiver: AsyncReceiver<SubscriptionEvent>,
    },
    /// Subscription failed
    SubscriptionFailed {
        command_id: Uuid,
        error: Box<PocketError>,
    },
    /// History Response
    History { command_id: Uuid, data: Vec<Candle> },
    /// Unsubscription successful
    UnsubscriptionSuccess { command_id: Uuid },
    /// Unsubscription failed
    UnsubscriptionFailed {
        command_id: Uuid,
        error: Box<PocketError>,
    },
    /// Returns the number of active subscriptions and the configured maximum
    SubscriptionCount {
        command_id: Uuid,
        count: u32,
        max: usize,
    },
    /// History failed
    HistoryFailed {
        command_id: Uuid,
        error: Box<PocketError>,
    },
    /// The module has stopped and cannot fulfill the request.
    Shutdown { command_id: Uuid },
}

/// Represents the data sent through the subscription stream.
pub struct SubscriptionStream {
    receiver: AsyncReceiver<SubscriptionEvent>,
    sender: Option<AsyncSender<Command>>,
    router: Arc<ResponseRouter>,
    asset: String,
    sub_type: SubscriptionType,
    subscription_id: Uuid,
}

/// Callback for when there is a disconnection
struct SubscriptionCallback;

#[async_trait]
impl ReconnectCallback<State> for SubscriptionCallback {
    async fn call(&self, state: Arc<State>, ws_sender: &AsyncSender<Message>) -> CoreResult<()> {
        tokio::time::sleep(RECONNECT_INITIAL_DELAY).await;
        // Resubscribe to all active subscriptions
        let subscriptions = state.active_subscriptions.read().await.clone();

        // Send subscription messages concurrently (all unique types per asset)
        let mut futures = Vec::new();
        for (symbol, vec) in subscriptions {
            // Keep track of unique periods to avoid redundant subfor messages
            let mut seen_periods = Vec::new();
            for (_, sub_type, _) in vec {
                let period = sub_type.period_secs().unwrap_or(1);
                if !seen_periods.contains(&period) {
                    seen_periods.push(period);
                    let ws_sender = ws_sender.clone();
                    let symbol_clone = symbol.clone();
                    futures.push(async move {
                        send_subscribe_message(&ws_sender, &symbol_clone, period).await
                    });
                }
            }
        }

        let results = join_all(futures).await;

        // Check for errors
        for result in results {
            result?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct SubscriptionsHandle {
    sender: AsyncSender<Command>,
    router: Arc<ResponseRouter>,
    cached_max: Arc<AtomicUsize>,
}

/// Cancels the module-side pending history request for `(asset, period)` if the
/// caller is dropped before the response arrives.
///
/// `SubscriptionsApiModule` rejects a second history request for the same
/// `(asset, period)` while one is pending. When a caller is cancelled (for
/// example when Python abandons the await), nothing else clears that pending
/// entry, so every later request would fail as a duplicate forever.
struct HistoryRequestGuard {
    sender: AsyncSender<Command>,
    asset: String,
    period: u32,
    command_id: Uuid,
    armed: bool,
}

impl Drop for HistoryRequestGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let command = Command::CancelHistory {
            asset: self.asset.clone(),
            period: self.period,
            command_id: self.command_id,
        };
        // `try_send` drops the command when the channel is full, which would
        // leave the pending entry behind, so retry from a task in that case.
        // `Ok(true)` means it was queued; an error means the module is gone and
        // there is nothing left to cancel.
        if let Ok(false) = self.sender.as_sync().try_send(command.clone()) {
            let sender = self.sender.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = sender.send(command).await;
                });
            }
        }
    }
}

impl SubscriptionsHandle {
    /// Subscribe to an asset's real-time data stream.
    ///
    /// # Arguments
    /// * `asset` - The asset symbol to subscribe to
    ///
    /// # Returns
    /// * `PocketResult<(Uuid, AsyncReceiver<StreamData>)>` - Subscription ID and data receiver
    ///
    /// # Errors
    /// * Returns error if maximum subscriptions reached
    /// * Returns error if subscription fails
    pub async fn subscribe(
        &self,
        asset: String,
        sub_type: SubscriptionType,
    ) -> PocketResult<SubscriptionStream> {
        let id = Uuid::new_v4();
        let receiver = self.router.register(id).await;
        self.sender
            .send(Command::Subscribe {
                asset: asset.clone(),
                sub_type: sub_type.clone(),
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the subscription response with timeout
        match tokio::time::timeout(SUBSCRIBE_TIMEOUT, receiver)
            .await
            .map_err(|_| {
                PocketError::General(format!(
                    "Subscription timed out after {:?} waiting for server response for asset: {}",
                    SUBSCRIBE_TIMEOUT, asset
                ))
            })?
            .map_err(|_| PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "Response router channel closed".to_string(),
            })? {
            CommandResponse::SubscriptionSuccess {
                command_id: _,
                subscription_id,
                stream_receiver,
            } => Ok(SubscriptionStream {
                receiver: stream_receiver,
                sender: Some(self.sender.clone()),
                router: self.router.clone(),
                asset,
                sub_type,
                subscription_id,
            }),
            CommandResponse::SubscriptionFailed { error, .. } => Err(*error),
            CommandResponse::Shutdown { .. } => Err(PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "SubscriptionsApiModule stopped during request".to_string(),
            }),
            _ => Err(PocketError::General(
                "Unexpected response to subscribe command".into(),
            )),
        }
    }

    /// Unsubscribe from an asset's stream.
    ///
    /// # Arguments
    /// * `subscription_id` - The ID of the subscription to cancel
    ///
    /// # Returns
    /// * `PocketResult<()>` - Success or error
    pub async fn unsubscribe(&self, asset: String) -> PocketResult<()> {
        let id = Uuid::new_v4();
        let receiver = self.router.register(id).await;
        self.sender
            .send(Command::Unsubscribe {
                asset,
                subscription_id: None, // Remove all subscriptions for this asset
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the unsubscription response with timeout
        match tokio::time::timeout(SUBSCRIBE_TIMEOUT, receiver)
            .await
            .map_err(|_| {
                PocketError::General(format!(
                    "Unsubscribe timed out after {:?} waiting for server response",
                    SUBSCRIBE_TIMEOUT
                ))
            })?
            .map_err(|_| PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "Response router channel closed".to_string(),
            })? {
            CommandResponse::UnsubscriptionSuccess { .. } => Ok(()),
            CommandResponse::UnsubscriptionFailed { error, .. } => Err(*error),
            CommandResponse::Shutdown { .. } => Err(PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "SubscriptionsApiModule stopped during request".to_string(),
            }),
            _ => Err(PocketError::General(
                "Unexpected response to unsubscribe command".into(),
            )),
        }
    }

    /// Get the number of active subscriptions.
    ///
    /// # Returns
    /// * `PocketResult<usize>` - Number of active subscriptions
    pub async fn get_active_subscriptions_count(&self) -> PocketResult<u32> {
        let id = Uuid::new_v4();
        let receiver = self.router.register(id).await;
        self.sender
            .send(Command::SubscriptionCount { command_id: id })
            .await
            .map_err(CoreError::from)?;
        // Wait for the subscription count response with timeout
        match tokio::time::timeout(SUBSCRIBE_TIMEOUT, receiver)
            .await
            .map_err(|_| {
                PocketError::General(format!(
                    "Subscription count request timed out after {:?} waiting for server response",
                    SUBSCRIBE_TIMEOUT
                ))
            })?
            .map_err(|_| PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "Response router channel closed".to_string(),
            })? {
            CommandResponse::SubscriptionCount { count, max, .. } => {
                self.cached_max.store(max, Ordering::Relaxed);
                Ok(count)
            }
            CommandResponse::Shutdown { .. } => Err(PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "SubscriptionsApiModule stopped during request".to_string(),
            }),
            _ => Err(PocketError::General(
                "Unexpected response to subscription count command".into(),
            )),
        }
    }

    /// Gets the history for an asset with its period
    ///
    /// **Constraint:**
    /// Only one outstanding history call per `(asset, period)` is supported.
    /// Duplicate requests will be rejected with `HistoryFailed`.
    ///
    /// # Arguments
    /// * `asset` - The asset symbol
    /// * `period` - The period in minutes
    /// # Returns
    /// * `PocketResult<Vec<Candle>>` - Vector of candles
    pub async fn history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let id = Uuid::new_v4();
        let receiver = self.router.register(id).await;
        self.sender
            .send(Command::History {
                asset: asset.clone(),
                period,
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;

        // If this future is dropped (for example cancelled by the caller)
        // before the module resolves the request, cancel the pending entry so
        // later requests for the same (asset, period) are not rejected as
        // duplicates.
        let mut guard = HistoryRequestGuard {
            sender: self.sender.clone(),
            asset,
            period,
            command_id: id,
            armed: true,
        };

        match tokio::time::timeout(SUBSCRIBE_TIMEOUT, receiver).await {
            Ok(Ok(response)) => {
                guard.armed = false;
                match response {
                    CommandResponse::History { data, .. } => Ok(data),
                    CommandResponse::HistoryFailed { error, .. } => Err(*error),
                    CommandResponse::Shutdown { .. } => Err(PocketError::ModuleStopped {
                        module_name: "SubscriptionsApiModule".to_string(),
                        context: "SubscriptionsApiModule stopped during request".to_string(),
                    }),
                    _ => Err(PocketError::General(
                        "Unexpected response to history command".into(),
                    )),
                }
            }
            Ok(Err(_)) => Err(PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "Response router channel closed".to_string(),
            }),
            Err(_) => Err(PocketError::General(format!(
                "History request timed out after {:?} waiting for server response",
                SUBSCRIBE_TIMEOUT
            ))),
        }
    }
}

/// The API module for handling subscription operations.
pub struct SubscriptionsApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    /// Symbol the server is currently streaming the chart for.
    ///
    /// PocketOption streams the `changeSymbol` symbol plus a small number of
    /// watchlist symbols per session; there is no working "unsubscribe" event
    /// (verified against the live server: `unsubscribeSymbol` is ignored), so the
    /// only way to free a stream is to point the chart at another symbol. Tracking
    /// it lets a released subscription hand its stream to a symbol still in use.
    chart_symbol: Option<String>,
}

#[async_trait]
impl ApiModule<State> for SubscriptionsApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = SubscriptionsHandle;

    fn new(
        state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self {
            state,
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
            chart_symbol: None,
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        SubscriptionsHandle {
            sender,
            router: ResponseRouter::new(receiver),
            cached_max: Arc::new(AtomicUsize::new(DEFAULT_CACHED_MAX)),
        }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                cmd_res = self.command_receiver.recv() => {
                    let cmd = match cmd_res {
                        Ok(cmd) => cmd,
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            return Ok(());
                        }
                    };
                    match cmd {
                        Command::Subscribe {
                            asset,
                            sub_type,
                            command_id,
                        } => {

                            let period = sub_type.period_secs().unwrap_or(1);
                            let (stream_sender, stream_receiver) =
                                bounded_async(MAX_CHANNEL_CAPACITY);
                            let subscription_id = Uuid::new_v4();

                            if let Err(e) = self.add_subscription(asset.clone(), sub_type.clone(), stream_sender.clone(), subscription_id).await {
                                if let Err(e) = self.command_responder.send(CommandResponse::SubscriptionFailed {
                                    command_id,
                                    error: Box::new(e),
                                }).await {
                                    warn!(target: "SubscriptionsApiModule", "Failed to send SubscriptionFailed (add_subscription) response: {}", e);
                                }
                                continue;
                            }

                            if let Err(e) = self.point_chart_at(&asset, period).await {
                                let _ = self.remove_subscription(&asset, Some(subscription_id)).await;
                                if let Err(e) = self.command_responder.send(CommandResponse::SubscriptionFailed {
                                    command_id,
                                    error: Box::new(e.into()),
                                }).await {
                                    warn!(target: "SubscriptionsApiModule", "Failed to send SubscriptionFailed (chart) response: {}", e);
                                }
                                continue;
                            }

                            if let Err(e) = self.send_subfor(&asset).await {
                                let _ = self.remove_subscription(&asset, Some(subscription_id)).await;
                                if let Err(e) = self.command_responder.send(CommandResponse::SubscriptionFailed {
                                    command_id,
                                    error: Box::new(e.into()),
                                }).await {
                                    warn!(target: "SubscriptionsApiModule", "Failed to send SubscriptionFailed (send_subscribe) response: {}", e);
                                }
                                continue;
                            }

                            self.warn_if_over_stream_limit().await;

                            if let Err(e) = self.command_responder.send(CommandResponse::SubscriptionSuccess {
                                command_id,
                                subscription_id,
                                stream_receiver,
                            }).await {
                                warn!(target: "SubscriptionsApiModule", "Failed to send SubscriptionSuccess response: {}", e);
                            }
                        }
                        Command::Unsubscribe { asset, subscription_id, command_id } => {
                            match self.remove_subscription(&asset, subscription_id).await {
                                Ok(b) => {
                                    if b {
                                        self.repoint_released_chart(&asset).await;
                                        if let Err(e) = self.command_responder.send(CommandResponse::UnsubscriptionSuccess { command_id }).await {
                                            warn!(target: "SubscriptionsApiModule", "Failed to send UnsubscriptionSuccess response: {}", e);
                                        }
                                    } else {
                                        if let Err(e) = self.command_responder.send(CommandResponse::UnsubscriptionFailed {
                                            command_id,
                                            error: Box::new(PocketError::General("Subscription not found".to_string())),
                                        }).await {
                                            warn!(target: "SubscriptionsApiModule", "Failed to send UnsubscriptionFailed (not found) response: {}", e);
                                        }
                                    }
                                },
                                Err(err) => {
                                    if let Err(e) = self.command_responder.send(CommandResponse::UnsubscriptionFailed {
                                        command_id,
                                        error: Box::new(err.into()),
                                    }).await {
                                        warn!(target: "SubscriptionsApiModule", "Failed to send UnsubscriptionFailed (error) response: {}", e);
                                    }
                                }
                            }
                        },
                        Command::SubscriptionCount { command_id } => {
                            let subscriptions = self.state.active_subscriptions.read().await;
                            let count = subscriptions.values().map(|v| v.len()).sum::<usize>() as u32;
                            if let Err(e) = self.command_responder.send(CommandResponse::SubscriptionCount {
                                command_id,
                                count,
                                max: OBSERVED_CONCURRENT_STREAM_LIMIT,
                            }).await {
                                warn!(target: "SubscriptionsApiModule", "Failed to send SubscriptionCount response: {}", e);
                            }
                        },
                        Command::History { asset, period, command_id } => {
                            let is_duplicate = self.state.histories.read().await.iter().any(|(a, p, _)| a == &asset && *p == period);
                            if is_duplicate {
                                if let Err(e) = self.command_responder.send(CommandResponse::HistoryFailed {
                                    command_id,
                                    error: Box::new(PocketError::General(format!("Duplicate history request for asset: {}, period: {}", asset, period))),
                                }).await {
                                    warn!(target: "SubscriptionsApiModule", "Failed to send HistoryFailed (duplicate) response: {}", e);
                                }
                            } else if let Err(e) = self.send_subscribe_message(&asset, period).await {
                                if let Err(e) = self.command_responder.send(CommandResponse::HistoryFailed {
                                    command_id,
                                    error: Box::new(e.into()),
                                }).await {
                                    warn!(target: "SubscriptionsApiModule", "Failed to send HistoryFailed (subscribe error) response: {}", e);
                                }
                            } else {
                                self.state.histories.write().await.push((asset, period, command_id));
                            }
                        }
                        Command::CancelHistory {
                            asset,
                            period,
                            command_id,
                        } => {
                            self.state.histories.write().await.retain(|(a, p, id)| {
                                !(a == &asset && *p == period && *id == command_id)
                            });
                        }
                    }
                },
                msg_res = self.message_receiver.recv() => {
                    let msg = match msg_res {
                        Ok(msg) => msg,
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            return Ok(());
                        }
                    };

                    let response = match msg.as_ref() {
                        Message::Binary(data) => match serde_json::from_slice::<ServerResponse>(data) {
                            Ok(res) => Some(res),
                            Err(e) => {
                                warn!(target: "SubscriptionsApiModule", "Failed to parse binary ServerResponse: {}", e);
                                None
                            }
                        },
                        Message::Text(text) => {
                            if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                                Some(res)
                            } else if let Some(frame) = SocketIoFrame::parse(text) {
                                let event_payload: Option<(String, serde_json::Value)> = frame.extract_event();
                                if let Some((event, payload)) = event_payload {
                                    match event.as_str() {
                                        "updateStream" | "updateHistory" | "updateHistoryNewFast" | "updateHistoryNew" | "history" | "loadHistoryPeriod" => {
                                            match serde_json::from_value::<ServerResponse>(payload) {
                                                Ok(res) => Some(res),
                                                Err(e) => {
                                                    warn!(target: "SubscriptionsApiModule", "Failed to parse event {} payload: {}", event, e);
                                                    None
                                                }
                                            }
                                        }
                                        _ => None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(response) = response {
                        match response {
                            ServerResponse::Candle(data) => {
                                // Live ticks carry the broker's wall clock; history is UTC, so
                                // each tick is normalised before it reaches the aggregators.
                                for tick in data.0 {
                                    let timestamp = self.state.normalize_stream_timestamp(tick.timestamp);
                                    if let Err(e) = self.forward_data_to_stream(&tick.symbol, tick.price, timestamp).await {
                                        warn!(target: "SubscriptionsApiModule", "Failed to forward data: {}", e);
                                    }
                                }
                            },
                            ServerResponse::History(data) => {
                                let mut id = None;
                                self.state.histories.write().await.retain(|(asset, period, c_id)| {
                                    if asset == &data.asset && *period == data.period {
                                        id = Some(*c_id);
                                        false
                                    } else {
                                        true
                                    }
                                });
                                if let Some(command_id) = id {
                                    let symbol = data.asset.clone();
                                    let candles_res = if let Some(candles) = data.candles {
                                        candles.into_iter()
                                            .map(|c| Candle::try_from((c, symbol.clone())))
                                            .collect::<Result<Vec<_>, _>>()
                                            .map_err(|e| PocketError::General(e.to_string()))
                                    } else if let Some(history) = data.history {
                                        // `updateHistoryNewFast` is stamped with the broker's
                                        // wall clock, while ticks compiled from `loadHistoryPeriod`
                                        // follow the UTC `time` we send. Shift it onto UTC so both
                                        // history paths agree.
                                        let delta = -self.state.broker_clock_offset();
                                        let history: Vec<HistoryItem> =
                                            history.into_iter().map(|item| item.shifted(delta)).collect();
                                        Ok(compile_candles_from_ticks(&history, data.period, &symbol))
                                    } else {
                                        Ok(Vec::new())
                                    };

                                    match candles_res {
                                        Ok(candles) => {
                                            if let Err(e) = self.command_responder.send(CommandResponse::History {
                                                command_id,
                                                data: candles
                                            }).await {
                                                warn!(target: "SubscriptionsApiModule", "Failed to send History response: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(e) = self.command_responder.send(CommandResponse::HistoryFailed {
                                                command_id,
                                                error: Box::new(e)
                                            }).await {
                                                warn!(target: "SubscriptionsApiModule", "Failed to send HistoryFailed response: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn callback(
        _shared_state: Arc<State>,
        _command_receiver: AsyncReceiver<Self::Command>,
        _command_responder: AsyncSender<Self::CommandResponse>,
        _message_receiver: AsyncReceiver<Arc<Message>>,
        _to_ws_sender: AsyncSender<Message>,
    ) -> CoreResult<Option<Box<dyn ReconnectCallback<State>>>> {
        Ok(Some(Box::new(SubscriptionCallback)))
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec![
            "updateStream",
            "updateHistory",
            "updateHistoryNewFast",
            "updateHistoryNew",
            "successChangeSymbol",
            "successSubfor",
            "history",
            "loadHistoryPeriod",
        ]))
    }
}

impl SubscriptionsApiModule {
    /// Notifies all pending waiters that the module has stopped.
    async fn notify_waiters_module_stopped(&mut self) {
        // Histories are notified via the CommandResponse channel (ResponseRouter handles it)
        let mut histories_lock = self.state.histories.write().await;
        let pending = std::mem::take(&mut *histories_lock);
        for (_, _, command_id) in pending {
            if let Err(e) = self
                .command_responder
                .send(CommandResponse::Shutdown { command_id })
                .await
            {
                warn!(target: "SubscriptionsApiModule", "Failed to send Shutdown response in notify_waiters: {}", e);
            }
        }

        // Active streams should also be notified
        let mut subscriptions_lock = self.state.active_subscriptions.write().await;
        let active = std::mem::take(&mut *subscriptions_lock);
        for (_, subs) in active {
            for (sender, _, _) in subs {
                if let Err(e) = sender
                    .send(SubscriptionEvent::Terminated {
                        reason: "SubscriptionsApiModule stopped".to_string(),
                    })
                    .await
                {
                    warn!(target: "SubscriptionsApiModule", "Failed to send Terminated event to stream: {}", e);
                }
            }
        }
    }

    /// Add a new subscription.
    async fn add_subscription(
        &mut self,
        asset: String,
        sub_type: SubscriptionType,
        stream_sender: AsyncSender<SubscriptionEvent>,
        subscription_id: Uuid,
    ) -> PocketResult<()> {
        let mut subscriptions = self.state.active_subscriptions.write().await;
        let entry = subscriptions.entry(asset).or_insert_with(Vec::new);
        entry.push((stream_sender, sub_type, subscription_id));
        Ok(())
    }

    /// Remove a subscription.
    async fn remove_subscription(
        &mut self,
        asset: &str,
        subscription_id: Option<Uuid>,
    ) -> CoreResult<bool> {
        let (removed_senders, removed_at_least_one) = {
            let mut subscriptions = self.state.active_subscriptions.write().await;
            let mut removed_senders = Vec::new();
            let mut removed_at_least_one = false;

            if let Some(vec) = subscriptions.get_mut(asset) {
                if let Some(sub_id) = subscription_id {
                    if let Some(idx) = vec.iter().position(|(_, _, id)| *id == sub_id) {
                        let (stream_sender, _, _) = vec.remove(idx);
                        removed_senders.push(stream_sender);
                        removed_at_least_one = true;
                        if vec.is_empty() {
                            subscriptions.remove(asset);
                        }
                    }
                } else {
                    removed_senders = vec
                        .drain(..)
                        .map(|(stream_sender, _, _)| stream_sender)
                        .collect();
                    removed_at_least_one = !removed_senders.is_empty();
                    subscriptions.remove(asset);
                }
            }
            (removed_senders, removed_at_least_one)
        };

        for stream_sender in removed_senders {
            if let Err(e) = stream_sender
                .send(SubscriptionEvent::Terminated {
                    reason: "Unsubscribed from main module".to_string(),
                })
                .await
            {
                warn!(target: "SubscriptionsApiModule", "Failed to send Terminated event during remove_subscription: {}", e);
            }
        }

        Ok(removed_at_least_one)
    }

    async fn send_subscribe_message(&self, asset: &str, period: u32) -> CoreResult<()> {
        send_subscribe_message(&self.to_ws_sender, asset, period).await
    }

    async fn forward_data_to_stream(
        &self,
        asset: &str,
        price: Decimal,
        timestamp: i64,
    ) -> CoreResult<()> {
        let senders: Vec<AsyncSender<SubscriptionEvent>> = {
            let subscriptions = self.state.active_subscriptions.read().await;
            if let Some(vec) = subscriptions.get(asset) {
                vec.iter().map(|(sender, _, _)| sender.clone()).collect()
            } else {
                return Ok(());
            }
        };

        let update = SubscriptionEvent::Update {
            asset: asset.to_string(),
            price,
            timestamp,
        };

        // Never await on a stream's channel here: this loop serves every
        // subscription, so one consumer that stops polling would otherwise stall
        // the ticks of all the others until they time out.
        for stream_sender in senders {
            match stream_sender.as_sync().try_send(update.clone()) {
                Ok(true) => {}
                Ok(false) => {
                    warn!(target: "SubscriptionsApiModule", "Stream buffer full for {asset}; dropping tick (consumer is not keeping up)");
                }
                Err(_) => {} // receiver dropped; cleanup happens through Unsubscribe
            }
        }
        Ok(())
    }
}

impl SubscriptionStream {
    /// Get the asset symbol for this subscription stream
    pub fn asset(&self) -> &str {
        &self.asset
    }

    /// Unsubscribe from the stream
    pub async fn unsubscribe(&self) -> PocketResult<()> {
        let command_id = Uuid::new_v4();
        let receiver = self.router.register(command_id).await;
        if let Some(sender) = &self.sender {
            sender
                .send(Command::Unsubscribe {
                    asset: self.asset.clone(),
                    subscription_id: Some(self.subscription_id),
                    command_id,
                })
                .await
                .map_err(CoreError::from)?;
        } else {
            return Ok(());
        }

        match tokio::time::timeout(SUBSCRIBE_TIMEOUT, receiver)
            .await
            .map_err(|_| {
                PocketError::General(format!(
                    "Unsubscribe timed out after {:?} waiting for server response",
                    SUBSCRIBE_TIMEOUT
                ))
            })?
            .map_err(|_| PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "Response router channel closed".to_string(),
            })? {
            CommandResponse::UnsubscriptionSuccess { .. } => Ok(()),
            CommandResponse::UnsubscriptionFailed { error, .. } => Err(*error),
            CommandResponse::Shutdown { .. } => Err(PocketError::ModuleStopped {
                module_name: "SubscriptionsApiModule".to_string(),
                context: "SubscriptionsApiModule stopped during request".to_string(),
            }),
            _ => Err(PocketError::General(
                "Unexpected response to unsubscribe command".into(),
            )),
        }
    }

    /// Receive the next candle from the stream
    pub async fn receive(&mut self) -> PocketResult<Candle> {
        self.receive_with_timeout(DEFAULT_RECEIVE_TIMEOUT).await
    }

    /// Receive the next candle from the stream with a custom timeout
    pub async fn receive_with_timeout(&mut self, timeout: Duration) -> PocketResult<Candle> {
        loop {
            match tokio::time::timeout(timeout, self.receiver.recv()).await {
                Ok(Ok(crate::pocketoption::types::SubscriptionEvent::Update {
                    asset,
                    price,
                    timestamp,
                })) => {
                    if asset == self.asset {
                        let candle = self.process_update(timestamp, price)?;
                        if let Some(candle) = candle {
                            return Ok(candle);
                        }
                    }
                }
                Ok(Ok(crate::pocketoption::types::SubscriptionEvent::Terminated { reason })) => {
                    return Err(PocketError::General(format!("Stream terminated: {reason}")));
                }
                Ok(Err(e)) => {
                    return Err(CoreError::from(e).into());
                }
                Err(_) => {
                    return Err(PocketError::General(format!(
                        "Subscription stream timed out after {:?} waiting for data from {}",
                        timeout, self.asset
                    )));
                }
            }
        }
    }

    /// Process an incoming price update based on subscription type
    fn process_update(&mut self, timestamp: i64, price: Decimal) -> PocketResult<Option<Candle>> {
        let asset = self.asset().to_string();
        let price_f64 = price.to_f64().ok_or_else(|| {
            PocketError::General(format!(
                "Failed to convert price {} to f64 for asset {} at timestamp {}",
                price, asset, timestamp
            ))
        })?;
        if let Some(c) = self
            .sub_type
            .update(&BaseCandle::from((timestamp, price_f64)))?
        {
            Ok(Some(Candle::try_from((c, asset)).map_err(|e| {
                PocketError::General(format!("Failed to convert candle: {e}"))
            })?))
        } else {
            Ok(None)
        }
    }

    /// Convert to a futures Stream.
    ///
    /// This method consumes the `SubscriptionStream` by value. After calling `to_stream()`,
    /// cleanup is handled by the returned stream's `Drop` implementation, which will
    /// automatically send an unsubscribe command when the stream is dropped.
    pub fn to_stream(self) -> impl futures_util::Stream<Item = PocketResult<Candle>> + 'static {
        Box::pin(unfold(self, |mut stream| async move {
            let result = stream.receive().await;
            Some((result, stream))
        }))
    }

    /// Check if the subscription type uses time alignment
    pub fn is_time_aligned(&self) -> bool {
        matches!(self.sub_type, SubscriptionType::TimeAligned { .. })
    }

    /// Get the current subscription type
    pub fn subscription_type(&self) -> &SubscriptionType {
        &self.sub_type
    }
}

impl Clone for SubscriptionStream {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            sender: self.sender.clone(),
            router: self.router.clone(),
            asset: self.asset.clone(),
            sub_type: self.sub_type.clone(),
            subscription_id: self.subscription_id,
        }
    }
}

async fn send_subscribe_message(
    ws_sender: &AsyncSender<Message>,
    asset: &str,
    period: u32,
) -> CoreResult<()> {
    ws_sender
        .send(Message::text(
            ChangeSymbol {
                asset: asset.to_string(),
                period: period as i64,
            }
            .to_string(),
        ))
        .await
        .map_err(CoreError::from)?;
    ws_sender
        .send(Message::text(format!("42[\"subfor\",\"{asset}\"]")))
        .await
        .map_err(CoreError::from)?;
    Ok(())
}

impl Drop for SubscriptionStream {
    fn drop(&mut self) {
        if let Some(sender) = &self.sender {
            let drop_command = Command::Unsubscribe {
                asset: self.asset.clone(),
                subscription_id: Some(self.subscription_id),
                command_id: Uuid::nil(),
            };
            let _ = sender.as_sync().try_send(drop_command);
        }
    }
}

impl SubscriptionsApiModule {
    /// Warns once when more symbols are subscribed than the server will stream.
    ///
    /// This is guidance, not enforcement: the extra streams may still arrive if the
    /// server has spare capacity, but in practice they starve, and a user who knows
    /// the ceiling can rotate instead of waiting out a timeout per symbol.
    async fn warn_if_over_stream_limit(&self) {
        use std::sync::atomic::{AtomicBool, Ordering};
        static WARNED: AtomicBool = AtomicBool::new(false);

        let distinct = {
            let subscriptions = self.state.active_subscriptions.read().await;
            subscriptions.len()
        };
        if distinct > OBSERVED_CONCURRENT_STREAM_LIMIT && !WARNED.swap(true, Ordering::Relaxed) {
            warn!(
                target: "SubscriptionsApiModule",
                "{distinct} symbols are subscribed, but the server was measured streaming only \
                 ~{OBSERVED_CONCURRENT_STREAM_LIMIT} at a time; the extra ones will receive no \
                 ticks. Rotate a small pool of subscriptions, or poll `candles()` for snapshot \
                 data instead of holding a stream per symbol."
            );
        }
    }

    /// Adds `asset` to the server-side watchlist that is streamed alongside the chart.
    async fn send_subfor(&self, asset: &str) -> CoreResult<()> {
        self.to_ws_sender
            .send(Message::text(format!(r#"42["subfor","{asset}"]"#)))
            .await
            .map_err(CoreError::from)
    }

    /// Points the server's chart stream at `asset`, which is what actually starts
    /// (and stops) a per-symbol tick stream.
    async fn point_chart_at(&mut self, asset: &str, period: u32) -> CoreResult<()> {
        if self.chart_symbol.as_deref() == Some(asset) {
            return Ok(());
        }
        self.to_ws_sender
            .send(Message::text(
                ChangeSymbol {
                    asset: asset.to_string(),
                    period: period as i64,
                }
                .to_string(),
            ))
            .await
            .map_err(CoreError::from)?;
        self.chart_symbol = Some(asset.to_string());
        Ok(())
    }

    /// Hands a released stream to a symbol that is still subscribed.
    ///
    /// Called when the chart symbol is unsubscribed: the server keeps streaming it
    /// until the chart moves, so anything else we are still serving would starve.
    async fn repoint_released_chart(&mut self, released: &str) {
        if self.chart_symbol.as_deref() != Some(released) {
            return;
        }
        let next = {
            let subscriptions = self.state.active_subscriptions.read().await;
            subscriptions.keys().next().cloned()
        };
        match next {
            Some(asset) => {
                let period = self
                    .state
                    .active_subscriptions
                    .read()
                    .await
                    .get(&asset)
                    .and_then(|entries| {
                        entries
                            .iter()
                            .find_map(|(_, sub_type, _)| sub_type.period_secs())
                    })
                    .unwrap_or(1);
                if let Err(e) = self.point_chart_at(&asset, period).await {
                    warn!(target: "SubscriptionsApiModule", "Failed to move the chart stream to {asset}: {}", e);
                }
            }
            None => self.chart_symbol = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binary_options_tools_core::reimports::bounded_async;

    /// Builds a module whose only observable side effect is what it sends on the wire.
    async fn test_module() -> (SubscriptionsApiModule, AsyncReceiver<Message>) {
        let ssid = crate::pocketoption::ssid::Ssid::parse(
            r#"42["auth",{"session":"dummy","isDemo":1,"uid":1,"platform":2}]"#,
        )
        .unwrap();
        let state = Arc::new(
            crate::pocketoption::state::StateBuilder::default()
                .ssid(ssid)
                .build()
                .unwrap(),
        );
        let (cmd_tx, cmd_rx) = bounded_async::<Command>(8);
        let (resp_tx, _resp_rx) = bounded_async::<CommandResponse>(8);
        let (_msg_tx, msg_rx) = bounded_async::<Arc<Message>>(8);
        let (ws_tx, ws_rx) = bounded_async::<Message>(32);
        let (runner_tx, _runner_rx) = bounded_async::<RunnerCommand>(4);
        let module = SubscriptionsApiModule::new(state, cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);
        drop(cmd_tx);
        (module, ws_rx)
    }

    fn time_subscription(secs: u64) -> SubscriptionType {
        SubscriptionType::Time {
            start_time: None,
            duration: Duration::from_secs(secs),
            candle: Default::default(),
        }
    }

    #[tokio::test]
    async fn pointing_the_chart_sends_change_symbol_once() {
        let (mut module, ws_rx) = test_module().await;

        module.point_chart_at("EURUSD_otc", 5).await.unwrap();
        module.point_chart_at("EURUSD_otc", 5).await.unwrap();
        module.point_chart_at("GBPUSD_otc", 5).await.unwrap();

        // Re-pointing at the symbol already streaming must not resend: every
        // redundant `changeSymbol` interrupts the stream the server is sending.
        assert_eq!(
            ws_rx.recv().await.unwrap(),
            Message::text(r#"42["changeSymbol",{"asset":"EURUSD_otc","period":5}]"#)
        );
        assert_eq!(
            ws_rx.recv().await.unwrap(),
            Message::text(r#"42["changeSymbol",{"asset":"GBPUSD_otc","period":5}]"#)
        );
        // `Ok(None)` is kanal's "nothing queued".
        assert!(
            matches!(ws_rx.try_recv(), Ok(None)),
            "only real chart moves may be sent"
        );
    }

    #[tokio::test]
    async fn releasing_the_chart_symbol_hands_the_stream_to_a_live_subscription() {
        let (mut module, ws_rx) = test_module().await;

        // Two symbols are subscribed; the server only streams the chart symbol.
        let (eur_tx, _eur_rx) = bounded_async::<SubscriptionEvent>(4);
        let (gbp_tx, _gbp_rx) = bounded_async::<SubscriptionEvent>(4);
        module
            .add_subscription(
                "EURUSD_otc".to_string(),
                time_subscription(5),
                eur_tx,
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        module
            .add_subscription(
                "GBPUSD_otc".to_string(),
                time_subscription(60),
                gbp_tx,
                Uuid::new_v4(),
            )
            .await
            .unwrap();

        module.point_chart_at("EURUSD_otc", 5).await.unwrap();
        assert_eq!(
            ws_rx.recv().await.unwrap(),
            Message::text(r#"42["changeSymbol",{"asset":"EURUSD_otc","period":5}]"#)
        );

        // Dropping the chart symbol must move the stream instead of leaving it
        // parked on a symbol nobody consumes: the server ignores `unsubscribeSymbol`.
        module
            .remove_subscription("EURUSD_otc", None)
            .await
            .unwrap();
        module.repoint_released_chart("EURUSD_otc").await;

        match ws_rx.recv().await.unwrap() {
            Message::Text(text) => {
                assert!(
                    text.contains(r#""asset":"GBPUSD_otc""#),
                    "the stream should move to the remaining subscription, got {text}"
                );
                assert!(
                    text.contains(r#""period":60"#),
                    "the remaining subscription's period should be used, got {text}"
                );
            }
            other => panic!("expected a changeSymbol frame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn releasing_a_non_chart_symbol_leaves_the_stream_alone() {
        let (mut module, ws_rx) = test_module().await;
        module.point_chart_at("EURUSD_otc", 5).await.unwrap();
        ws_rx.recv().await.unwrap();

        module.repoint_released_chart("GBPUSD_otc").await;
        assert!(
            matches!(ws_rx.try_recv(), Ok(None)),
            "releasing a symbol that is not the chart must not move the chart"
        );
    }

    #[tokio::test]
    async fn dropping_armed_history_guard_cancels_pending_request() {
        let (sender, receiver) = bounded_async::<Command>(8);
        let request_id = Uuid::new_v4();
        {
            let _guard = HistoryRequestGuard {
                sender: sender.clone(),
                asset: "EURUSD_otc".to_string(),
                period: 60,
                command_id: request_id,
                armed: true,
            };
        }
        match receiver.recv().await.unwrap() {
            Command::CancelHistory {
                asset,
                period,
                command_id,
            } => {
                assert_eq!(asset, "EURUSD_otc");
                assert_eq!(period, 60);
                assert_eq!(command_id, request_id);
            }
            other => panic!("expected CancelHistory, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn full_channel_still_delivers_cancellation() {
        // A cancellation must not be lost just because the command queue is full.
        let (sender, receiver) = bounded_async::<Command>(1);
        let request_id = Uuid::new_v4();
        sender
            .send(Command::SubscriptionCount {
                command_id: Uuid::new_v4(),
            })
            .await
            .unwrap();
        {
            let _guard = HistoryRequestGuard {
                sender: sender.clone(),
                asset: "EURUSD_otc".to_string(),
                period: 60,
                command_id: request_id,
                armed: true,
            };
        }
        // First message fills the slot; the cancellation is retried from a task.
        assert!(matches!(
            receiver.recv().await.unwrap(),
            Command::SubscriptionCount { .. }
        ));
        match receiver.recv().await.unwrap() {
            Command::CancelHistory { command_id, .. } => assert_eq!(command_id, request_id),
            other => panic!("expected CancelHistory, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn disarmed_history_guard_sends_nothing() {
        let (sender, receiver) = bounded_async::<Command>(8);
        {
            let _guard = HistoryRequestGuard {
                sender: sender.clone(),
                asset: "EURUSD_otc".to_string(),
                period: 60,
                command_id: Uuid::new_v4(),
                armed: false,
            };
        }
        drop(sender);
        assert!(receiver.recv().await.is_err());
    }
}
