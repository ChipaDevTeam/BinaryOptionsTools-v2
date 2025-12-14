use async_trait::async_trait;
use binary_options_tools_core_pre::error::CoreError;
use binary_options_tools_core_pre::reimports::bounded_async;
use binary_options_tools_core_pre::traits::ReconnectCallback;
use binary_options_tools_core_pre::{
    error::CoreResult,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use core::fmt;
use futures_util::stream::unfold;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tokio::select;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::pocketoption::candle::{BaseCandle, SubscriptionType};
use crate::pocketoption::error::PocketError;
use crate::pocketoption::types::{MultiPatternRule, StreamData as RawCandle};
use crate::pocketoption::{
    candle::Candle, // Assuming this exists in your types
    error::PocketResult,
    state::State,
};

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
    pub candles: Vec<BaseCandle>,
    pub history: Vec<Vec<f64>>,
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
    Subscribe { asset: String, command_id: Uuid },
    /// Unsubscribe from an asset's stream
    Unsubscribe { asset: String, command_id: Uuid },
    /// History
    History {
        asset: String,
        period: u32,
        command_id: Uuid,
    },
    /// Requests the number of active subscriptions
    SubscriptionCount,
}

/// Response enum for subscription commands
#[derive(Debug)]
pub enum CommandResponse {
    /// Successful subscription with stream receiver
    SubscriptionSuccess {
        command_id: Uuid,
        stream_receiver: AsyncReceiver<StreamData>,
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
    SubscriptionCount(u32),
}

/// Represents the data sent through the subscription stream.
pub struct SubscriptionStream {
    receiver: AsyncReceiver<StreamData>,
    sender: AsyncSender<Command>,
    asset: String,
    sub_type: SubscriptionType,
}

/// Data sent through the subscription stream
#[derive(Debug, Clone)]
pub enum StreamData {
    /// New candle data
    Update {
        asset: String,
        price: f64,
        timestamp: f64,
    },
    /// Subscription terminated (stream should end)
    Terminated { reason: String },
    /// Unsubscribe signal (stream should end gracefully)
    Unsubscribe,
}

/// Callback for when there is a disconnection
struct SubscriptionCallback {
    /// Active subscriptions mapped by subscription symbol
    active_subscriptions: Arc<RwLock<HashMap<String, AsyncSender<StreamData>>>>,
}

/// Handle for interacting with the `SubscriptionsApiModule`.
#[derive(Clone)]
pub struct SubscriptionsHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
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
        // TODO: Implement subscription logic
        // 1. Generate subscription ID
        // 2. Send Command::Subscribe
        // 3. Wait for CommandResponse::SubscriptionSuccess
        // 4. Return subscription ID and stream receiver
        let id = Uuid::new_v4();
        self.sender
            .send(Command::Subscribe {
                asset: asset.clone(),
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the subscription response

        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::SubscriptionSuccess {
                    command_id,
                    stream_receiver,
                }) => {
                    if command_id == id {
                        return Ok(SubscriptionStream {
                            receiver: stream_receiver,
                            sender: self.sender.clone(),
                            asset,
                            sub_type,
                        });
                    } else {
                        // If the request ID does not match, continue waiting for the correct response
                        continue;
                    }
                }
                Ok(CommandResponse::SubscriptionFailed { command_id, error }) => {
                    if command_id == id {
                        return Err(*error);
                    }
                    continue;
                }
                Ok(_) => continue,
                Err(e) => return Err(CoreError::from(e).into()),
            }
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
        // TODO: Implement unsubscription logic
        // 1. Send Command::Unsubscribe
        // 2. Wait for CommandResponse::UnsubscriptionSuccess
        let id = Uuid::new_v4();
        self.sender
            .send(Command::Unsubscribe {
                asset,
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the unsubscription response
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::UnsubscriptionSuccess { command_id }) => {
                    if command_id == id {
                        return Ok(());
                    } else {
                        // If the request ID does not match, continue waiting for the correct response
                        continue;
                    }
                }
                Ok(CommandResponse::UnsubscriptionFailed { command_id, error }) => {
                    if command_id == id {
                        return Err(*error);
                    }
                    continue;
                }
                Ok(_) => continue,
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }

