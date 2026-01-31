use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender, bounded};
use async_trait::async_trait;
use futures_util::{
    SinkExt, StreamExt,
    future::select_all,
    stream::{SplitSink, SplitStream},
};
use tokio::{
    net::TcpStream,
    select,
    sync::{Mutex, RwLock, Notify},
    task::JoinHandle,
    time::{interval, sleep, timeout},
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
        connection::{ConnectionManager, EnhancedConnectionManager},
        events::{Event, EventManager, EventType},
        send::SenderMessage,
        traits::{Connect, Credentials, DataHandler, InnerConfig, MessageHandler, MessageTransfer},
        types::{Data, MessageType},
    },
};

/// Enhanced WebSocket client with modern patterns inspired by the Python implementation
#[derive(Clone)]
pub struct EnhancedWebSocketClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    inner: Arc<EnhancedWebSocketInner<Transfer, Handler, Connector, Creds, T, U>>,
}

/// Internal client implementation following the Python patterns
pub struct EnhancedWebSocketInner<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer,
    Handler: MessageHandler,
    Connector: Connect,
    Creds: Credentials,
    T: DataHandler,
    U: InnerConfig,
{
    /// Connection manager similar to Python implementation
    connection_manager: Arc<EnhancedConnectionManager>,
    /// Event manager for handling WebSocket events
    event_manager: Arc<EventManager>,
    /// Application data handler
    data: Data<T, Transfer>,
    /// Message sender for outgoing messages
    message_sender: Sender<Message>,
    /// Message receiver for outgoing messages
    message_receiver: Receiver<Message>,
    /// Configuration
    config: Config<T, Transfer, U>,
    /// Reconnect notification
    reconnect_notify: Arc<Notify>,
    /// Connection state and statistics
    connection_state: Arc<RwLock<ConnectionState>>,
    /// Background tasks
    background_tasks: Arc<Mutex<Vec<JoinHandle<BinaryOptionsResult<()>>>>>,
    /// Keep-alive manager
    keep_alive: Arc<Mutex<Option<KeepAliveManager>>>,
    /// Message batcher for performance optimization
    message_batcher: Arc<MessageBatcher>,
    /// Auto-reconnect settings
    auto_reconnect: bool,
    /// Connection URLs to try
    connection_urls: Vec<Url>,
    /// Reconnection supervisor task
    reconnect_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Connection state tracking similar to Python implementation
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub is_connected: bool,
    pub connection_attempts: u64,
    pub successful_connections: u64,
    pub disconnections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub last_ping_time: Option<Instant>,
    pub connection_start_time: Option<Instant>,
    pub current_region: Option<String>,
    pub last_error: Option<String>,
    pub reconnect_attempts: u32,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            is_connected: false,
            connection_attempts: 0,
            successful_connections: 0,
            disconnections: 0,
            messages_sent: 0,
            messages_received: 0,
            last_ping_time: None,
            connection_start_time: None,
            current_region: None,
            last_error: None,
            reconnect_attempts: 0,
        }
    }
}

/// Keep-alive manager similar to Python's persistent connection
pub struct KeepAliveManager {
    ping_task: Option<JoinHandle<()>>,
    reconnect_task: Option<JoinHandle<()>>,
    ping_interval: Duration,
    is_running: bool,
}

impl KeepAliveManager {
    pub fn new(ping_interval: Duration) -> Self {
        Self {
            ping_task: None,
            reconnect_task: None,
            ping_interval,
            is_running: false,
        }
    }

