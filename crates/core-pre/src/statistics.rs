use kanal::{AsyncReceiver, AsyncSender};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;

/// Comprehensive connection statistics for WebSocket testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// Total number of connection attempts
    pub connection_attempts: u64,
    /// Total number of successful connections
    pub successful_connections: u64,
    /// Total number of failed connections
    pub failed_connections: u64,
    /// Total number of disconnections
    pub disconnections: u64,
    /// Total number of reconnections
    pub reconnections: u64,
    /// Average connection latency in milliseconds
    pub avg_connection_latency_ms: f64,
    /// Last connection latency in milliseconds
    pub last_connection_latency_ms: f64,
    /// Total uptime in seconds
    pub total_uptime_seconds: f64,
    /// Current connection uptime in seconds (if connected)
    pub current_uptime_seconds: f64,
    /// Time since last disconnection in seconds
    pub time_since_last_disconnection_seconds: f64,
    /// Messages sent count
    pub messages_sent: u64,
    /// Messages received count
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Average messages per second (sent)
    pub avg_messages_sent_per_second: f64,
    /// Average messages per second (received)
    pub avg_messages_received_per_second: f64,
    /// Average bytes per second (sent)
    pub avg_bytes_sent_per_second: f64,
    /// Average bytes per second (received)
    pub avg_bytes_received_per_second: f64,
    /// Is currently connected
    pub is_connected: bool,
    /// Connection history (last 10 connections)
    pub connection_history: Vec<ConnectionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionEvent {
    pub event_type: ConnectionEventType,
    pub timestamp: u64,           // Unix timestamp in milliseconds
    pub duration_ms: Option<u64>, // Duration for connection events
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionEventType {
    ConnectionAttempt,
    ConnectionSuccess,
    ConnectionFailure,
    Disconnection,
    Reconnection,
    MessageSent,
    MessageReceived,
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            connection_attempts: 0,
            successful_connections: 0,
            failed_connections: 0,
            disconnections: 0,
            reconnections: 0,
            avg_connection_latency_ms: 0.0,
            last_connection_latency_ms: 0.0,
            total_uptime_seconds: 0.0,
            current_uptime_seconds: 0.0,
            time_since_last_disconnection_seconds: 0.0,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            avg_messages_sent_per_second: 0.0,
            avg_messages_received_per_second: 0.0,
            avg_bytes_sent_per_second: 0.0,
            avg_bytes_received_per_second: 0.0,
            is_connected: false,
            connection_history: Vec::new(),
        }
    }
}

/// Internal statistics tracker with atomic operations for performance
pub struct StatisticsTracker {
    // Atomic counters for thread-safe access
    connection_attempts: AtomicU64,
    successful_connections: AtomicU64,
    failed_connections: AtomicU64,
    disconnections: AtomicU64,
    reconnections: AtomicU64,
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,

    // Connection timing
    start_time: Instant,
    last_connection_attempt: RwLock<Option<Instant>>,
    current_connection_start: RwLock<Option<Instant>>,
    last_disconnection: RwLock<Option<Instant>>,
    total_uptime: RwLock<Duration>,

    // Connection latency tracking
    connection_latencies: RwLock<Vec<Duration>>,

    // Connection state
    is_connected: AtomicBool,

    // Event history
    event_history: RwLock<Vec<ConnectionEvent>>,
}

