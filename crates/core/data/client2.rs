use std::{
    collections::HashMap, f32::consts::E, ops::Deref, sync::Arc, time::{Duration, Instant}
};

use async_channel::{Receiver, Sender, bounded};
use async_trait::async_trait;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time::{sleep, interval},
    select,
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::{
    constants::MAX_CHANNEL_CAPACITY,
    error::{BinaryOptionsResult, BinaryOptionsToolsError},
    general::{
        batching::{BatchingConfig, MessageBatcher, RateLimiter},
        config::Config,
        connection::{ConnectionManager, ConnectionStats, EnhancedConnectionManager},
        events::{Event, EventHandler, EventManager, EventType},
        traits::{Connect, Credentials, DataHandler, InnerConfig, MessageHandler, MessageTransfer},
        types::{Data, MessageType},
    },
};

/// Enhanced WebSocket events based on Python implementation patterns
#[derive(Debug, Clone)]
pub enum WebSocketEvent<Transfer: MessageTransfer> {
    /// Connection established successfully
    Connected { region: Option<String> },
    /// Connection lost with reason
    Disconnected { reason: String },
    /// Authentication completed successfully  
    Authenticated { data: serde_json::Value },
    /// Balance data received
    BalanceUpdated { balance: f64, currency: String },
    /// Order opened successfully
    OrderOpened { order_id: String, data: serde_json::Value },
    /// Order closed with result
    OrderClosed { order_id: String, result: serde_json::Value },
    /// Stream update received (candles, etc.)
    StreamUpdate { asset: String, data: serde_json::Value },
    /// Candles data received
    CandlesReceived { asset: String, candles: Vec<serde_json::Value> },
    /// Message received from WebSocket
    MessageReceived { message: Transfer },
    /// Raw message received (unparsed)
    RawMessageReceived { data: Transfer::Raw },
    /// Message sent to WebSocket
    MessageSent { message: Transfer },
    /// Error occurred during operation
    Error { error: String, context: Option<String> },
    /// Connection is being closed
    Closing,
    /// Keep-alive ping sent
    PingSent { timestamp: Instant },
    /// Pong received
    PongReceived { timestamp: Instant },
}

/// Event handler trait for processing WebSocket events
#[async_trait]
pub trait WebSocketEventHandler<Transfer: MessageTransfer>: Send + Sync {
    /// Handle a WebSocket event
    async fn handle_event(&self, event: &WebSocketEvent<Transfer>) -> BinaryOptionsResult<()>;

    /// Get the handler's name for identification
    fn name(&self) -> &'static str;

    /// Whether this handler should receive specific event types
    fn handles_event(&self, event: &WebSocketEvent<Transfer>) -> bool {
        true // Default: handle all events
    }
}

/// Connection statistics and state tracking (inspired by Python implementation)
#[derive(Debug, Default, Clone)]
pub struct ConnectionState {
    /// Whether currently connected
    pub is_connected: bool,
    /// Total connection attempts made
    pub connection_attempts: u64,
    /// Successful connections established
    pub successful_connections: u64,
    /// Total disconnections
    pub disconnections: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Last ping sent time
    pub last_ping_time: Option<Instant>,
    /// Connection establishment time
    pub connection_start_time: Option<Instant>,
    /// Current connected region
    pub current_region: Option<String>,
    /// Last error encountered
    pub last_error: Option<String>,
    /// Current reconnect attempt count
    pub reconnect_attempts: u32,
    /// Maximum reconnect attempts
    pub max_reconnect_attempts: u32,
    /// Connection quality metrics
    pub avg_response_time: Duration,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

/// Keep-alive manager for persistent connections (like Python's persistent mode)
pub struct KeepAliveManager {
    /// Ping task handle
    ping_task: Option<JoinHandle<()>>,
    /// Reconnection monitoring task
    reconnect_task: Option<JoinHandle<()>>,
    /// Ping interval duration
    ping_interval: Duration,
    /// Whether keep-alive is active
    is_running: bool,
    /// Message sender for pings
    message_sender: Option<Sender<Message>>,
}

impl KeepAliveManager {
    pub fn new(ping_interval: Duration) -> Self {
        Self {
            ping_task: None,
            reconnect_task: None,
            ping_interval,
            is_running: false,
            message_sender: None,
        }
    }