    /// Get the number of active subscriptions.
    ///
    /// # Returns
    /// * `PocketResult<usize>` - Number of active subscriptions
    pub async fn get_active_subscriptions_count(&self) -> PocketResult<u32> {
        self.sender
            .send(Command::SubscriptionCount)
            .await
            .map_err(CoreError::from)?;
        // Wait for the subscription count response
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::SubscriptionCount(count)) => {
                    return Ok(count);
                }
                Ok(_) => continue,
                Err(e) => return Err(CoreError::from(e).into()),
            }
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
    /// # Arguments
    /// * `asset` - The asset symbol
    /// * `period` - The period in minutes
    /// # Returns
    /// * `PocketResult<Vec<Candle>>` - Vector of candles
    pub async fn history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let id = Uuid::new_v4();
        self.sender
            .send(Command::History {
                asset,
                period,
                command_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        // Wait for the history response
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::History { command_id, data }) => {
                    if command_id == id {
                        return Ok(data);
                    } else {
                        // If the request ID does not match, continue waiting for the correct response
                        continue;
                    }
                }
                Ok(_) => continue,
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }
}

/// The API module for handling subscription operations.
pub struct SubscriptionsApiModule {
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,

    /// Active subscriptions mapped by subscription symbol
    active_subscriptions: Arc<RwLock<HashMap<String, AsyncSender<StreamData>>>>,
    histories: Arc<RwLock<Vec<(String, u32, Uuid)>>>,
}

