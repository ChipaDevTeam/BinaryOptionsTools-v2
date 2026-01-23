use crate::builder::ClientBuilder;
use crate::client::{Client, ClientRunner};
use crate::connector::Connector;
use crate::error::{CoreError, CoreResult};
use crate::middleware::{MiddlewareContext, WebSocketMiddleware};
use crate::statistics::{ConnectionStats, StatisticsTracker};
use crate::traits::AppState;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Configuration for the testing wrapper
#[derive(Debug, Clone)]
pub struct TestingConfig {
    /// How often to collect and log statistics
    pub stats_interval: Duration,
    /// Whether to log statistics to console
    pub log_stats: bool,
    /// Whether to track detailed connection events
    pub track_events: bool,
    /// Maximum number of reconnection attempts
    pub max_reconnect_attempts: Option<u32>,
    /// Delay between reconnection attempts
    pub reconnect_delay: Duration,
    /// Connection timeout duration
    pub connection_timeout: Duration,
    /// Whether to automatically reconnect on disconnection
    pub auto_reconnect: bool,
}

impl Default for TestingConfig {
    fn default() -> Self {
        Self {
            stats_interval: Duration::from_secs(30),
            log_stats: true,
            track_events: true,
            max_reconnect_attempts: Some(5),
            reconnect_delay: Duration::from_secs(5),
            connection_timeout: Duration::from_secs(10),
            auto_reconnect: true,
        }
    }
}

/// A testing wrapper around the Client that provides comprehensive statistics
/// and monitoring capabilities for WebSocket connections.
pub struct TestingWrapper<S: AppState> {
    client: Client<S>,
    runner: Option<ClientRunner<S>>,
    stats: Arc<StatisticsTracker>,
    config: TestingConfig,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    stats_task: Option<tokio::task::JoinHandle<()>>,
    runner_task: Option<tokio::task::JoinHandle<()>>,
}