    /// Start keep-alive with ping loop (like Python's _ping_loop)
    pub async fn start(&mut self, message_sender: Sender<Message>) -> BinaryOptionsResult<()> {
        if self.is_running {
            return Ok(());
        }

        self.is_running = true;
        self.message_sender = Some(message_sender.clone());

        // Start ping task similar to Python implementation
        let ping_sender = message_sender.clone();
        let ping_interval = self.ping_interval;
        
        self.ping_task = Some(tokio::spawn(async move {
            let mut interval = interval(ping_interval);
            info!("Starting ping loop with {}s interval", ping_interval.as_secs());
            
            loop {
                interval.tick().await;
                
                // Send ping message like Python: '42["ps"]'
                match ping_sender.send(Message::text(r#"42["ps"]"#.to_string())).await {
                    Ok(_) => {
                        debug!("Sent keep-alive ping");
                    }
                    Err(e) => {
                        error!("Failed to send ping: {}", e);
                        break;
                    }
                }
            }
            
            warn!("Ping loop terminated");
        }));

        info!("Keep-alive manager started");
        Ok(())
    }

    /// Stop keep-alive manager
    pub async fn stop(&mut self) {
        self.is_running = false;
        self.message_sender = None;

        if let Some(task) = self.ping_task.take() {
            task.abort();
        }

        if let Some(task) = self.reconnect_task.take() {
            task.abort();
        }

        info!("Keep-alive manager stopped");
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

/// Enhanced WebSocket client configuration
#[derive(Debug, Clone)]
pub struct WebSocketClientConfig {
    /// Enable automatic reconnection
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay between attempts
    pub reconnect_delay: Duration,
    /// Enable persistent connection with keep-alive
    pub persistent_connection: bool,
    /// Ping interval for keep-alive
    pub ping_interval: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable message batching
    pub enable_batching: bool,
    /// Batching configuration
    pub batching_config: BatchingConfig,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit (messages per second)
    pub rate_limit: Option<u32>,
    /// Maximum concurrent event handlers
    pub max_concurrent_handlers: usize,
    /// Event buffer size
    pub event_buffer_size: usize,
    /// Enable detailed logging
    pub enable_logging: bool,
}

impl Default for WebSocketClientConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(5),
            persistent_connection: false,
            ping_interval: Duration::from_secs(20),
            connection_timeout: Duration::from_secs(10),
            enable_batching: false,
            batching_config: BatchingConfig::default(),
            enable_rate_limiting: false,
            rate_limit: Some(100),
            max_concurrent_handlers: 10,
            event_buffer_size: 1000,
            enable_logging: true,
        }
    }
}

/// Shared state accessible across the application
#[derive(Clone)]
pub struct SharedState<T: DataHandler> {
    /// Application-specific data handler
    pub data: Data<T, T::Transfer>,
    /// Connection state and statistics
    pub connection_state: Arc<RwLock<ConnectionState>>,
    /// Event handlers registry
    pub event_handlers: Arc<RwLock<Vec<Arc<dyn WebSocketEventHandler<T::Transfer>>>>>,
    /// WebSocket client configuration
    pub config: Arc<RwLock<WebSocketClientConfig>>,
    /// Event manager for internal events
    pub event_manager: Arc<EventManager>,
}

impl<T: DataHandler> SharedState<T> {
    /// Add an event handler to the registry
    pub async fn add_event_handler(&self, handler: Arc<dyn WebSocketEventHandler<T::Transfer>>) {
        let mut handlers = self.event_handlers.write().await;
        info!("Added event handler: {}", handler.name());
        handlers.push(handler);
    }

    /// Remove an event handler by name
    pub async fn remove_event_handler(&self, name: &str) -> bool {
        let mut handlers = self.event_handlers.write().await;
        let original_len = handlers.len();
        handlers.retain(|h| h.name() != name);
        let removed = handlers.len() != original_len;
        if removed {
            info!("Removed event handler: {}", name);
        }
        removed
    }

    /// Get current connection state
    pub async fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.read().await.clone()
    }

    /// Update connection state using a closure
    pub async fn update_connection_state<F>(&self, updater: F)
    where
        F: FnOnce(&mut ConnectionState),
    {
        let mut state = self.connection_state.write().await;
        updater(&mut *state);
    }