impl ConnectionStats {
    /// Generate a comprehensive, user-readable summary of the connection statistics
    pub fn summary(&self) -> String {
        let mut summary = String::new();

        // Header
        summary.push_str(
            "╔═══════════════════════════════════════════════════════════════════════════════╗\n",
        );
        summary.push_str(
            "║                         WebSocket Connection Summary                          ║\n",
        );
        summary.push_str(
            "╠═══════════════════════════════════════════════════════════════════════════════╣\n",
        );

        // Connection Status
        let status = if self.is_connected {
            "🟢 CONNECTED"
        } else {
            "🔴 DISCONNECTED"
        };
        summary.push_str(&format!("║ Status: {status:<67} ║\n"));

        // Connection Statistics
        summary.push_str(
            "║                                                                               ║\n",
        );
        summary.push_str(
            "║ Connection Statistics:                                                        ║\n",
        );
        summary.push_str(&format!(
            "║   • Total Attempts: {:<57} ║\n",
            self.connection_attempts
        ));
        summary.push_str(&format!(
            "║   • Successful: {:<61} ║\n",
            self.successful_connections
        ));
        summary.push_str(&format!(
            "║   • Failed: {:<65} ║\n",
            self.failed_connections
        ));
        summary.push_str(&format!(
            "║   • Disconnections: {:<57} ║\n",
            self.disconnections
        ));
        summary.push_str(&format!(
            "║   • Reconnections: {:<58} ║\n",
            self.reconnections
        ));

        // Success Rate
        if self.connection_attempts > 0 {
            let success_rate =
                (self.successful_connections as f64 / self.connection_attempts as f64) * 100.0;
            summary.push_str(&format!(
                "║   • Success Rate: {:<59} ║\n",
                format!("{:.1}%", success_rate)
            ));
        }

        // Connection Latency
        if self.avg_connection_latency_ms > 0.0 {
            summary.push_str("║                                                                               ║\n");
            summary.push_str("║ Connection Latency:                                                           ║\n");
            summary.push_str(&format!(
                "║   • Average: {:<62} ║\n",
                format!("{:.2}ms", self.avg_connection_latency_ms)
            ));
            summary.push_str(&format!(
                "║   • Last: {:<65} ║\n",
                format!("{:.2}ms", self.last_connection_latency_ms)
            ));
        }

        // Uptime Information
        summary.push_str(
            "║                                                                               ║\n",
        );
        summary.push_str(
            "║ Uptime Information:                                                           ║\n",
        );
        summary.push_str(&format!(
            "║   • Total Uptime: {:<57} ║\n",
            Self::format_duration(self.total_uptime_seconds)
        ));

        if self.is_connected {
            summary.push_str(&format!(
                "║   • Current Connection: {:<51} ║\n",
                Self::format_duration(self.current_uptime_seconds)
            ));
        }

        if self.time_since_last_disconnection_seconds > 0.0 {
            summary.push_str(&format!(
                "║   • Since Last Disconnect: {:<46} ║\n",
                Self::format_duration(self.time_since_last_disconnection_seconds)
            ));
        }

        // Message Statistics
        summary.push_str(
            "║                                                                               ║\n",
        );
        summary.push_str(
            "║ Message Statistics:                                                           ║\n",
        );
        summary.push_str(&format!(
            "║   • Messages Sent: {:<56} ║\n",
            format!(
                "{} ({:.2}/s)",
                self.messages_sent, self.avg_messages_sent_per_second
            )
        ));
        summary.push_str(&format!(
            "║   • Messages Received: {:<52} ║\n",
            format!(
                "{} ({:.2}/s)",
                self.messages_received, self.avg_messages_received_per_second
            )
        ));

        // Data Transfer Statistics
        summary.push_str(
            "║                                                                               ║\n",
        );
        summary.push_str(
            "║ Data Transfer:                                                                ║\n",
        );
        summary.push_str(&format!(
            "║   • Bytes Sent: {:<59} ║\n",
            format!(
                "{} ({}/s)",
                Self::format_bytes(self.bytes_sent),
                Self::format_bytes(self.avg_bytes_sent_per_second as u64)
            )
        ));
        summary.push_str(&format!(
            "║   • Bytes Received: {:<55} ║\n",
            format!(
                "{} ({}/s)",
                Self::format_bytes(self.bytes_received),
                Self::format_bytes(self.avg_bytes_received_per_second as u64)
            )
        ));

        // Recent Activity
        if !self.connection_history.is_empty() {
            summary.push_str("║                                                                               ║\n");
            summary.push_str("║ Recent Activity (Last 5 events):                                             ║\n");

            let recent_events: Vec<&ConnectionEvent> =
                self.connection_history.iter().rev().take(5).collect();
            for event in recent_events.iter().rev() {
                let timestamp = Self::format_timestamp(event.timestamp);
                let event_desc = Self::format_event_description(event);
                summary.push_str(&format!("║   • {timestamp}: {event_desc:<51} ║\n"));
            }
        }

        // Connection Health Assessment
        summary.push_str(
            "║                                                                               ║\n",
        );
        summary.push_str(
            "║ Connection Health:                                                            ║\n",
        );
        let health_status = self.assess_connection_health();
        summary.push_str(&format!("║   • Overall Health: {health_status:<55} ║\n"));

        // Performance Metrics
        if self.total_uptime_seconds > 0.0 {
            let stability = (self.total_uptime_seconds
                / (self.total_uptime_seconds + (self.disconnections as f64 * 5.0)))
                * 100.0;
            summary.push_str(&format!(
                "║   • Stability Score: {:<54} ║\n",
                format!("{:.1}%", stability)
            ));
        }

        // Footer
        summary.push_str(
            "╚═══════════════════════════════════════════════════════════════════════════════╝\n",
        );

        summary
    }