    pub async fn start(&mut self, message_sender: Sender<Message>) {
        if self.is_running {
            return;
        }

        self.is_running = true;

        // Start ping task (like Python implementation)
        let ping_sender = message_sender.clone();
        let ping_interval = self.ping_interval;
        self.ping_task = Some(tokio::spawn(async move {
            let mut interval = interval(ping_interval);
            loop {
                interval.tick().await;
                if let Err(e) = ping_sender
                    .send(Message::Text(r#"42["ps"]"#.to_string()))
                    .await
                {
                    error!("Failed to send ping: {}", e);
                    break;
                }
                debug!("Sent ping message");
            }
        }));
    }

    pub async fn stop(&mut self) {
        self.is_running = false;

        if let Some(task) = self.ping_task.take() {
            task.abort();
        }

        if let Some(task) = self.reconnect_task.take() {
            task.abort();
        }
    }
}

impl<Transfer, Handler, Connector, Creds, T, U>
    EnhancedWebSocketClient<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer + 'static,
    Handler: MessageHandler<Transfer = Transfer> + 'static,
    Creds: Credentials + 'static,
    Connector: Connect<Creds = Creds> + 'static,
    T: DataHandler<Transfer = Transfer> + 'static,
    U: InnerConfig + 'static,
{
    /// Initialize the enhanced WebSocket client
    pub async fn init(
        credentials: Creds,
        data: Data<T, Transfer>,
        handler: Handler,
        config: Config<T, Transfer, U>,
        connection_urls: Vec<Url>,
        auto_reconnect: bool,
    ) -> BinaryOptionsResult<Self> {
        let inner = EnhancedWebSocketInner::init(
            credentials,
            data,
            handler,
            config,
            connection_urls,
            auto_reconnect,
        )
        .await?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Connect to WebSocket with automatic region fallback (like Python)
    pub async fn connect(&self) -> BinaryOptionsResult<()> {
        self.inner.connect().await
    }

    /// Connect with persistent connection and keep-alive (like Python)
    pub async fn connect_persistent(&self) -> BinaryOptionsResult<()> {
        self.inner.connect_persistent().await?;

        // Start reconnection supervisor
        let mut task_lock = self.inner.reconnect_task.lock().await;
        if task_lock.is_none() {
            let inner = self.inner.clone();
            *task_lock = Some(tokio::spawn(async move {
                let mut attempts = 0;
                loop {
                    inner.reconnect_notify.notified().await;
                    
                    if !inner.auto_reconnect {
                        break;
                    }

                    info!("Connection lost, attempting to reconnect...");
                    
                    // Exponential backoff with jitter
                    attempts += 1;
                    let base_delay = inner.config.reconnect_time;
                    let delay_secs = std::cmp::min(
                        base_delay.saturating_mul(2u64.saturating_pow(attempts.min(10))),
                        300
                    );
                    
                    use rand::Rng;
                    let jitter = rand::rng().random_range(0.8..1.2);
                    let delay = Duration::from_secs_f64(delay_secs as f64 * jitter);
                    
                    debug!("Reconnection attempt {}, sleeping for {:?}", attempts, delay);
                    sleep(delay).await;

                    match inner.connect().await {
                        Ok(_) => {
                            info!("Reconnected successfully");
                            attempts = 0;
                            // Restart keep-alive if needed
                            if let Some(keep_alive_manager) = inner.keep_alive.lock().await.as_mut() {
                                keep_alive_manager.start(inner.message_sender.clone()).await;
                            }
                        }
                        Err(e) => {
                            error!("Reconnect failed (attempt {}): {}", attempts, e);
                            // Trigger notify again to retry
                            inner.reconnect_notify.notify_one();
                        }
                    }
                }
            }));
        }

        Ok(())
    }

    /// Disconnect gracefully
    pub async fn disconnect(&self) -> BinaryOptionsResult<()> {
        self.inner.disconnect().await
    }

    /// Send a message (with automatic retry logic like Python)
    pub async fn send_message(&self, message: Message) -> BinaryOptionsResult<()> {
        self.inner.send_message(message).await
    }

    /// Send a raw message string (like Python's send_message)
    pub async fn send_raw_message(&self, message: &str) -> BinaryOptionsResult<()> {
        self.inner
            .send_message(Message::Text(message.to_string()))
            .await
    }

    /// Check if connected (like Python's is_connected property)
    pub async fn is_connected(&self) -> bool {
        self.inner.connection_state.read().await.is_connected
    }

    /// Get connection statistics (like Python's get_connection_stats)
    pub async fn get_connection_stats(&self) -> ConnectionState {
        self.inner.connection_state.read().await.clone()
    }

    /// Add event handler for WebSocket events
    pub async fn add_event_handler<F>(
        &self,
        event_type: EventType,
        handler: F,
    ) -> BinaryOptionsResult<()>
    where
        F: Fn(&serde_json::Value) -> BinaryOptionsResult<()> + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        self.inner
            .event_manager
            .add_handler(event_type, handler)
            .await;
        Ok(())
    }

    /// Get current region (like Python's connection_info)
    pub async fn get_current_region(&self) -> Option<String> {
        self.inner
            .connection_state
            .read()
            .await
            .current_region
            .clone()
    }
}

impl<Transfer, Handler, Connector, Creds, T, U>
    EnhancedWebSocketInner<Transfer, Handler, Connector, Creds, T, U>
where
    Transfer: MessageTransfer + 'static,
    Handler: MessageHandler<Transfer = Transfer> + 'static,
    Creds: Credentials + 'static,
    Connector: Connect<Creds = Creds> + 'static,
    T: DataHandler<Transfer = Transfer> + 'static,
    U: InnerConfig + 'static,
{
    /// Initialize the inner client
    pub async fn init(
        credentials: Creds,
        data: Data<T, Transfer>,
        handler: Handler,
        config: Config<T, Transfer, U>,
        connection_urls: Vec<Url>,
        auto_reconnect: bool,
    ) -> BinaryOptionsResult<Self> {
        // Create connection manager
        let connection_manager = Arc::new(EnhancedConnectionManager::new(
            10,                      // max_connections
            Duration::from_secs(10), // connect_timeout
            false,                   // ssl_verify
        ));

        // Create event manager
        let event_manager = Arc::new(EventManager::new(1000));

        // Create message channel
        let (message_sender, message_receiver) = bounded(MAX_CHANNEL_CAPACITY);

        // Create reconnect notify
        let reconnect_notify = Arc::new(Notify::new());

        // Create connection state
        let connection_state = Arc::new(RwLock::new(ConnectionState::default()));

        // Create message batcher
        let batching_config = BatchingConfig::default();
        let message_batcher = Arc::new(MessageBatcher::new(batching_config));

        // Create keep-alive manager
        let keep_alive = Arc::new(Mutex::new(Some(KeepAliveManager::new(
            Duration::from_secs(20),
        ))));

        Ok(Self {
            connection_manager,
            event_manager,
            data,
            message_sender,
            message_receiver,
            config,
            reconnect_notify,
            connection_state,
            background_tasks: Arc::new(Mutex::new(Vec::new())),
            keep_alive,
            message_batcher,
            auto_reconnect,
            connection_urls,
            reconnect_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Connect with automatic region fallback (following Python patterns)
    pub async fn connect(&self) -> BinaryOptionsResult<()> {
        let mut state = self.connection_state.write().await;
        state.connection_attempts += 1;
        drop(state);

        // Try each URL in sequence (like Python)
        for url in &self.connection_urls {
            match self.try_connect_single(url).await {
                Ok(websocket) => {
                    info!(
                        "Connected to region: {}",
                        url.host_str().unwrap_or("unknown")
                    );

                    // Update connection state
                    let mut state = self.connection_state.write().await;
                    state.is_connected = true;
                    state.successful_connections += 1;
                    state.connection_start_time = Some(Instant::now());
                    state.current_region = url.host_str().map(|s| s.to_string());
                    state.reconnect_attempts = 0;
                    drop(state);

                    // Emit connected event
                    self.event_manager
                        .emit(Event::new(
                            EventType::Connected,
                            serde_json::json!({"region": url.host_str()}),
                        ))
                        .await?;

                    // Start connection handler
                    self.start_connection_handler(websocket).await?;
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to connect to {}: {}", url, e);
                    continue;
                }
            }
        }

        Err(BinaryOptionsToolsError::WebsocketConnectionError(
            tokio_tungstenite::tungstenite::Error::ConnectionClosed,
        ))
    }

    /// Connect with persistent connection and keep-alive
    pub async fn connect_persistent(&self) -> BinaryOptionsResult<()> {
        self.connect().await?;

        // Start keep-alive manager
        if let Some(keep_alive_manager) = self.keep_alive.lock().await.as_mut() {
            keep_alive_manager.start(self.message_sender.clone()).await;
        }

        Ok(())
    }

    /// Try to connect to a single URL
    async fn try_connect_single(
        &self,
        url: &Url,
    ) -> BinaryOptionsResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let start_time = Instant::now();

        match timeout(
            Duration::from_secs(10),
            self.connection_manager.connect(&[url.clone()]),
        )
        .await
        {
            Ok(Ok((websocket, _))) => {
                let response_time = start_time.elapsed();
                debug!("Connected to {} in {:?}", url, response_time);
                Ok(websocket)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(BinaryOptionsToolsError::TimeoutError {
                task: "Connection".to_string(),
                duration: Duration::from_secs(10),
            }),
        }
    }

    /// Start connection handler (combines Python's message sending and receiving loops)
    async fn start_connection_handler(
        &self,
        websocket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> BinaryOptionsResult<()> {
        let (write, read) = websocket.split();

        // Start message sender task
        let sender_task = self.start_sender_task(write).await?;

        // Start message receiver task
        let receiver_task = self.start_receiver_task(read).await?;

        // Store tasks for cleanup
        let mut tasks = self.background_tasks.lock().await;
        tasks.retain(|t| !t.is_finished());
        tasks.push(sender_task);
        tasks.push(receiver_task);

        Ok(())
    }

    /// Start message sender task (like Python's sender loop)
    async fn start_sender_task(
        &self,
        mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    ) -> BinaryOptionsResult<JoinHandle<BinaryOptionsResult<()>>> {
        let message_receiver = self.message_receiver.clone();
        let connection_state = self.connection_state.clone();
        let event_manager = self.event_manager.clone();

        let task = tokio::spawn(async move {
            while let Ok(message) = message_receiver.recv().await {
                match write.send(message.clone()).await {
                    Ok(_) => {
                        // Update stats
                        /*
                        // Note: We already update stats in send_message, but that's when it's queued.
                        // Maybe we want to track actual sent messages here?
                        // For now, let's just log debug
                        */
                        debug!("Message sent to WebSocket");
                    }
                    Err(e) => {
                        error!("Failed to send message to WebSocket: {}", e);
                        event_manager
                            .emit(Event::new(
                                EventType::Error,
                                serde_json::json!({
                                    "error": "Failed to send message",
                                    "details": e.to_string()
                                }),
                            ))
                            .await?;
                        
                        // If we can't write, the connection is likely dead.
                        // The receiver task should handle the close/error, but we can also break here.
                        break;
                    }
                }
            }
            Ok(())
        });

        Ok(task)
    }

    /// Start message receiver task (like Python's listener loop)
    async fn start_receiver_task(
        &self,
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> BinaryOptionsResult<JoinHandle<BinaryOptionsResult<()>>> {
        let connection_state = self.connection_state.clone();
        let event_manager = self.event_manager.clone();
        let data = self.data.clone();

        let task = tokio::spawn(async move {
            while let Some(message_result) = read.next().await {
                match message_result {
                    Ok(message) => {
                        // Update stats
                        {
                            let mut state = connection_state.write().await;
                            state.messages_received += 1;
                        }

                        // Process message (similar to Python's message processing)
                        match message {
                            Message::Text(text) => {
                                debug!("Received text message: {}", text);

                                // Emit message received event
                                event_manager
                                    .emit(Event::new(
                                        EventType::MessageReceived,
                                        serde_json::json!({"message": text}),
                                    ))
                                    .await?;

                                // Process based on message type (like Python's _process_message)
                                Self::process_text_message(&text, &event_manager).await?;
                            }
                            Message::Binary(data) => {
                                debug!("Received binary message: {} bytes", data.len());

                                // Try to parse as JSON (like Python's bytes message handling)
                                if let Ok(text) = String::from_utf8(data) {
                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(&text)
                                    {
                                        event_manager
                                            .emit(Event::new(
                                                EventType::Custom("json_data".to_string()),
                                                json,
                                            ))
                                            .await?;
                                    }
                                }
                            }
                            Message::Close(_) => {
                                info!("WebSocket close frame received");
                                event_manager
                                    .emit(Event::new(
                                        EventType::Disconnected,
                                        serde_json::json!({"reason": "close_frame"}),
                                    ))
                                    .await?;
                                self.reconnect_notify.notify_one();
                                break;
                            }
                            Message::Ping(ping_data) => {
                                debug!("Received ping");
                                if let Err(e) = self.message_sender.try_send(Message::Pong(ping_data)) {
                                    error!("Failed to queue pong: {}", e);
                                }
                            }
                            Message::Pong(_) => {
                                debug!("Received pong");
                            }
                            Message::Frame(_) => {
                                debug!("Received frame");
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocket message error: {}", e);
                        event_manager
                            .emit(Event::new(
                                EventType::Error,
                                serde_json::json!({"error": e.to_string()}),
                            ))
                            .await?;
                        self.reconnect_notify.notify_one();
                        break;
                    }
                }
            }

            // Connection ended
            {
                let mut state = connection_state.write().await;
                state.is_connected = false;
                state.disconnections += 1;
            }

            Ok(())
        });

        Ok(task)
    }

    /// Process text messages (similar to Python's message type handling)
    async fn process_text_message(
        text: &str,
        event_manager: &EventManager,
    ) -> BinaryOptionsResult<()> {
        // Handle different message types like Python implementation
        if text.starts_with("0") && text.contains("sid") {
            // Initial connection message
            debug!("Received initial connection message");
        } else if text == "2" {
            // Ping message
            debug!("Received ping message");
        } else if text.starts_with("40") && text.contains("sid") {
            // Connection established
            event_manager
                .emit(Event::new(
                    EventType::Connected,
                    serde_json::json!({"established": true}),
                ))
                .await?;
        } else if text.starts_with("42") {
            // Socket.IO message
            Self::process_socket_io_message(text, event_manager).await?;
        } else if text.starts_with("451-[") {
            // JSON message
            if let Some(json_part) = text.strip_prefix("451-") {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_part) {
                    Self::handle_json_message(&data, event_manager).await?;
                }
            }
        }

        Ok(())
    }

    /// Process Socket.IO messages (like Python's auth message handling)
    async fn process_socket_io_message(
        text: &str,
        event_manager: &EventManager,
    ) -> BinaryOptionsResult<()> {
        if text.contains("NotAuthorized") {
            event_manager
                .emit(Event::new(
                    EventType::Error,
                    serde_json::json!({"error": "Authentication failed"}),
                ))
                .await?;
        } else if let Some(json_part) = text.strip_prefix("42") {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_part) {
                Self::handle_json_message(&data, event_manager).await?;
            }
        }

        Ok(())
    }

    /// Handle JSON messages (similar to Python's _handle_json_message)
    async fn handle_json_message(
        data: &serde_json::Value,
        event_manager: &EventManager,
    ) -> BinaryOptionsResult<()> {
        if let Some(array) = data.as_array() {
            if let Some(event_type) = array.get(0).and_then(|v| v.as_str()) {
                let event_data = array.get(1).unwrap_or(&serde_json::Value::Null);

                match event_type {
                    "successauth" => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("authenticated".to_string()),
                                event_data.clone(),
                            ))
                            .await?;
                    }
                    "successupdateBalance" => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("balance_updated".to_string()),
                                event_data.clone(),
                            ))
                            .await?;
                    }
                    "successopenOrder" => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("order_opened".to_string()),
                                event_data.clone(),
                            ))
                            .await?;
                    }
                    "successcloseOrder" => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("order_closed".to_string()),
                                event_data.clone(),
                            ))
                            .await?;
                    }
                    "updateStream" => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("stream_update".to_string()),
                                event_data.clone(),
                            ))
                            .await?;
                    }
                    "loadHistoryPeriod" => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("candles_received".to_string()),
                                event_data.clone(),
                            ))
                            .await?;
                    }
                    _ => {
                        event_manager
                            .emit(Event::new(
                                EventType::Custom("unknown_event".to_string()),
                                serde_json::json!({"type": event_type, "data": event_data}),
                            ))
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Send a message through the WebSocket
    pub async fn send_message(&self, message: Message) -> BinaryOptionsResult<()> {
        // Update stats
        {
            let mut state = self.connection_state.write().await;
            state.messages_sent += 1;
        }

        // Send through message batcher or directly
        self.message_sender
            .send(message)
            .await
            .map_err(|e| BinaryOptionsToolsError::ChannelRequestSendingError(e.to_string()))?;

        Ok(())
    }

    /// Disconnect gracefully (like Python's disconnect method)
    pub async fn disconnect(&self) -> BinaryOptionsResult<()> {
        info!("Disconnecting WebSocket client...");

        // Stop keep-alive manager
        if let Some(keep_alive_manager) = self.keep_alive.lock().await.as_mut() {
            keep_alive_manager.stop().await;
        }

        // Stop reconnection supervisor
        if let Some(task) = self.reconnect_task.lock().await.take() {
            task.abort();
        }

        // Cancel all background tasks
        let mut tasks = self.background_tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }

        // Update connection state
        let mut state = self.connection_state.write().await;
        state.is_connected = false;
        state.connection_start_time = None;
        state.current_region = None;

        // Emit disconnected event
        self.event_manager
            .emit(Event::new(
                EventType::Disconnected,
                serde_json::json!({"reason": "manual_disconnect"}),
            ))
            .await?;

        info!("WebSocket client disconnected successfully");
        Ok(())
    }
}

