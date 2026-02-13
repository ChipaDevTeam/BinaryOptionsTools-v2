use async_trait::async_trait;
use binary_options_tools_core_pre::error::CoreError;
use binary_options_tools_core_pre::reimports::bounded_async;
use binary_options_tools_core_pre::traits::ReconnectCallback;
use binary_options_tools_core_pre::{
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
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::oneshot;
use tokio::sync::Mutex as TokioMutex;

use tracing::{debug, warn};
use uuid::Uuid;

use crate::pocketoption::candle::{
    compile_candles_from_ticks, BaseCandle, HistoryItem, SubscriptionType,
};
use crate::pocketoption::error::PocketError;
use crate::pocketoption::types::{MultiPatternRule, StreamData as RawCandle, SubscriptionEvent};
use crate::pocketoption::{
    candle::Candle, // Assuming this exists in your types
    error::PocketResult,
    state::State,
};

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
                        let _ = tx.send(resp);
                    }
                }
            }
        });
        router
    }

    pub async fn wait_for(&self, id: Uuid) -> PocketResult<CommandResponse> {
        let rx = self.register(id).await;
        rx.await
            .map_err(|_| PocketError::General("Response router channel closed".into()))
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
    Candle(RawCandle),
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

/// Maximum number of concurrent subscriptions allowed
const MAX_SUBSCRIPTIONS: usize = 4;
const MAX_CHANNEL_CAPACITY: usize = 64;
const RECONNECT_INITIAL_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("Maximum subscriptions limit reached")]
    MaxSubscriptionsReached,
    #[error("Subscription already exists")]
    SubscriptionAlreadyExists,
}

