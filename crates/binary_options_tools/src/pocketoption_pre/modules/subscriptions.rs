use core::fmt;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core_pre::error::CoreError;
use binary_options_tools_core_pre::reimports::bounded_async;
use binary_options_tools_core_pre::traits::ReconnectCallback;
use binary_options_tools_core_pre::{
    error::CoreResult,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::select;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::pocketoption_pre::error::PocketError;
use crate::pocketoption_pre::types::{StreamData as RawCandle, TwoStepRule};
use crate::pocketoption_pre::{
    error::PocketResult,
    state::State,
    types::Candle, // Assuming this exists in your types
};

#[derive(Serialize)]
pub struct ChangeSymbol {
    // Making it public as it may be used somewhere else
    pub asset: String,
    pub period: i64,
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

pub enum SubscriptionType {
    None,
    Chunk {
        size: usize,    // Number of candles to aggregate
        current: usize, // Current aggregated candle count
        candle: Option<Candle>, // Current aggregated candle
    },
    Time {
        start_time: Option<f64>,
        duration: Duration,
        candle: Option<Candle>,
    },
    TimeAligned {
        duration: Duration, // No need for start_time
        candle: Option<Candle>,
    },
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
}

/// The API module for handling subscription operations.
pub struct SubscriptionsApiModule {
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,

    /// Active subscriptions mapped by subscription symbol
    active_subscriptions: Arc<RwLock<HashMap<String, AsyncSender<StreamData>>>>,
}

#[async_trait]
impl ReconnectCallback<State> for SubscriptionCallback {
    async fn call(&self, _: Arc<State>, ws_sender: &AsyncSender<Message>) -> CoreResult<()> {
        tokio::time::sleep(Duration::from_secs(2)).await; // FIXME: This is a temporary delay, it may need to be fine tuned
        // Resubscribe to all active subscriptions
        for symbol in self.active_subscriptions.read().await.keys() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            // Resubscribe to each active subscription
            send_subscribe_message(ws_sender, symbol).await?;
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
                                self.send_subscribe_message(&asset).await?;
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
                            match serde_json::from_slice::<RawCandle>(data) {
                                Ok(data) => {
                                    // Forward data to stream
                                    if let Err(e) = self.forward_data_to_stream(&data.symbol, data.price, data.timestamp).await {
                                        warn!(target: "SubscriptionsApiModule", "Failed to forward data: {}", e);
                                    }
                                },
                                Err(e) => {
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
        
        Ok(Some(Box::new(
            SubscriptionCallback { active_subscriptions: self.active_subscriptions.clone() }
        )))
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        // TODO: Implement rule for subscription-related messages
        // This should match messages like:
        // - Asset data updates
        // - Subscription confirmations
        // - Subscription errors
        Box::new(TwoStepRule::new(r#"451-["updateStream",{"#))
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
        self.active_subscriptions.write().await.insert(asset, stream_sender);
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
            self.send_subscribe_message(symbol).await?;
        }
        Ok(())
    }

    /// Send subscription message to WebSocket.
    ///
    /// # Arguments
    /// * `asset` - The asset to subscribe to
    async fn send_subscribe_message(&self, asset: &str) -> CoreResult<()> {
        // TODO: Implement WebSocket subscription message
        // Create and send appropriate subscription message format
        send_subscribe_message(&self.to_ws_sender, asset).await
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
                        let candle = self.process_update(price, timestamp)?;
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
    fn process_update(&mut self, price: f64, timestamp: f64) -> PocketResult<Option<Candle>> {
        let asset = self.asset().to_string();
        match &mut self.sub_type {
            SubscriptionType::None => {
                // Return immediately with a simple candle
                Ok(Some(Candle::new(asset.clone(), timestamp, price)))
            }

            SubscriptionType::Chunk {
                size,
                current,
                candle,
            } => {
                // Update the aggregated candle
                if *current == 0 {
                    *candle = Some(Candle::new(asset.clone(), timestamp, price))
                } else if let Some(c) = candle.as_mut() { c.update(timestamp, price) }

                *current += 1;

                if *current >= *size {
                    let result = candle.clone();
                    *current = 0; // Reset for next chunk
                    Ok(result)
                } else {
                    Ok(None)
                }
            }

            SubscriptionType::Time {
                start_time,
                duration,
                candle,
            } => {
                if start_time.is_none() {
                    *start_time = Some(timestamp);
                    *candle = Some(Candle::new(asset.clone(), timestamp, price));
                    return Ok(None);
                }

                // Update the aggregated candle
                if let Some(c) = candle.as_mut() { c.update(timestamp, price) }

                let elapsed = if let Some(c) = candle {
                    c.datetime()
                    .signed_duration_since(
                        DateTime::from_timestamp(start_time.unwrap() as i64, 0)
                            .unwrap_or_else(Utc::now),
                    )
                    .to_std()
                    .map_err(|_| PocketError::General("Time calculation error".to_string()))?
                } else {
                    Duration::ZERO
                };
                

                if elapsed >= *duration {
                    let result = candle.clone();
                    *start_time = None; // Reset for next period
                    Ok(result)
                } else {
                    Ok(None)
                }
            }

            SubscriptionType::TimeAligned { duration, candle } => {
                Self::process_time_aligned_update_static(
                    price,
                    timestamp,
                    duration,
                    candle,
                    &self.asset,
                )
            }
        }
    }

    /// Process time-aligned updates that align with PocketOption's candle intervals
    fn process_time_aligned_update_static(
        price: f64,
        timestamp: f64,
        duration: &Duration,
        candle: &mut Option<Candle>,
        asset: &str,
    ) -> PocketResult<Option<Candle>> {
        let duration_secs = duration.as_secs();

        // Calculate the aligned time boundaries
        let timestamp_secs = timestamp as i64;
        let aligned_start = Self::align_timestamp_to_interval(timestamp_secs, duration_secs);
        let aligned_end = aligned_start + duration_secs as i64;

        // Check if we need to initialize or if we've crossed into a new interval
        let timestamp = candle.as_ref().map(|c| c.timestamp).unwrap_or(0.0);
        let current_interval_start =
            Self::align_timestamp_to_interval(timestamp as i64, duration_secs);

        if timestamp == 0.0 || aligned_start != current_interval_start {
            // Starting a new interval
            *candle = Some(Candle::new(asset.to_string(), aligned_start as f64, price));

            // Check if we should immediately return (for intervals that have already passed)
            if timestamp_secs >= aligned_end {
                let result = candle.clone();
                return Ok(result);
            }

            return Ok(None);
        }

        // Update the current candle
        if let Some(c) = candle.as_mut() { c.update_price(price) }
        
        // Check if we've reached the end of the current interval
        if timestamp_secs >= aligned_end {
            let result = candle.clone();
            Ok(result)
        } else {
            Ok(None)
        }
    }

    /// Align a timestamp to the nearest interval boundary
    fn align_timestamp_to_interval(timestamp: i64, interval_secs: u64) -> i64 {
        let interval = interval_secs as i64;
        (timestamp / interval) * interval
    }

    /// Convert to a futures Stream
    pub fn to_stream(self) -> impl futures_util::Stream<Item = PocketResult<Candle>> + 'static {
        futures_util::stream::unfold(self, |mut stream| async move {
            let result = stream.receive().await;
            Some((result, stream))
        })
    }

    /// Convert to a futures Stream with a static lifetime using Arc
    pub fn to_stream_static(
        self: Arc<Self>,
    ) -> impl futures_util::Stream<Item = PocketResult<Candle>> + 'static {
        let stream = (*self).clone();
        futures_util::stream::unfold(stream, |mut stream| async move {
            let result = stream.receive().await;
            Some((result, stream))
        })
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

// Add Clone implementation for SubscriptionType
impl Clone for SubscriptionType {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Chunk {
                size,
                current,
                candle,
            } => Self::Chunk {
                size: *size,
                current: *current,
                candle: candle.clone(),
            },
            Self::Time {
                start_time,
                duration,
                candle,
            } => Self::Time {
                start_time: *start_time,
                duration: *duration,
                candle: candle.clone(),
            },
            Self::TimeAligned { duration, candle } => Self::TimeAligned {
                duration: *duration,
                candle: candle.clone(),
            },
        }
    }
}

// Helper function to validate duration against PocketOption's supported candle lengths
impl SubscriptionStream {
    /// Validate that a duration is supported by PocketOption
    const SUPPORTED_DURATIONS: &[u64] = &[
        5, 15, 30, 60, 120, 180, 300, 600, 900, 1800, 2700, 3600, 7200, 10800, 14400,
    ];
    pub fn validate_duration(duration: Duration) -> bool {

        let duration_secs = duration.as_secs();
        Self::SUPPORTED_DURATIONS.contains(&duration_secs)
    }

    /// Create a new time-aligned subscription stream with validation
    pub fn new_time_aligned(
        receiver: AsyncReceiver<StreamData>,
        sender: AsyncSender<Command>,
        asset: String,
        duration: Duration,
    ) -> PocketResult<Self> {
        if !Self::validate_duration(duration) {
            return Err(PocketError::General(format!(
                "Unsupported candle duration: {}s. Supported durations: 5s, 15s, 30s, 1m, 2m, 3m, 5m, 10m, 15m, 30m, 45m, 1h, 2h, 3h, 4h",
                duration.as_secs()
            )));
        }

        Ok(Self {
            receiver,
            sender,
            asset: asset.clone(),
            sub_type: SubscriptionType::TimeAligned {
                duration,
                candle: None,
            },
        })
    }
}

async fn send_subscribe_message(ws_sender: &AsyncSender<Message>, asset: &str) -> CoreResult<()> {
    // TODO: Implement WebSocket subscription message
    // Create and send appropriate subscription message format
    ws_sender
        .send(Message::text(
            ChangeSymbol {
                asset: asset.to_string(),
                period: 1,
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

impl SubscriptionType {
    pub fn none() -> Self {
        SubscriptionType::None
    }

    pub fn chunk(size: usize) -> Self {
        SubscriptionType::Chunk {
            size,
            current: 0,
            candle: None,
        }
    }

    pub fn time(duration: Duration) -> Self {
        SubscriptionType::Time {
            start_time: None,
            duration,
            candle: None,
        }
    }

    pub fn time_aligned(duration: Duration) -> PocketResult<Self> {
        if !SubscriptionStream::validate_duration(duration) {
            return Err(PocketError::General(format!(
                "Unsupported candle duration: {}s",
                duration.as_secs()
            )));
        }
        Ok(SubscriptionType::TimeAligned {
                    duration,
                    candle: None,
                })
    }
}