/// Event handler for logging (similar to Python's logging)
pub struct LoggingEventHandler;

impl LoggingEventHandler {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl crate::general::events::EventHandler<serde_json::Value> for LoggingEventHandler {
    async fn handle(&self, event: &Event<serde_json::Value>) -> BinaryOptionsResult<()> {
        match event.event_type {
            EventType::Connected => info!("🔗 WebSocket connected"),
            EventType::Disconnected => warn!("❌ WebSocket disconnected"),
            EventType::MessageReceived => debug!("📨 Message received"),
            EventType::MessageSent => debug!("📤 Message sent"),
            EventType::Error => error!("❌ WebSocket error: {:?}", event.data),
            EventType::Custom(ref name) => match name.as_str() {
                "authenticated" => info!("✅ Successfully authenticated"),
                "balance_updated" => info!("💰 Balance updated"),
                "order_opened" => info!("📈 Order opened"),
                "order_closed" => info!("📊 Order closed"),
                "candles_received" => debug!("🕯️ Candles received"),
                _ => debug!("🔔 Event: {}", name),
            },
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_state() {
        let mut state = ConnectionState::default();
        assert!(!state.is_connected);
        assert_eq!(state.connection_attempts, 0);

        state.connection_attempts += 1;
        assert_eq!(state.connection_attempts, 1);
    }

    #[tokio::test]
    async fn test_keep_alive_manager() {
        let mut manager = KeepAliveManager::new(Duration::from_secs(1));
        assert!(!manager.is_running);

        let (sender, _receiver) = bounded(10);
        manager.start(sender).await;
        assert!(manager.is_running);

        manager.stop().await;
        assert!(!manager.is_running);
    }
}