    /// Broadcast an event to all registered handlers (like Python's _emit_event)
    pub async fn broadcast_event(&self, event: WebSocketEvent<T::Transfer>) {
        let handlers = self.event_handlers.read().await;
        let config = self.get_config().await;
        
        if handlers.is_empty() {
            return;
        }

        let mut tasks = Vec::new();

        for handler in handlers.iter() {
            if handler.handles_event(&event) {
                let handler = handler.clone();
                let event = event.clone();
                
                let task = tokio::spawn(async move {
                    if let Err(e) = handler.handle_event(&event).await {
                        error!("Event handler '{}' failed: {}", handler.name(), e);
                    }
                });
                tasks.push(task);
                
                // Limit concurrent handlers
                if tasks.len() >= config.max_concurrent_handlers {
                    break;
                }
            }
        }

        // Wait for all handlers to complete (with timeout like Python)
        let timeout_duration = Duration::from_secs(5);
        if let Err(_) = tokio::time::timeout(
            timeout_duration, 
            futures_util::future::join_all(tasks)
        ).await {
            warn!("Some event handlers timed out after {:?}", timeout_duration);
        }
    }
}

impl<T: DataHandler> Deref for SharedState<T> {
    type Target = Data<T, T::Transfer>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub struct WebSocketConfig {
    /// Enable message batching for better performance
    pub enable_batching: bool,
    /// Batching configuration
    pub batching_config: BatchingConfig,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit (messages per second)
    pub rate_limit: Option<u32>,
    /// Maximum concurrent event handlers
    pub max_concurrent_handlers: usize,
    /// Event buffer size
    pub event_buffer_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enable_batching: false,
            enable_rate_limiting: false,
            batching_config: BatchingConfig::default(),
            rate_limit: Some(100),
            max_concurrent_handlers: 10,
            event_buffer_size: 1000,
        }
    }
}

impl<T: DataHandler> SharedState<T> {
    /// Create new shared state with default configuration
    pub fn new(data: Data<T, T::Transfer>, buffer_size: usize) -> Self {
        Self {
            data,
            connection_state: Arc::new(RwLock::new(ConnectionState::default())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(RwLock::new(WebSocketClientConfig::default())),
            event_manager: Arc::new(EventManager::new(buffer_size))
        }
    }

    /// Add an event handler to the registry
    pub async fn add_handler(&self, handler: Arc<dyn WebSocketEventHandler<T::Transfer>>) {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(handler);
    }

    /// Remove an event handler by name
    pub async fn remove_handler(&self, name: &str) -> bool {
        let mut handlers = self.event_handlers.write().await;
        let original_len = handlers.len();
        handlers.retain(|h| h.name() != name);
        handlers.len() != original_len
    }

    /// Get current connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        self.stats.read().await.clone()
    }

    /// Update connection statistics
    pub async fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut ConnectionStats),
    {
        let mut stats = self.stats.write().await;
        updater(&mut *stats);
    }

    /// Get current configuration
    pub async fn get_config(&self) -> WebSocketClientConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config<F>(&self, updater: F)
    where
        F: FnOnce(&mut WebSocketClientConfig),
    {
        let mut config = self.config.write().await;
        updater(&mut *config);
    }
}

/// Enhanced WebSocket client with event-driven architecture
#[derive(Clone)]
pub struct WebSocketClient2<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    inner: Arc<WebSocketInnerClient2<Transfer, Handler, Connector, Creds, T, U>>,
}

/// Internal client implementation with event processing
pub struct WebSocketInnerClient2<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    /// Authentication credentials
    pub credentials: Creds,
    /// Connection handler
    pub connector: Connector,
    /// Message processor
    pub handler: Handler,
    /// Shared application state
    pub shared_state: SharedState<T>,
    /// Message sender for outgoing messages
    pub sender: Sender<Message>,
    /// Configuration from the original system
    pub config: Config<T, Transfer, U>,
    /// Event loop handle
    _event_loop: JoinHandle<BinaryOptionsResult<()>>,
    /// Optional message batcher for performance
    batcher: Option<MessageBatcher>,
    /// Optional rate limiter
    rate_limiter: Option<RateLimiter>,
}