#[async_trait]
impl ReconnectCallback<State> for SubscriptionCallback {
    async fn call(&self, state: Arc<State>, ws_sender: &AsyncSender<Message>) -> CoreResult<()> {
        tokio::time::sleep(Duration::from_secs(state.reconnection_delay)).await;
        // Resubscribe to all active subscriptions
        for symbol in self.active_subscriptions.read().await.keys() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            // Resubscribe to each active subscription
            send_subscribe_message(ws_sender, symbol, 1).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ApiModule<State> for SubscriptionsApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = SubscriptionsHandle;

    fn new(
        _: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
    ) -> Self {
        Self {
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
            active_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            histories: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        SubscriptionsHandle { sender, receiver }
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
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::Subscribe { asset, command_id } => {
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
                                self.send_subscribe_message(&asset, 1).await?;
                                let (stream_sender, stream_receiver) = bounded_async(MAX_CHANNEL_CAPACITY);
                                self.add_subscription(asset.clone(), stream_sender).await.map_err(|e| CoreError::Other(e.to_string()))?;


                                // Send success response with stream receiver
                                self.command_responder.send(CommandResponse::SubscriptionSuccess {
                                    command_id,
                                    stream_receiver,
                                }).await?;
                            }
                        },
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
                        Command::SubscriptionCount => {
                            let count = self.active_subscriptions.read().await.len() as u32;
                            self.command_responder.send(CommandResponse::SubscriptionCount(count)).await?;
                        },
                        Command::History { asset, period, command_id } => {
                            self.send_subscribe_message(&asset, period).await?;
                            self.histories.write().await.push((asset, period, command_id));
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    // TODO: Handle incoming WebSocket messages
                    // 1. Parse message for asset data
                    // 2. Find corresponding subscription
                    // 3. Forward data to stream
                    // 4. Handle subscription confirmations/errors
                    match msg.as_ref() {
                        Message::Binary(data) => {
                            // Parse the message for asset data
                            match serde_json::from_slice::<ServerResponse>(data) {
                                Ok(ServerResponse::Candle(data)) => {
                                    // Forward data to stream
                                    if let Err(e) = self.forward_data_to_stream(&data.symbol, data.price, data.timestamp).await {
                                        warn!(target: "SubscriptionsApiModule", "Failed to forward data: {}", e);
                                    }
                                },
                                Ok(ServerResponse::History(data)) => {
                                    let mut id = None;
                                    self.histories.write().await.retain(|(asset, period, c_id)| {
                                        if asset == &data.asset && *period == data.period {
                                            id = Some(*c_id);
                                            false
                                        } else {
                                            true
                                        }
                                    });
                                    if let Some(command_id) = id {
                                        let candles = data.candles.into_iter().map(|c| Candle::try_from((c, data.asset.clone()))).collect::<Result<Vec<_>, _>>().map_err(|e| CoreError::Other(e.to_string()))?;
                                        if let Err(e) = self.command_responder.send(CommandResponse::History {
                                            command_id,
                                            data: candles
                                        }).await {
                                            warn!(target: "SubscriptionsApiModule", "Failed to send history response: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(target: "SubscriptionsApiModule", "Received data: {:?}",  String::from_utf8(data.to_vec()));
                                    warn!(target: "SubscriptionsApiModule", "Failed to parse message: {}", e);
                                }
                            }
                        },
                        _ => {
                            warn!(target: "SubscriptionsApiModule", "Received unsupported message type");
                            debug!(target: "SubscriptionsApiModule", "Message: {:?}", msg);
                        }
                    }
                }
            }
        }
    }

    fn callback(&self) -> CoreResult<Option<Box<dyn ReconnectCallback<State>>>> {
        // Default implementation does nothing.
        // This is useful for modules that do not require a callback.

        Ok(Some(Box::new(SubscriptionCallback {
            active_subscriptions: self.active_subscriptions.clone(),
        })))
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        // TODO: Implement rule for subscription-related messages
        // This should match messages like:
        // - Asset data updates
        // - Subscription confirmations
        // - Subscription errors
        Box::new(MultiPatternRule::new(vec![
            r#"451-["updateStream",{"#,
            r#"451-["updateHistoryNewFast","#,
        ]))
    }
}

impl SubscriptionsApiModule {
    /// Check if maximum subscriptions limit is reached.
    ///
    /// # Returns
    /// * `bool` - True if limit reached
    async fn is_max_subscriptions_reached(&self) -> bool {
        self.active_subscriptions.read().await.len() >= MAX_SUBSCRIPTIONS
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
        stream_sender: AsyncSender<StreamData>,
    ) -> PocketResult<()> {
        if self.is_max_subscriptions_reached().await {
            return Err(SubscriptionError::MaxSubscriptionsReached.into());
        }

        // Check if subscription already exists
        if self.active_subscriptions.read().await.contains_key(&asset) {
            return Err(SubscriptionError::SubscriptionAlreadyExists.into());
        }

        // Add to active subscriptions
        self.active_subscriptions
            .write()
            .await
            .insert(asset, stream_sender);
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
        if let Some(stream_sender) = self.active_subscriptions.write().await.remove(asset) {
            stream_sender.send(StreamData::Terminated { reason: "Unsubscribed from main module".to_string() })
                .await.inspect_err(|e| warn!(target: "SubscriptionsApiModule", "Failed to send termination signal: {}", e))?;
            return Ok(true);
        }
        self.resend_connection_messages().await?;
        Ok(false)
    }

    async fn resend_connection_messages(&self) -> CoreResult<()> {
        // Resend connection messages to re-establish subscriptions
        for symbol in self.active_subscriptions.read().await.keys() {
            // Send subscription message for each active asset
            self.send_subscribe_message(symbol, 1).await?;
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
        price: f64,
        timestamp: f64,
    ) -> CoreResult<()> {
        // TODO: Implement data forwarding
        // 1. Find subscription by asset
        // 2. Send StreamData::Candle to stream
        // 3. Handle send errors (stream might be closed)
        if let Some(stream_sender) = self.active_subscriptions.read().await.get(asset) {
            stream_sender
                .send(StreamData::Update {
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
    pub async fn unsubscribe(self) -> PocketResult<()> {
        // Send unsubscribe command through the main handle
        let command_id = Uuid::new_v4();
        self.sender
            .send(Command::Unsubscribe {
                asset: self.asset.clone(),
                command_id,
            })
            .await
            .map_err(CoreError::from)?;

        // We don't need to wait for response since we're consuming self
        Ok(())
    }

    /// Receive the next candle from the stream
    pub async fn receive(&mut self) -> PocketResult<Candle> {
        loop {
            match self.receiver.recv().await {
                Ok(StreamData::Update {
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
                Ok(StreamData::Terminated { reason }) => {
                    return Err(PocketError::General(format!("Stream terminated: {reason}")));
                }
                Ok(StreamData::Unsubscribe) => {
                    return Err(PocketError::General("Stream unsubscribed".to_string()));
                }
                Err(e) => {
                    return Err(CoreError::from(e).into());
                }
            }
        }
    }

    /// Process an incoming price update based on subscription type
    fn process_update(&mut self, timestamp: f64, price: f64) -> PocketResult<Option<Candle>> {
        let asset = self.asset().to_string();
        if let Some(c) = self
            .sub_type
            .update(&BaseCandle::from((timestamp, price)))?
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
        let _ = self
            .sender
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