    /// Generate a compact, single-line summary
    pub fn compact_summary(&self) -> String {
        let status = if self.is_connected {
            "CONNECTED"
        } else {
            "DISCONNECTED"
        };
        let success_rate = if self.connection_attempts > 0 {
            (self.successful_connections as f64 / self.connection_attempts as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "Status: {} | Attempts: {} | Success Rate: {:.1}% | Uptime: {} | Messages: {}↑ {}↓ | Data: {}↑ {}↓",
            status,
            self.connection_attempts,
            success_rate,
            Self::format_duration(self.total_uptime_seconds),
            self.messages_sent,
            self.messages_received,
            Self::format_bytes(self.bytes_sent),
            Self::format_bytes(self.bytes_received)
        )
    }

    /// Assess the overall health of the connection
    fn assess_connection_health(&self) -> String {
        let mut health_score = 100.0;
        let mut issues = Vec::new();

        // Check success rate
        if self.connection_attempts > 0 {
            let success_rate =
                (self.successful_connections as f64 / self.connection_attempts as f64) * 100.0;
            if success_rate < 50.0 {
                health_score -= 40.0;
                issues.push("Low success rate");
            } else if success_rate < 80.0 {
                health_score -= 20.0;
                issues.push("Moderate success rate");
            }
        }

        // Check disconnection frequency
        if self.disconnections > 0 && self.total_uptime_seconds > 0.0 {
            let disconnections_per_hour =
                (self.disconnections as f64) / (self.total_uptime_seconds / 3600.0);
            if disconnections_per_hour > 5.0 {
                health_score -= 30.0;
                issues.push("Frequent disconnections");
            } else if disconnections_per_hour > 2.0 {
                health_score -= 15.0;
                issues.push("Occasional disconnections");
            }
        }

        // Check connection latency
        if self.avg_connection_latency_ms > 5000.0 {
            health_score -= 20.0;
            issues.push("High latency");
        } else if self.avg_connection_latency_ms > 2000.0 {
            health_score -= 10.0;
            issues.push("Moderate latency");
        }

        // Check if currently connected
        if !self.is_connected {
            health_score -= 25.0;
            issues.push("Currently disconnected");
        }

        let health_level = if health_score >= 90.0 {
            "🟢 Excellent"
        } else if health_score >= 70.0 {
            "🟡 Good"
        } else if health_score >= 50.0 {
            "🟠 Fair"
        } else {
            "🔴 Poor"
        };

        if issues.is_empty() {
            format!("{health_level} ({health_score:.0}/100)")
        } else {
            format!(
                "{} ({:.0}/100) - {}",
                health_level,
                health_score,
                issues.join(", ")
            )
        }
    }

    /// Format duration in a human-readable way
    fn format_duration(seconds: f64) -> String {
        if seconds < 60.0 {
            format!("{seconds:.1}s")
        } else if seconds < 3600.0 {
            format!("{:.1}m", seconds / 60.0)
        } else if seconds < 86400.0 {
            format!("{:.1}h", seconds / 3600.0)
        } else {
            format!("{:.1}d", seconds / 86400.0)
        }
    }

    /// Format bytes in a human-readable way
    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Format timestamp in a readable way
    fn format_timestamp(timestamp: u64) -> String {
        // Convert Unix timestamp to readable format
        let duration = std::time::Duration::from_millis(timestamp);
        let secs = duration.as_secs();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let diff = now.saturating_sub(secs);

        if diff < 60 {
            format!("{diff}s ago")
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }

    /// Format event description
    fn format_event_description(event: &ConnectionEvent) -> String {
        match &event.event_type {
            ConnectionEventType::ConnectionAttempt => "Connection attempt".to_string(),
            ConnectionEventType::ConnectionSuccess => {
                if let Some(duration) = event.duration_ms {
                    format!("Connected ({duration}ms)")
                } else {
                    "Connected".to_string()
                }
            }
            ConnectionEventType::ConnectionFailure => {
                if let Some(reason) = &event.reason {
                    format!("Connection failed: {reason}")
                } else {
                    "Connection failed".to_string()
                }
            }
            ConnectionEventType::Disconnection => {
                if let Some(reason) = &event.reason {
                    format!("Disconnected: {reason}")
                } else {
                    "Disconnected".to_string()
                }
            }
            ConnectionEventType::Reconnection => "Reconnection attempt".to_string(),
            ConnectionEventType::MessageSent => "Message sent".to_string(),
            ConnectionEventType::MessageReceived => "Message received".to_string(),
        }
    }
}

impl StatisticsTracker {
    pub fn new() -> Self {
        Self {
            connection_attempts: AtomicU64::new(0),
            successful_connections: AtomicU64::new(0),
            failed_connections: AtomicU64::new(0),
            disconnections: AtomicU64::new(0),
            reconnections: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            start_time: Instant::now(),
            last_connection_attempt: RwLock::new(None),
            current_connection_start: RwLock::new(None),
            last_disconnection: RwLock::new(None),
            total_uptime: RwLock::new(Duration::ZERO),
            connection_latencies: RwLock::new(Vec::new()),
            is_connected: AtomicBool::new(false),
            event_history: RwLock::new(Vec::new()),
        }
    }

    pub async fn record_connection_attempt(&self) {
        self.connection_attempts.fetch_add(1, Ordering::SeqCst);
        *self.last_connection_attempt.write().await = Some(Instant::now());

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::ConnectionAttempt,
            timestamp: Self::current_timestamp(),
            duration_ms: None,
            reason: None,
        })
        .await;
    }