impl<Transfer, Handler, Connector, Creds, T, U> Deref
    for WebSocketClient2<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    type Target = WebSocketInnerClient2<Transfer, Handler, Connector, Creds, T, U>;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl<Transfer, Handler, Connector, Creds, T, U>
    WebSocketClient2<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer + 'static,
    Handler: MessageHandler<Transfer = Transfer> + 'static,
    Creds: Credentials + 'static,
    Connector: Connect<Creds = Creds> + 'static,
    T: DataHandler<Transfer = Transfer> + 'static,
    U: InnerConfig + 'static,
{
    /// Initialize a new WebSocket client with event-driven architecture
    pub async fn init(
        credentials: Creds,
        connector: Connector,
        data: Data<T, Transfer>,
        handler: Handler,
        config: Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<Self> {
        let inner =
            WebSocketInnerClient2::init(credentials, connector, data, handler, config).await?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Add an event handler to process WebSocket events
    pub async fn add_event_handler(&self, handler: Arc<dyn EventHandler<Transfer>>) {
        self.shared_state.add_handler(handler).await;
    }

    /// Remove an event handler by name
    pub async fn remove_event_handler(&self, name: &str) -> bool {
        self.shared_state.remove_handler(name).await
    }

    /// Get current connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        self.shared_state.get_stats().await
    }

    /// Update WebSocket configuration
    pub async fn update_websocket_config<F>(&self, updater: F)
    where
        F: FnOnce(&mut WebSocketConfig),
    {
        self.shared_state.update_config(updater).await;
    }
}

impl<Transfer, Handler, Connector, Creds, T, U>
    WebSocketInnerClient2<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer + 'static,
    Handler: MessageHandler<Transfer = Transfer> + 'static,
    Creds: Credentials + 'static,
    Connector: Connect<Creds = Creds> + 'static,
    T: DataHandler<Transfer = Transfer> + 'static,
    U: InnerConfig + 'static,
{
    /// Initialize the internal client and start background tasks
    pub async fn init(
        credentials: Creds,
        connector: Connector,
        data: Data<T, Transfer>,
        handler: Handler,
        config: Config<T, Transfer, U>,
    ) -> BinaryOptionsResult<Self> {
        // Test connection first
        let _test_connection = connector.connect(credentials.clone(), &config).await?;

        // Create shared state
        let shared_state = SharedState::new(data);

        // Create communication channels
        let (sender, receiver) = bounded(MAX_CHANNEL_CAPACITY);

        // Initialize optional components based on configuration
        let ws_config = shared_state.get_config().await;
        let batcher = if ws_config.enable_batching {
            Some(MessageBatcher::new(ws_config.batching_config))
        } else {
            None
        };

        let rate_limiter = if ws_config.enable_rate_limiting {
            ws_config.rate_limit.map(RateLimiter::new)
        } else {
            None
        };

        // Start the main event loop
        let event_loop = Self::start_event_loop(
            handler.clone(),
            credentials.clone(),
            shared_state.clone(),
            connector.clone(),
            config.clone(),
            receiver,
        )
        .await?;

        // Wait for initialization
        sleep(config.get_connection_initialization_timeout()?).await;

        Ok(Self {
            credentials,
            connector,
            handler,
            shared_state,
            sender,
            config,
            _event_loop: event_loop,
            batcher,
            rate_limiter,
        })
    }

    /// Start the main event loop that handles all WebSocket operations
    async fn start_event_loop(
        handler: Handler,
        credentials: Creds,
        shared_state: SharedState<T>,
        connector: Connector,
        config: Config<T, Transfer, U>,
        message_receiver: Receiver<Message>,
    ) -> BinaryOptionsResult<JoinHandle<BinaryOptionsResult<()>>> {
        let task = tokio::spawn(async move {
            let mut reconnect_attempts = 0;
            let max_reconnects = config.get_max_allowed_loops()?;

            loop {
                // Update connection stats
                shared_state
                    .update_stats(|stats| {
                        stats.connection_attempts += 1;
                    })
                    .await;

                // Attempt to connect
                match connector.connect(credentials.clone(), &config).await {
                    Ok(websocket) => {
                        info!("WebSocket connection established");

                        // Update stats
                        shared_state
                            .update_stats(|stats| {
                                stats.successful_connections += 1;
                                stats.connected_at = Some(std::time::Instant::now());
                            })
                            .await;

                        // Broadcast connected event
                        shared_state
                            .broadcast_event(WebSocketEvent::Connected)
                            .await;

                        // Split the WebSocket stream
                        let (write, read) = websocket.split();

                        // Run the connection until it fails
                        match Self::run_connection(
                            handler.clone(),
                            shared_state.clone(),
                            message_receiver.clone(),
                            write,
                            read,
                        )
                        .await
                        {
                            Ok(_) => {
                                info!("Connection closed gracefully");
                                break;
                            }
                            Err(e) => {
                                error!("Connection failed: {}", e);

                                // Update stats
                                shared_state
                                    .update_stats(|stats| {
                                        stats.disconnections += 1;
                                        stats.last_error = Some(e.to_string());
                                        stats.connected_at = None;
                                    })
                                    .await;

                                // Broadcast disconnected event
                                shared_state
                                    .broadcast_event(WebSocketEvent::Disconnected(e.to_string()))
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to connect: {}", e);

                        // Update stats
                        shared_state
                            .update_stats(|stats| {
                                stats.last_error = Some(e.to_string());
                            })
                            .await;

                        // Broadcast error event
                        shared_state
                            .broadcast_event(WebSocketEvent::Error(e.to_string()))
                            .await;
                    }
                }

                // Check if we should continue reconnecting
                reconnect_attempts += 1;
                if reconnect_attempts >= max_reconnects {
                    error!("Max reconnection attempts reached");
                    return Err(BinaryOptionsToolsError::MaxReconnectAttemptsReached(
                        max_reconnects,
                    ));
                }

                // Wait before reconnecting
                let sleep_duration = Duration::from_secs(config.get_sleep_interval()?);
                warn!(
                    "Reconnecting in {:?} (attempt {} of {})",
                    sleep_duration, reconnect_attempts, max_reconnects
                );
                sleep(sleep_duration).await;
            }

            Ok(())
        });

        Ok(task)
    }

    /// Run a single WebSocket connection until it fails or is closed
    async fn run_connection(
        handler: Handler,
        shared_state: SharedState<T>,
        message_receiver: Receiver<Message>,
        mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> BinaryOptionsResult<()> {
        // Spawn message sender task
        let sender_task = {
            let mut write = write.clone();
            let shared_state = shared_state.clone();
            tokio::spawn(async move {
                while let Ok(message) = message_receiver.recv().await {
                    // Apply rate limiting if enabled
                    // (Implementation would check shared_state config)

                    if let Err(e) = write.send(message.clone()).await {
                        error!("Failed to send message: {}", e);
                        return Err(BinaryOptionsToolsError::WebSocketMessageError(
                            e.to_string(),
                        ));
                    }

                    // Update stats
                    shared_state
                        .update_stats(|stats| {
                            stats.messages_sent += 1;
                        })
                        .await;

                    debug!("Message sent successfully");
                }
                Ok(())
            })
        };

        // Spawn message receiver task
        let receiver_task = {
            let shared_state = shared_state.clone();
            let handler = handler.clone();
            tokio::spawn(async move {
                let mut previous_info = None;

                while let Some(message_result) = read.next().await {
                    match message_result {
                        Ok(message) => {
                            // Update stats
                            shared_state
                                .update_stats(|stats| {
                                    stats.messages_received += 1;
                                })
                                .await;

                            // Process the message
                            match handler
                                .process_message(&message, &previous_info, &shared_state.data.raw_sender())
                                .await
                            {
                                Ok((processed_message, should_close)) => {
                                    if should_close {
                                        info!("Received close frame");
                                        shared_state.broadcast_event(WebSocketEvent::Closing).await;
                                        return Ok(());
                                    }

                                    if let Some(msg_type) = processed_message {
                                        match msg_type {
                                            crate::general::types::MessageType::Info(info) => {
                                                debug!("Received info: {}", info);
                                                previous_info = Some(info);
                                            }
                                            crate::general::types::MessageType::Transfer(
                                                transfer,
                                            ) => {
                                                debug!("Received transfer: {}", transfer.info());

                                                // Update data
                                                if let Err(e) = shared_state
                                                    .data
                                                    .update_data(transfer.clone())
                                                    .await
                                                {
                                                    error!("Failed to update data: {}", e);
                                                }

                                                // Broadcast message received event
                                                shared_state
                                                    .broadcast_event(
                                                        WebSocketEvent::MessageReceived(transfer),
                                                    )
                                                    .await;
                                            }
                                            crate::general::types::MessageType::Raw(raw) => {
                                                debug!("Received raw message");

                                                // Send to raw receivers
                                                if let Err(e) =
                                                    shared_state.data.raw_send(raw.clone()).await
                                                {
                                                    error!("Failed to send raw message: {}", e);
                                                }

                                                // Broadcast raw message event
                                                shared_state
                                                    .broadcast_event(
                                                        WebSocketEvent::RawMessageReceived(raw),
                                                    )
                                                    .await;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Message processing error: {}", e);
                                    shared_state
                                        .broadcast_event(WebSocketEvent::Error(e.to_string()))
                                        .await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("WebSocket message error: {}", e);
                            return Err(BinaryOptionsToolsError::WebSocketMessageError(
                                e.to_string(),
                            ));
                        }
                    }
                }

                Err(BinaryOptionsToolsError::WebSocketMessageError(
                    "Message stream ended unexpectedly".to_string(),
                ))
            })
        };

        // Wait for either task to complete
        tokio::select! {
            result = sender_task => {
                result??;
            }
            result = receiver_task => {
                result??;
            }
        }

        Ok(())
    }

    /// Send a message through the WebSocket connection
    pub async fn send_message(&self, message: Message) -> BinaryOptionsResult<()> {
        // Apply rate limiting if enabled
        if let Some(rate_limiter) = &self.rate_limiter {
            rate_limiter.acquire().await?;
        }

        // Send through batcher if enabled, otherwise send directly
        if let Some(batcher) = &self.batcher {
            batcher.add_message(message).await?;
        } else {
            self.sender
                .send(message)
                .await
                .map_err(|e| BinaryOptionsToolsError::ChannelRequestSendingError(e.to_string()))?;
        }

        Ok(())
    }

    /// Get access to the shared state for advanced operations
    pub fn get_shared_state(&self) -> &SharedState<T> {
        &self.shared_state
    }
}

// Example event handlers that can be used with the new client

/// Default logging event handler
pub struct LoggingEventHandler;

#[async_trait]
impl<Transfer: MessageTransfer> EventHandler<Transfer> for LoggingEventHandler {
    async fn handle_event(&self, event: WebSocketEvent<Transfer>) -> BinaryOptionsResult<()> {
        match event {
            WebSocketEvent::Connected => {
                info!("WebSocket connected");
            }
            WebSocketEvent::Disconnected(reason) => {
                warn!("WebSocket disconnected: {}", reason);
            }
            WebSocketEvent::MessageReceived(msg) => {
                debug!("Message received: {}", msg.info());
            }
            WebSocketEvent::MessageSent(msg) => {
                debug!("Message sent: {}", msg.info());
            }
            WebSocketEvent::Error(error) => {
                error!("WebSocket error: {}", error);
            }
            WebSocketEvent::Closing => {
                info!("WebSocket closing");
            }
            WebSocketEvent::RawMessageReceived(_) => {
                debug!("Raw message received");
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "LoggingEventHandler"
    }
}

/// Statistics tracking event handler
pub struct StatsEventHandler {
    custom_stats: Arc<Mutex<HashMap<String, u64>>>,
}

impl StatsEventHandler {
    pub fn new() -> Self {
        Self {
            custom_stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_custom_stats(&self) -> HashMap<String, u64> {
        self.custom_stats.lock().await.clone()
    }
}

#[async_trait]
impl<Transfer: MessageTransfer> EventHandler<Transfer> for StatsEventHandler {
    async fn handle_event(&self, event: WebSocketEvent<Transfer>) -> BinaryOptionsResult<()> {
        let mut stats = self.custom_stats.lock().await;

        match event {
            WebSocketEvent::Connected => {
                *stats.entry("connections".to_string()).or_insert(0) += 1;
            }
            WebSocketEvent::Disconnected(_) => {
                *stats.entry("disconnections".to_string()).or_insert(0) += 1;
            }
            WebSocketEvent::MessageReceived(_) => {
                *stats.entry("messages_received".to_string()).or_insert(0) += 1;
            }
            WebSocketEvent::MessageSent(_) => {
                *stats.entry("messages_sent".to_string()).or_insert(0) += 1;
            }
            WebSocketEvent::Error(_) => {
                *stats.entry("errors".to_string()).or_insert(0) += 1;
            }
            _ => {}
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "StatsEventHandler"
    }

    fn handles_event(&self, event: &WebSocketEvent<Transfer>) -> bool {
        matches!(
            event,
            WebSocketEvent::Connected
                | WebSocketEvent::Disconnected(_)
                | WebSocketEvent::MessageReceived(_)
                | WebSocketEvent::MessageSent(_)
                | WebSocketEvent::Error(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Default)]
    struct TestEventHandler {
        event_count: AtomicU64,
    }

    #[async_trait]
    impl<Transfer: MessageTransfer> EventHandler<Transfer> for TestEventHandler {
        async fn handle_event(&self, _event: WebSocketEvent<Transfer>) -> BinaryOptionsResult<()> {
            self.event_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn name(&self) -> &'static str {
            "TestEventHandler"
        }
    }

    // Additional tests would go here
}
