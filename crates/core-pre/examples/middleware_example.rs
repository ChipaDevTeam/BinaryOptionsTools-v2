use binary_options_tools_core_pre::builder::ClientBuilder;
use binary_options_tools_core_pre::connector::{Connector, ConnectorResult, WsStream};
use binary_options_tools_core_pre::error::CoreResult;
use binary_options_tools_core_pre::middleware::{WebSocketMiddleware, MiddlewareContext};
use binary_options_tools_core_pre::traits::{ApiModule, AppState, Rule};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_tungstenite::tungstenite::Message;
use kanal::{AsyncReceiver, AsyncSender};
use tracing::info;

#[derive(Debug)]
struct ExampleState;

#[async_trait]
impl AppState for ExampleState {
    async fn clear_temporal_data(&self) {}
}

// Example statistics middleware
struct StatisticsMiddleware {
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    connections: AtomicU64,
    disconnections: AtomicU64,
}

impl StatisticsMiddleware {
    pub fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            connections: AtomicU64::new(0),
            disconnections: AtomicU64::new(0),
        }
    }

    pub fn get_stats(&self) -> StatisticsReport {
        StatisticsReport {
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            connections: self.connections.load(Ordering::Relaxed),
            disconnections: self.disconnections.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatisticsReport {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections: u64,
    pub disconnections: u64,
}

#[async_trait]
impl WebSocketMiddleware<ExampleState> for StatisticsMiddleware {
    async fn on_send(&self, message: &Message, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        
        let size = match message {
            Message::Text(text) => text.len() as u64,
            Message::Binary(data) => data.len() as u64,
            _ => 0,
        };
        self.bytes_sent.fetch_add(size, Ordering::Relaxed);
        
        info!("Middleware: Sending message (size: {} bytes)", size);
        Ok(())
    }

    async fn on_receive(&self, message: &Message, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        
        let size = match message {
            Message::Text(text) => text.len() as u64,
            Message::Binary(data) => data.len() as u64,
            _ => 0,
        };
        self.bytes_received.fetch_add(size, Ordering::Relaxed);
        
        info!("Middleware: Received message (size: {} bytes)", size);
        Ok(())
    }

    async fn on_connect(&self, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        self.connections.fetch_add(1, Ordering::Relaxed);
        info!("Middleware: Connected to WebSocket");
        Ok(())
    }

    async fn on_disconnect(&self, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        self.disconnections.fetch_add(1, Ordering::Relaxed);
        info!("Middleware: Disconnected from WebSocket");
        Ok(())
    }
}

// Example logging middleware
struct LoggingMiddleware;

#[async_trait]
impl WebSocketMiddleware<ExampleState> for LoggingMiddleware {
    async fn on_send(&self, message: &Message, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        info!("Logging: Sending message: {:?}", message);
        Ok(())
    }

    async fn on_receive(&self, message: &Message, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        info!("Logging: Received message: {:?}", message);
        Ok(())
    }

    async fn on_connect(&self, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        info!("Logging: WebSocket connected");
        Ok(())
    }

    async fn on_disconnect(&self, _context: &MiddlewareContext<ExampleState>) -> CoreResult<()> {
        info!("Logging: WebSocket disconnected");
        Ok(())
    }
}

// Mock connector for demonstration
struct MockConnector;

#[async_trait]
impl Connector<ExampleState> for MockConnector {
    async fn connect(&self, _: Arc<ExampleState>) -> ConnectorResult<WsStream> {
        // This would be a real WebSocket connection in practice
        Err(binary_options_tools_core_pre::connector::ConnectorError::Custom("Mock connector".to_string()))
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

// Example API module
pub struct ExampleModule {
    _msg_rx: AsyncReceiver<Arc<Message>>,
}

#[async_trait]
impl ApiModule<ExampleState> for ExampleModule {
    type Command = String;
    type CommandResponse = String;
    type Handle = ExampleHandle;

    fn new(
        _state: Arc<ExampleState>,
        _cmd_rx: AsyncReceiver<Self::Command>,
        _cmd_ret_tx: AsyncSender<Self::CommandResponse>,
        msg_rx: AsyncReceiver<Arc<Message>>,
        _to_ws: AsyncSender<Message>,
    ) -> Self {
        Self {
            _msg_rx: msg_rx,
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
        
    ) -> Self::Handle {
        ExampleHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        // Example module logic
        info!("Example module running");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(move |_msg: &Message| true)
    }
}

#[derive(Clone)]
pub struct ExampleHandle {
    sender: AsyncSender<String>,
    receiver: AsyncReceiver<String>,
}

#[tokio::main]
async fn main() -> CoreResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create statistics middleware
    let stats_middleware = Arc::new(StatisticsMiddleware::new());
    
    // Build the client with middleware
    let (client, mut runner) = ClientBuilder::new(MockConnector, ExampleState)
        .with_middleware(Box::new(LoggingMiddleware))
        .with_middleware(Box::new(StatisticsMiddleware::new()))
        .with_module::<ExampleModule>()
        .build()
        .await?;

    info!("Client built with middleware layers");

    // In a real application, you would:
    // 1. Start the runner in a background task
    // 2. Use the client to send messages
    // 3. Check statistics periodically
    
    // For demonstration, we'll just show the statistics
    let stats = stats_middleware.get_stats();
    info!("Current statistics: {:?}", stats);

    Ok(())
}