    pub async fn record_connection_success(&self) {
        self.successful_connections.fetch_add(1, Ordering::SeqCst);
        self.is_connected.store(true, Ordering::SeqCst);

        let now = Instant::now();
        *self.current_connection_start.write().await = Some(now);

        // Calculate connection latency
        let latency = if let Some(attempt_time) = *self.last_connection_attempt.read().await {
            now.duration_since(attempt_time)
        } else {
            Duration::ZERO
        };

        self.connection_latencies.write().await.push(latency);

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::ConnectionSuccess,
            timestamp: Self::current_timestamp(),
            duration_ms: Some(latency.as_millis() as u64),
            reason: None,
        })
        .await;
    }

    pub async fn record_connection_failure(&self, reason: Option<String>) {
        self.failed_connections.fetch_add(1, Ordering::SeqCst);
        self.is_connected.store(false, Ordering::SeqCst);

        let latency = (*self.last_connection_attempt.read().await)
            .map(|attempt_time| Instant::now().duration_since(attempt_time));

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::ConnectionFailure,
            timestamp: Self::current_timestamp(),
            duration_ms: latency.map(|d| d.as_millis() as u64),
            reason,
        })
        .await;
    }

    pub async fn record_disconnection(&self, reason: Option<String>) {
        self.disconnections.fetch_add(1, Ordering::SeqCst);
        self.is_connected.store(false, Ordering::SeqCst);

        let now = Instant::now();
        *self.last_disconnection.write().await = Some(now);

        // Update total uptime
        if let Some(connection_start) = *self.current_connection_start.read().await {
            let uptime = now.duration_since(connection_start);
            *self.total_uptime.write().await += uptime;
        }

        *self.current_connection_start.write().await = None;

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::Disconnection,
            timestamp: Self::current_timestamp(),
            duration_ms: None,
            reason,
        })
        .await;
    }

    pub async fn record_reconnection(&self) {
        self.reconnections.fetch_add(1, Ordering::SeqCst);

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::Reconnection,
            timestamp: Self::current_timestamp(),
            duration_ms: None,
            reason: None,
        })
        .await;
    }

    pub async fn record_message_sent(&self, message: &Message) {
        self.messages_sent.fetch_add(1, Ordering::SeqCst);
        self.bytes_sent
            .fetch_add(Self::message_size(message), Ordering::SeqCst);

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::MessageSent,
            timestamp: Self::current_timestamp(),
            duration_ms: None,
            reason: None,
        })
        .await;
    }

    pub async fn record_message_received(&self, message: &Message) {
        self.messages_received.fetch_add(1, Ordering::SeqCst);
        self.bytes_received
            .fetch_add(Self::message_size(message), Ordering::SeqCst);

        self.add_event(ConnectionEvent {
            event_type: ConnectionEventType::MessageReceived,
            timestamp: Self::current_timestamp(),
            duration_ms: None,
            reason: None,
        })
        .await;
    }

    pub async fn get_stats(&self) -> ConnectionStats {
        let now = Instant::now();
        let elapsed = now.duration_since(self.start_time);

        let connection_latencies = self.connection_latencies.read().await;
        let avg_latency = if connection_latencies.is_empty() {
            0.0
        } else {
            connection_latencies.iter().sum::<Duration>().as_millis() as f64
                / connection_latencies.len() as f64
        };

        let last_latency = connection_latencies
            .last()
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);

        let total_uptime = *self.total_uptime.read().await;
        let current_uptime =
            if let Some(connection_start) = *self.current_connection_start.read().await {
                now.duration_since(connection_start)
            } else {
                Duration::ZERO
            };

        let time_since_last_disconnection =
            if let Some(last_disc) = *self.last_disconnection.read().await {
                now.duration_since(last_disc)
            } else {
                elapsed
            };

        let messages_sent = self.messages_sent.load(Ordering::SeqCst);
        let messages_received = self.messages_received.load(Ordering::SeqCst);
        let bytes_sent = self.bytes_sent.load(Ordering::SeqCst);
        let bytes_received = self.bytes_received.load(Ordering::SeqCst);

        let elapsed_seconds = elapsed.as_secs_f64();

        ConnectionStats {
            connection_attempts: self.connection_attempts.load(Ordering::SeqCst),
            successful_connections: self.successful_connections.load(Ordering::SeqCst),
            failed_connections: self.failed_connections.load(Ordering::SeqCst),
            disconnections: self.disconnections.load(Ordering::SeqCst),
            reconnections: self.reconnections.load(Ordering::SeqCst),
            avg_connection_latency_ms: avg_latency,
            last_connection_latency_ms: last_latency,
            total_uptime_seconds: total_uptime.as_secs_f64(),
            current_uptime_seconds: current_uptime.as_secs_f64(),
            time_since_last_disconnection_seconds: time_since_last_disconnection.as_secs_f64(),
            messages_sent,
            messages_received,
            bytes_sent,
            bytes_received,
            avg_messages_sent_per_second: if elapsed_seconds > 0.0 {
                messages_sent as f64 / elapsed_seconds
            } else {
                0.0
            },
            avg_messages_received_per_second: if elapsed_seconds > 0.0 {
                messages_received as f64 / elapsed_seconds
            } else {
                0.0
            },
            avg_bytes_sent_per_second: if elapsed_seconds > 0.0 {
                bytes_sent as f64 / elapsed_seconds
            } else {
                0.0
            },
            avg_bytes_received_per_second: if elapsed_seconds > 0.0 {
                bytes_received as f64 / elapsed_seconds
            } else {
                0.0
            },
            is_connected: self.is_connected.load(Ordering::SeqCst),
            connection_history: self.event_history.read().await.clone(),
        }
    }

    fn message_size(message: &Message) -> u64 {
        match message {
            Message::Text(text) => text.len() as u64,
            Message::Binary(data) => data.len() as u64,
            Message::Ping(data) => data.len() as u64,
            Message::Pong(data) => data.len() as u64,
            Message::Close(_) => 0,
            Message::Frame(_) => 0,
        }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    async fn add_event(&self, event: ConnectionEvent) {
        let mut history = self.event_history.write().await;
        history.push(event);

        // Keep only last 100 events to prevent memory growth
        if history.len() > 100 {
            history.drain(0..50); // Remove oldest 50 events
        }
    }
}