/// Command enum for the `SubscriptionsApiModule`.
#[derive(Debug)]
pub enum Command {
    /// Subscribe to an asset's stream
    Subscribe {
        asset: String,
        sub_type: SubscriptionType,
        command_id: Uuid,
    },
    /// Unsubscribe from an asset's stream
    Unsubscribe { asset: String, command_id: Uuid },
    /// History
    History {
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
    /// Returns the number of active subscriptions
    SubscriptionCount { command_id: Uuid, count: u32 },
    /// History failed
    HistoryFailed {
        command_id: Uuid,
        error: Box<PocketError>,
    },
}

/// Represents the data sent through the subscription stream.
pub struct SubscriptionStream {
    receiver: AsyncReceiver<SubscriptionEvent>,
    sender: Option<AsyncSender<Command>>,
    router: Arc<ResponseRouter>,
    asset: String,
    sub_type: SubscriptionType,
}

/// Callback for when there is a disconnection
struct SubscriptionCallback;

#[async_trait]
impl ReconnectCallback<State> for SubscriptionCallback {
    async fn call(&self, state: Arc<State>, ws_sender: &AsyncSender<Message>) -> CoreResult<()> {
        tokio::time::sleep(RECONNECT_INITIAL_DELAY).await;
        // Resubscribe to all active subscriptions
        let subscriptions = state.active_subscriptions.read().await.clone();

        // Send subscription messages concurrently
        let futures = subscriptions.into_iter().map(|(symbol, (_, sub_type))| {
            let ws_sender = ws_sender.clone();
            let period = sub_type.period_secs().unwrap_or(1);
            async move { send_subscribe_message(&ws_sender, &symbol, period).await }
        });

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
        // Wait for the subscription response

        match receiver
            .await
            .map_err(|_| PocketError::General("Response router channel closed".into()))?
        {
            CommandResponse::SubscriptionSuccess {
                command_id: _,
                stream_receiver,
            } => Ok(SubscriptionStream {
                receiver: stream_receiver,
                sender: Some(self.sender.clone()),
                router: self.router.clone(),
                asset,
                sub_type,
            }),
            CommandResponse::SubscriptionFailed { error, .. } => Err(*error),
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
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the unsubscription response
        match receiver
            .await
            .map_err(|_| PocketError::General("Response router channel closed".into()))?
        {
            CommandResponse::UnsubscriptionSuccess { .. } => Ok(()),
            CommandResponse::UnsubscriptionFailed { error, .. } => Err(*error),
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
        // Wait for the subscription count response
        match receiver
            .await
            .map_err(|_| PocketError::General("Response router channel closed".into()))?
        {
            CommandResponse::SubscriptionCount { count, .. } => Ok(count),
            _ => Err(PocketError::General(
                "Unexpected response to subscription count command".into(),
            )),
        }
    }

    /// Check if maximum subscriptions limit is reached.
    ///
    /// # Returns
    /// * `PocketResult<bool>` - True if limit reached
    pub async fn is_max_subscriptions_reached(&self) -> PocketResult<bool> {
        self.get_active_subscriptions_count()
            .await
            .map(|count| count as usize == MAX_SUBSCRIPTIONS)
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
                asset,
                period,
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the history response
        match receiver
            .await
            .map_err(|_| PocketError::General("Response router channel closed".into()))?
        {
            CommandResponse::History { data, .. } => Ok(data),
            CommandResponse::HistoryFailed { error, .. } => Err(*error),
            _ => Err(PocketError::General(
                "Unexpected response to history command".into(),
            )),
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
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        SubscriptionsHandle {
            sender,
            router: ResponseRouter::new(receiver),
        }
    }

    async fn run(&mut self) -> CoreResult<()> {
        // TODO: Implement the main run loop
        // This loop should handle:
        // 1. Incoming commands (Subscribe, Unsubscribe, StreamTerminationRequest)
        // 2. Incoming WebSocket messages with asset data
        // 3. Managing subscription limits
        // 4. Forwarding data to appropriate streams
        //
        loop {
            select! {
                cmd_res = self.command_receiver.recv() => {
                    let cmd = match cmd_res {
                        Ok(cmd) => cmd,
                        Err(_) => return Ok(()), // Channel closed
                    };
                    match cmd {
                        Command::Subscribe {
                            asset,
                            sub_type,
                            command_id,
                        } => {
                            // TODO: Handle subscription request
                            // 1. Check if max subscriptions reached
                            // 2. Create stream channel
                            // 3. Send WebSocket subscription message
                            // 4. Store subscription info
                            // 5. Send success response with stream receiver

                            if self.is_max_subscriptions_reached().await {
                                self.command_responder.send(CommandResponse::SubscriptionFailed {
                                    command_id,
                                    error: Box::new(SubscriptionError::MaxSubscriptionsReached.into()),
                                }).await?;
                                continue;
                            } else {
                                // Create stream channel
                                let period = sub_type.period_secs().unwrap_or(1);
                                self.send_subscribe_message(&asset, period).await?;
                                let (stream_sender, stream_receiver) =
                                    bounded_async(MAX_CHANNEL_CAPACITY);
                                self.add_subscription(asset.clone(), sub_type, stream_sender)
                                    .await
                                    .map_err(|e| CoreError::Other(e.to_string()))?;

                                // Send success response with stream receiver
                                self.command_responder.send(CommandResponse::SubscriptionSuccess {
                                    command_id,
                                    stream_receiver,
                                }).await?;
                            }
                        }
                        Command::Unsubscribe { asset, command_id } => {
                            // TODO: Handle unsubscription request
                            // 1. Find subscription by ID
                            // 2. Send unsubscribe message to WebSocket
                            // 3. Send Unsubscribe signal to stream
                            // 4. Remove from active subscriptions
                            // 5. Send success response
                            match self.remove_subscription(&asset).await {
                                Ok(b) => {
                                    // Send Unsubscribe signal to stream
                                    if b {
                                        self.command_responder.send(CommandResponse::UnsubscriptionSuccess { command_id }).await?;
                                    } else {
                                        // Subscription not found, send failure response
                                        self.command_responder.send(CommandResponse::UnsubscriptionFailed {
                                            command_id,
                                            error: Box::new(PocketError::General("Subscription not found".to_string())),
                                        }).await?;
                                    }
                                },
                                Err(e) => {
                                    // Subscription not found, send failure response
                                    self.command_responder.send(CommandResponse::UnsubscriptionFailed {
                                        command_id,
                                        error: Box::new(e.into()),
                                    }).await?;
                                }
                            }
                        },
                        Command::SubscriptionCount { command_id } => {
                            let count = self.state.active_subscriptions.read().await.len() as u32;
                            self.command_responder.send(CommandResponse::SubscriptionCount { command_id, count }).await?;
                        },
                        Command::History { asset, period, command_id } => {
                            // Enforce single request
                            let is_duplicate = self.state.histories.read().await.iter().any(|(a, p, _)| a == &asset && *p == period);
                            if is_duplicate {
                                if let Err(e) = self.command_responder.send(CommandResponse::HistoryFailed {
                                    command_id,
                                    error: Box::new(PocketError::General(format!("Duplicate history request for asset: {}, period: {}", asset, period))),
                                }).await {
                                     warn!(target: "SubscriptionsApiModule", "Failed to send history failed response: {}", e);
                                }
                                                                } else if let Err(e) = self.send_subscribe_message(&asset, period).await {
                                                                    if let Err(e2) = self.command_responder.send(CommandResponse::HistoryFailed {
                                                                        command_id,
                                                                        error: Box::new(e.into()),
                                                                    }).await {
                                                                        warn!(target: "SubscriptionsApiModule", "Failed to send history failed response: {}", e2);
                                                                    }
                                                                } else {
                                                                    self.state.histories.write().await.push((asset, period, command_id));
                                                                }                        }
                    }
                },
                msg_res = self.message_receiver.recv() => {
                    let msg = match msg_res {
                        Ok(msg) => msg,
                        Err(_) => return Ok(()), // Channel closed
                    };
                    let response = match msg.as_ref() {
                        Message::Binary(data) => serde_json::from_slice::<ServerResponse>(data).ok(),
                        Message::Text(text) => serde_json::from_str::<ServerResponse>(text).ok(),
                        _ => None,
                    };

                    if let Some(response) = response {
                        match response {
                            ServerResponse::Candle(data) => {
                                // Forward data to stream
                                if let Err(e) = self.forward_data_to_stream(&data.symbol, data.price, data.timestamp).await {
                                    warn!(target: "SubscriptionsApiModule", "Failed to forward data: {}", e);
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
                                                warn!(target: "SubscriptionsApiModule", "Failed to send history response: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(e) = self.command_responder.send(CommandResponse::HistoryFailed {
                                                command_id,
                                                error: Box::new(e)
                                            }).await {
                                                warn!(target: "SubscriptionsApiModule", "Failed to send history failed response: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        debug!(target: "SubscriptionsApiModule", "Received message that didn't match ServerResponse: {:?}", msg);
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
        // TODO: Implement rule for subscription-related messages
        // This should match messages like:
        // - Asset data updates
        // - Subscription confirmations
        // - Subscription errors
        Box::new(MultiPatternRule::new(vec![
            "updateStream",
            "updateHistoryNewFast",
            "updateHistoryNew",
        ]))
    }
}

impl SubscriptionsApiModule {
    /// Check if maximum subscriptions limit is reached.
    ///
    /// # Returns
    /// * `bool` - True if limit reached
    async fn is_max_subscriptions_reached(&self) -> bool {
        self.state.active_subscriptions.read().await.len() >= MAX_SUBSCRIPTIONS
    }

    /// Add a new subscription.
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID
    /// * `asset` - The asset symbol
    /// * `stream_sender` - The sender for stream data
    ///
    /// # Returns
    /// * `Result<(), String>` - Success or error message
    async fn add_subscription(
        &mut self,
        asset: String,
        sub_type: SubscriptionType,
        stream_sender: AsyncSender<SubscriptionEvent>,
    ) -> PocketResult<()> {
        if self.is_max_subscriptions_reached().await {
            return Err(SubscriptionError::MaxSubscriptionsReached.into());
        }

        // Check if subscription already exists
        if self
            .state
            .active_subscriptions
            .read()
            .await
            .contains_key(&asset)
        {
            return Err(SubscriptionError::SubscriptionAlreadyExists.into());
        }

        // Add to active subscriptions
        self.state
            .active_subscriptions
            .write()
            .await
            .insert(asset, (stream_sender, sub_type));
        Ok(())
    }

    /// Remove a subscription.
    ///
    /// # Arguments
    /// * `asset` - The asset symbol
    ///
    /// # Returns
    /// * `PocketResult<bool>` - True if subscription was removed, false if not found
    async fn remove_subscription(&mut self, asset: &str) -> CoreResult<bool> {
        // TODO: Implement subscription removal
        // 1. Remove from active_subscriptions
        // 2. Remove from asset_to_subscription
        // 3. Return removed subscription info
        if let Some((stream_sender, _)) =
            self.state.active_subscriptions.write().await.remove(asset)
        {
            stream_sender.send(SubscriptionEvent::Terminated { reason: "Unsubscribed from main module".to_string() })
                .await.inspect_err(|e| warn!(target: "SubscriptionsApiModule", "Failed to send termination signal: {}", e))?;
            return Ok(true);
        }
        self.resend_connection_messages().await?;
        Ok(false)
    }

    async fn resend_connection_messages(&self) -> CoreResult<()> {
        // Resend connection messages to re-establish subscriptions
        let subscriptions = self.state.active_subscriptions.read().await.clone();
        for (symbol, (_, sub_type)) in subscriptions {
            let period = sub_type.period_secs().unwrap_or(1);
            // Send subscription message for each active asset
            self.send_subscribe_message(&symbol, period).await?;
        }
        Ok(())
    }

    /// Send subscription message to WebSocket.
    ///
    /// # Arguments
    /// * `asset` - The asset to subscribe to
    async fn send_subscribe_message(&self, asset: &str, period: u32) -> CoreResult<()> {
        // TODO: Implement WebSocket subscription message
        // Create and send appropriate subscription message format
        send_subscribe_message(&self.to_ws_sender, asset, period).await
    }
    /// Process incoming asset data and forward to appropriate streams.
    ///
    /// # Arguments
    /// * `asset` - The asset symbol
    /// * `candle` - The candle data
    async fn forward_data_to_stream(
        &self,
        asset: &str,
        price: Decimal,
        timestamp: i64,
    ) -> CoreResult<()> {
        // TODO: Implement data forwarding
        // 1. Find subscription by asset
        // 2. Send StreamData::Candle to stream
        // 3. Handle send errors (stream might be closed)
        if let Some((stream_sender, _)) = self.state.active_subscriptions.read().await.get(asset) {
            stream_sender
                .send(SubscriptionEvent::Update {
                    asset: asset.to_string(),
                    price,
                    timestamp,
                })
                .await
                .map_err(CoreError::from)?;
        }
        // If no subscription found for assets it's not an error, just ignore it
        Ok(())
    }
}

impl SubscriptionStream {
    /// Get the asset symbol for this subscription stream
    pub fn asset(&self) -> &str {
        &self.asset
    }

    /// Unsubscribe from the stream
    pub async fn unsubscribe(mut self) -> PocketResult<()> {
        // Send unsubscribe command through the main handle
        let command_id = Uuid::new_v4();
        let receiver = self.router.register(command_id).await;
        if let Some(sender) = self.sender.take() {
            sender
                .send(Command::Unsubscribe {
                    asset: self.asset.clone(),
                    command_id,
                })
                .await
                .map_err(CoreError::from)?;
        } else {
            return Ok(());
        }

        // Wait for response
        match receiver
            .await
            .map_err(|_| PocketError::General("Response router channel closed".into()))?
        {
            CommandResponse::UnsubscriptionSuccess { .. } => Ok(()),
            CommandResponse::UnsubscriptionFailed { error, .. } => Err(*error),
            _ => Err(PocketError::General(
                "Unexpected response to unsubscribe command".into(),
            )),
        }
    }

    /// Receive the next candle from the stream
    pub async fn receive(&mut self) -> PocketResult<Candle> {
        loop {
            match self.receiver.recv().await {
                Ok(crate::pocketoption::types::SubscriptionEvent::Update {
                    asset,
                    price,
                    timestamp,
                }) => {
                    if asset == self.asset {
                        let candle = self.process_update(timestamp, price)?;
                        if let Some(candle) = candle {
                            return Ok(candle);
                        }
                        // Continue if no candle is ready yet
                    }
                    // Continue if asset doesn't match (shouldn't happen but safety check)
                }
                Ok(crate::pocketoption::types::SubscriptionEvent::Terminated { reason }) => {
                    return Err(PocketError::General(format!("Stream terminated: {reason}")));
                }
                Err(e) => {
                    return Err(CoreError::from(e).into());
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
            // Successfully updated candle
            Ok(Some(Candle::try_from((c, asset)).map_err(|e| {
                warn!(target: "SubscriptionStream", "Failed to convert candle: {}", e);
                PocketError::General(format!("Failed to convert candle: {e}"))
            })?))
        } else {
            // No complete candle yet, continue waiting
            Ok(None)
        }
    }

    /// Convert to a futures Stream
    pub fn to_stream(self) -> impl futures_util::Stream<Item = PocketResult<Candle>> + 'static {
        Box::pin(unfold(self, |mut stream| async move {
            let result = stream.receive().await;
            Some((result, stream))
        }))
    }

    // /// Convert to a futures Stream with a static lifetime using Arc
    // pub fn to_stream_static(
    //     self
    // ) -> impl futures_util::Stream<Item = PocketResult<Candle>> + 'static {
    //     Box::pin(unfold(self, |mut stream| async move {
    //         let result = stream.receive().await;
    //         Some((result, stream))
    //     }))
    // }

    /// Check if the subscription type uses time alignment
    pub fn is_time_aligned(&self) -> bool {
        matches!(self.sub_type, SubscriptionType::TimeAligned { .. })
    }

    /// Get the current subscription type
    pub fn subscription_type(&self) -> &SubscriptionType {
        &self.sub_type
    }
}

// Add Clone implementation for SubscriptionStream
impl Clone for SubscriptionStream {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            sender: self.sender.clone(),
            router: self.router.clone(),
            asset: self.asset.clone(),
            sub_type: self.sub_type.clone(),
        }
    }
}

async fn send_subscribe_message(
    ws_sender: &AsyncSender<Message>,
    asset: &str,
    period: u32,
) -> CoreResult<()> {
    // TODO: Implement WebSocket subscription message
    // Create and send appropriate subscription message format
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
        .send(Message::text(format!("42[\"unsubfor\",\"{asset}\"]")))
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
        // Send Unsubscribe signal when the stream is dropped
        // This will gracefully end the stream and notify any listeners
        debug!(target: "SubscriptionStream", "Dropping subscription stream for asset: {}", self.asset);
        // Send Unsubscribe signal to the main handle
        // This will notify the main module to remove this subscription
        // We don't need to wait for response since we're consuming self
        // and it will be dropped anyway
        if let Some(sender) = &self.sender {
            let _ = sender
                .as_sync()
                .send(Command::Unsubscribe {
                    asset: self.asset.clone(),
                    command_id: Uuid::new_v4(),
                })
                .inspect_err(|e| {
                    warn!(target: "SubscriptionStream", "Failed to send unsubscribe command: {}", e);
                });
        }
    }
}