/// A testing middleware that tracks connection statistics using the shared StatisticsTracker
pub struct TestingMiddleware<S: AppState> {
    stats: Arc<StatisticsTracker>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: AppState> TestingMiddleware<S> {
    /// Create a new testing middleware with the provided StatisticsTracker
    pub fn new(stats: Arc<StatisticsTracker>) -> Self {
        Self {
            stats,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<S: AppState> WebSocketMiddleware<S> for TestingMiddleware<S> {
    async fn on_connection_attempt(&self, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        // 🎯 This is the missing piece!
        self.stats.record_connection_attempt().await;
        debug!(target: "TestingMiddleware", "Connection attempt recorded");
        Ok(())
    }

    async fn on_connection_failure(
        &self,
        _context: &MiddlewareContext<S>,
        reason: Option<String>,
    ) -> CoreResult<()> {
        // 🎯 This will give you proper failure tracking
        self.stats.record_connection_failure(reason).await;
        debug!(target: "TestingMiddleware", "Connection failure recorded");
        Ok(())
    }

    async fn on_connect(&self, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        // This calls record_connection_success - already implemented
        self.stats.record_connection_success().await;
        debug!(target: "TestingMiddleware", "Connection established");
        Ok(())
    }

    async fn on_disconnect(&self, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Record disconnection with reason
        self.stats
            .record_disconnection(Some("Connection lost".to_string()))
            .await;
        debug!(target: "TestingMiddleware", "Connection lost");
        Ok(())
    }

    async fn on_send(&self, message: &Message, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Record message sent with size tracking
        self.stats.record_message_sent(message).await;
        debug!(target: "TestingMiddleware", "Message sent: {} bytes", Self::get_message_size(message));
        Ok(())
    }

    async fn on_receive(
        &self,
        message: &Message,
        _context: &MiddlewareContext<S>,
    ) -> CoreResult<()> {
        // Record message received with size tracking
        self.stats.record_message_received(message).await;
        debug!(target: "TestingMiddleware", "Message received: {} bytes", Self::get_message_size(message));
        Ok(())
    }
}

impl<S: AppState> TestingMiddleware<S> {
    /// Get the size of a message in bytes
    fn get_message_size(message: &Message) -> usize {
        match message {
            Message::Text(text) => text.len(),
            Message::Binary(data) => data.len(),
            Message::Ping(data) => data.len(),
            Message::Pong(data) => data.len(),
            Message::Close(_) => 0,
            Message::Frame(_) => 0,
        }
    }
}

impl<S: AppState> TestingWrapper<S> {
    /// Create a new testing wrapper with the provided client and runner
    pub fn new(client: Client<S>, runner: ClientRunner<S>, config: TestingConfig) -> Self {
        let stats = Arc::new(StatisticsTracker::new());

        Self {
            client,
            runner: Some(runner),
            stats,
            config,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stats_task: None,
            runner_task: None,
        }
    }

    /// Create a new testing wrapper with a shared StatisticsTracker
    /// This is useful when you want to share statistics between multiple components
    pub fn new_with_stats(
        client: Client<S>,
        runner: ClientRunner<S>,
        config: TestingConfig,
        stats: Arc<StatisticsTracker>,
    ) -> Self {
        Self {
            client,
            runner: Some(runner),
            stats,
            config,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stats_task: None,
            runner_task: None,
        }
    }

    /// Create a TestingMiddleware that shares the same StatisticsTracker
    pub fn create_middleware(&self) -> TestingMiddleware<S> {
        TestingMiddleware::new(Arc::clone(&self.stats))
    }

    /// Start the testing wrapper, which will run the client and begin collecting statistics
    pub async fn start(&mut self) -> CoreResult<()> {
        self.is_running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Start statistics collection task
        if self.config.log_stats {
            let stats = self.stats.clone();
            let interval = self.config.stats_interval;
            let is_running = self.is_running.clone();

            self.stats_task = Some(tokio::spawn(async move {
                let mut interval = tokio::time::interval(interval);
                interval.tick().await; // Skip first tick

                while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    interval.tick().await;

                    let stats = stats.get_stats().await;
                    Self::log_statistics(&stats);
                }
            }));
        }

        // Record initial connection attempt
        self.stats.record_connection_attempt().await;

        // Start the actual ClientRunner in a separate task
        // We need to take ownership of the runner to move it into the task
        let runner = self.runner.take().ok_or_else(|| {
            CoreError::Other("Runner has already been started or consumed".to_string())
        })?;
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();

        self.runner_task = Some(tokio::spawn(async move {
            let mut runner = runner;

            // Create a wrapper around the runner that tracks statistics
            let result = Self::run_with_stats(&mut runner, stats.clone()).await;

            // Mark as not running when the runner exits
            is_running.store(false, std::sync::atomic::Ordering::SeqCst);

            match result {
                Ok(_) => {
                    info!("ClientRunner completed successfully");
                }
                Err(e) => {
                    error!("ClientRunner failed: {}", e);
                    // Record connection failure
                    stats.record_connection_failure(Some(e.to_string())).await;
                }
            }
        }));

        info!("Testing wrapper started successfully");
        Ok(())
    }

    /// Run the ClientRunner with statistics tracking
    async fn run_with_stats(
        runner: &mut ClientRunner<S>,
        stats: Arc<StatisticsTracker>,
    ) -> CoreResult<()> {
        // For now, we'll just run the runner directly
        // In a future enhancement, we could intercept connection events
        // and track them more granularly

        // Since ClientRunner.run() doesn't return a Result, we'll assume it succeeds
        // and track the connection success
        stats.record_connection_success().await;
        runner.run().await;
        Ok(())
    }

    /// Stop the testing wrapper
    pub async fn stop(mut self) -> CoreResult<ConnectionStats> {
        self.is_running
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // Abort the statistics task
        if let Some(task) = self.stats_task.take() {
            task.abort();
        }

        // Shutdown the client, which will signal the runner to stop
        // Note: This consumes the client, so we need to handle this carefully
        info!("Sending shutdown command to client...");

        // Record the disconnection before shutting down
        self.stats
            .record_disconnection(Some("Manual stop".to_string()))
            .await;

        // We can't consume self.client here because we need to return self
        // Instead, we'll wait for the runner task to complete naturally
        // The runner should stop when the connection is closed or on error

        if let Some(runner_task) = self.runner_task.take() {
            // Wait for the runner task to complete with a timeout
            match tokio::time::timeout(Duration::from_secs(10), runner_task).await {
                Ok(Ok(())) => {
                    info!("Runner task completed successfully");
                }
                Ok(Err(e)) => {
                    if e.is_cancelled() {
                        info!("Runner task was cancelled");
                    } else {
                        error!("Runner task failed: {}", e);
                    }
                }
                Err(_) => {
                    warn!("Runner task did not complete within timeout, it may still be running");
                }
            }
        }

        let stats = self.get_stats().await;

        // Shutdown the client
        info!("Shutting down client...");
        self.client.shutdown().await?;

        info!("Testing wrapper stopped");
        Ok(stats)
    }

    /// Get the current connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        self.stats.get_stats().await
    }

    /// Get a reference to the underlying client
    pub fn client(&self) -> &Client<S> {
        &self.client
    }

    /// Get a mutable reference to the underlying client
    pub fn client_mut(&mut self) -> &mut Client<S> {
        &mut self.client
    }

    /// Reset all statistics
    pub async fn reset_stats(&self) {
        // Create a new statistics tracker and replace the current one
        // Note: This is a simplified approach. In a real implementation,
        // you might want to use Arc::make_mut or other techniques
        // to properly reset the statistics while maintaining thread safety
        warn!("Statistics reset requested, but not fully implemented");
    }

    /// Export statistics to JSON
    pub async fn export_stats_json(&self) -> CoreResult<String> {
        let stats = self.get_stats().await;
        serde_json::to_string_pretty(&stats)
            .map_err(|e| CoreError::Other(format!("Failed to serialize stats: {e}")))
    }

    /// Export statistics to CSV
    pub async fn export_stats_csv(&self) -> CoreResult<String> {
        let stats = self.get_stats().await;

        let mut csv = String::new();
        csv.push_str("metric,value\n");
        csv.push_str(&format!(
            "connection_attempts,{}\n",
            stats.connection_attempts
        ));
        csv.push_str(&format!(
            "successful_connections,{}\n",
            stats.successful_connections
        ));
        csv.push_str(&format!(
            "failed_connections,{}\n",
            stats.failed_connections
        ));
        csv.push_str(&format!("disconnections,{}\n", stats.disconnections));
        csv.push_str(&format!("reconnections,{}\n", stats.reconnections));
        csv.push_str(&format!(
            "avg_connection_latency_ms,{}\n",
            stats.avg_connection_latency_ms
        ));
        csv.push_str(&format!(
            "last_connection_latency_ms,{}\n",
            stats.last_connection_latency_ms
        ));
        csv.push_str(&format!(
            "total_uptime_seconds,{}\n",
            stats.total_uptime_seconds
        ));
        csv.push_str(&format!(
            "current_uptime_seconds,{}\n",
            stats.current_uptime_seconds
        ));
        csv.push_str(&format!(
            "time_since_last_disconnection_seconds,{}\n",
            stats.time_since_last_disconnection_seconds
        ));
        csv.push_str(&format!("messages_sent,{}\n", stats.messages_sent));
        csv.push_str(&format!("messages_received,{}\n", stats.messages_received));
        csv.push_str(&format!("bytes_sent,{}\n", stats.bytes_sent));
        csv.push_str(&format!("bytes_received,{}\n", stats.bytes_received));
        csv.push_str(&format!(
            "avg_messages_sent_per_second,{}\n",
            stats.avg_messages_sent_per_second
        ));
        csv.push_str(&format!(
            "avg_messages_received_per_second,{}\n",
            stats.avg_messages_received_per_second
        ));
        csv.push_str(&format!(
            "avg_bytes_sent_per_second,{}\n",
            stats.avg_bytes_sent_per_second
        ));
        csv.push_str(&format!(
            "avg_bytes_received_per_second,{}\n",
            stats.avg_bytes_received_per_second
        ));
        csv.push_str(&format!("is_connected,{}\n", stats.is_connected));

        Ok(csv)
    }

    /// Log current statistics to console
    fn log_statistics(stats: &ConnectionStats) {
        info!("=== WebSocket Connection Statistics ===");
        info!(
            "Connection Status: {}",
            if stats.is_connected {
                "CONNECTED"
            } else {
                "DISCONNECTED"
            }
        );
        info!("Connection Attempts: {}", stats.connection_attempts);
        info!("Successful Connections: {}", stats.successful_connections);
        info!("Failed Connections: {}", stats.failed_connections);
        info!("Disconnections: {}", stats.disconnections);
        info!("Reconnections: {}", stats.reconnections);

        if stats.avg_connection_latency_ms > 0.0 {
            info!(
                "Average Connection Latency: {:.2}ms",
                stats.avg_connection_latency_ms
            );
            info!(
                "Last Connection Latency: {:.2}ms",
                stats.last_connection_latency_ms
            );
        }

        info!("Total Uptime: {:.2}s", stats.total_uptime_seconds);
        if stats.is_connected {
            info!(
                "Current Connection Uptime: {:.2}s",
                stats.current_uptime_seconds
            );
        }
        if stats.time_since_last_disconnection_seconds > 0.0 {
            info!(
                "Time Since Last Disconnection: {:.2}s",
                stats.time_since_last_disconnection_seconds
            );
        }

        info!(
            "Messages Sent: {} ({:.2}/s)",
            stats.messages_sent, stats.avg_messages_sent_per_second
        );
        info!(
            "Messages Received: {} ({:.2}/s)",
            stats.messages_received, stats.avg_messages_received_per_second
        );
        info!(
            "Bytes Sent: {} ({:.2}/s)",
            stats.bytes_sent, stats.avg_bytes_sent_per_second
        );
        info!(
            "Bytes Received: {} ({:.2}/s)",
            stats.bytes_received, stats.avg_bytes_received_per_second
        );

        if stats.connection_attempts > 0 {
            let success_rate =
                (stats.successful_connections as f64 / stats.connection_attempts as f64) * 100.0;
            info!("Connection Success Rate: {:.1}%", success_rate);
        }

        info!("========================================");
    }
}

/// A testing connector wrapper that tracks connection statistics
pub struct TestingConnector<C, S> {
    inner: C,
    stats: Arc<StatisticsTracker>,
    config: TestingConfig,
    _phantom: std::marker::PhantomData<S>,
}

impl<C, S> TestingConnector<C, S> {
    pub fn new(inner: C, stats: Arc<StatisticsTracker>, config: TestingConfig) -> Self {
        Self {
            inner,
            stats,
            config,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<C, S> Connector<S> for TestingConnector<C, S>
where
    C: Connector<S> + Send + Sync,
    S: AppState,
{
    async fn connect(
        &self,
        state: Arc<S>,
    ) -> crate::connector::ConnectorResult<crate::connector::WsStream> {
        self.stats.record_connection_attempt().await;

        let start_time = std::time::Instant::now();

        // Apply connection timeout
        let result =
            tokio::time::timeout(self.config.connection_timeout, self.inner.connect(state)).await;

        match result {
            Ok(Ok(stream)) => {
                self.stats.record_connection_success().await;
                debug!("Connection established in {:?}", start_time.elapsed());
                Ok(stream)
            }
            Ok(Err(err)) => {
                self.stats
                    .record_connection_failure(Some(err.to_string()))
                    .await;
                error!("Connection failed: {}", err);
                Err(err)
            }
            Err(_) => {
                let timeout_error = crate::connector::ConnectorError::Timeout;
                self.stats
                    .record_connection_failure(Some(timeout_error.to_string()))
                    .await;
                error!(
                    "Connection timed out after {:?}",
                    self.config.connection_timeout
                );
                Err(timeout_error)
            }
        }
    }

    async fn disconnect(&self) -> crate::connector::ConnectorResult<()> {
        self.stats
            .record_disconnection(Some("Manual disconnect".to_string()))
            .await;
        self.inner.disconnect().await
    }
}

/// Builder for creating a testing wrapper with custom configuration
pub struct TestingWrapperBuilder<S: AppState> {
    config: TestingConfig,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: AppState> TestingWrapperBuilder<S> {
    pub fn new() -> Self {
        Self {
            config: TestingConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_stats_interval(mut self, interval: Duration) -> Self {
        self.config.stats_interval = interval;
        self
    }

    pub fn with_log_stats(mut self, log_stats: bool) -> Self {
        self.config.log_stats = log_stats;
        self
    }

    pub fn with_track_events(mut self, track_events: bool) -> Self {
        self.config.track_events = track_events;
        self
    }

    pub fn with_max_reconnect_attempts(mut self, max_attempts: Option<u32>) -> Self {
        self.config.max_reconnect_attempts = max_attempts;
        self
    }

    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.reconnect_delay = delay;
        self
    }

    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout = timeout;
        self
    }

    pub fn with_auto_reconnect(mut self, auto_reconnect: bool) -> Self {
        self.config.auto_reconnect = auto_reconnect;
        self
    }

    pub fn build(self, client: Client<S>, runner: ClientRunner<S>) -> TestingWrapper<S> {
        TestingWrapper::new(client, runner, self.config)
    }

    /// Build the testing wrapper and return both the wrapper and a compatible middleware
    pub async fn build_with_middleware(
        self,
        builder: ClientBuilder<S>,
    ) -> CoreResult<TestingWrapper<S>> {
        let stats = Arc::new(StatisticsTracker::new());
        let middleware = TestingMiddleware::new(Arc::clone(&stats));
        let (client, runner) = builder
            .with_middleware(Box::new(middleware))
            .build()
            .await?;
        let wrapper = TestingWrapper::new_with_stats(client, runner, self.config, stats);

        Ok(wrapper)
    }
}

impl<S: AppState> Default for TestingWrapperBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}