impl Default for StatisticsTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper around AsyncSender to track message statistics
pub struct TrackedSender<T> {
    inner: AsyncSender<T>,
    stats: Arc<StatisticsTracker>,
}

impl<T> TrackedSender<T> {
    pub fn new(sender: AsyncSender<T>, stats: Arc<StatisticsTracker>) -> Self {
        Self {
            inner: sender,
            stats,
        }
    }

    pub async fn send(&self, item: T) -> Result<(), kanal::SendError> {
        let result = self.inner.send(item).await;

        // We'll track all sends for now, regardless of type
        if result.is_ok() {
            // Use tokio::spawn for async operation
            let stats = self.stats.clone();
            tokio::spawn(async move {
                // For now, we'll just track the count without message details
                // In a real implementation, you might want to have a trait for message sizing
                stats
                    .messages_sent
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            });
        }

        result
    }
}

/// Wrapper around AsyncReceiver to track message statistics
pub struct TrackedReceiver<T> {
    inner: AsyncReceiver<T>,
    stats: Arc<StatisticsTracker>,
}

impl<T> TrackedReceiver<T> {
    pub fn new(receiver: AsyncReceiver<T>, stats: Arc<StatisticsTracker>) -> Self {
        Self {
            inner: receiver,
            stats,
        }
    }

    pub async fn recv(&self) -> Result<T, kanal::ReceiveError> {
        let result = self.inner.recv().await;

        // We'll track all receives for now, regardless of type
        if result.is_ok() {
            // Use tokio::spawn for async operation
            let stats = self.stats.clone();
            tokio::spawn(async move {
                // For now, we'll just track the count without message details
                // In a real implementation, you might want to have a trait for message sizing
                stats
                    .messages_received
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            });
        }

        result
    }
}
