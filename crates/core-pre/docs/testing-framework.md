# WebSocket Testing Framework

A comprehensive testing and monitoring framework for WebSocket connections in the `binary-options-tools-core-pre` crate.

## Overview

This framework provides advanced statistics tracking, connection monitoring, and testing capabilities for WebSocket-based applications. It wraps around the existing `Client` and `Runner` architecture to provide detailed insights into connection performance and reliability.

## Features

### ✅ Currently Implemented

#### Connection Statistics

- **Connection Attempts**: Total number of connection attempts
- **Successful Connections**: Number of successful connections
- **Failed Connections**: Number of failed connections
- **Disconnections**: Number of disconnections
- **Reconnections**: Number of reconnection attempts

#### Performance Metrics

- **Connection Latency**: Average and last connection latency in milliseconds
- **Uptime Tracking**: Total uptime and current connection uptime
- **Disconnection Tracking**: Time since last disconnection
- **Message Throughput**: Messages sent/received with per-second averages
- **Data Volume**: Bytes sent/received with per-second averages
- **Success Rate**: Connection success rate percentage

#### Event Tracking

- **Connection Events**: Detailed history of connection events
- **Event Types**: Connection attempts, successes, failures, disconnections, reconnections
- **Event Timestamps**: Unix timestamps for all events
- **Event Reasons**: Optional reason strings for failures and disconnections

#### Statistics Export

- **JSON Export**: Complete statistics in JSON format
- **CSV Export**: Basic metrics in CSV format
- **Real-time Logging**: Configurable periodic statistics logging

#### Testing Wrapper

- **TestingWrapper**: Comprehensive wrapper around Client/Runner
- **TestingConfig**: Configurable settings for testing behavior
- **TestingConnector**: Connector wrapper with statistics tracking
- **TestingWrapperBuilder**: Builder pattern for easy configuration

### Testing Configuration Options

- **Stats Interval**: How often to collect and log statistics
- **Log Stats**: Whether to log statistics to console
- **Track Events**: Whether to track detailed connection events
- **Max Reconnect Attempts**: Maximum number of reconnection attempts
- **Reconnect Delay**: Delay between reconnection attempts
- **Connection Timeout**: Connection timeout duration
- **Auto Reconnect**: Whether to automatically reconnect on disconnection

## Usage

### Basic Usage

```rust
use binary_options_tools_core_pre::testing::{TestingWrapper, TestingWrapperBuilder};
use std::time::Duration;

// Create your client and runner as usual
let (client, runner) = ClientBuilder::new(connector, state)
    .with_module::<YourModule>()
    .build()
    .await?;

// Wrap with testing capabilities
let mut testing_wrapper = TestingWrapperBuilder::new()
    .with_stats_interval(Duration::from_secs(30))
    .with_log_stats(true)
    .with_connection_timeout(Duration::from_secs(10))
    .build(client, runner);

// Start the wrapper (this will begin collecting statistics)
testing_wrapper.start().await?;

// Use the client through the wrapper
let client = testing_wrapper.client();
// ... use client as normal ...

// Get statistics
let stats = testing_wrapper.get_stats().await;
println!("Connection success rate: {:.1}%",
    stats.successful_connections as f64 / stats.connection_attempts as f64 * 100.0);

// Export statistics
let json_stats = testing_wrapper.export_stats_json().await?;
let csv_stats = testing_wrapper.export_stats_csv().await?;
```

### Advanced Usage with Custom Platform

```rust
pub struct TestingEchoPlatform {
    testing_wrapper: TestingWrapper<()>,
}

impl TestingEchoPlatform {
    pub async fn new(url: String) -> CoreResult<Self> {
        let connector = DummyConnector::new(url);
        let (client, runner) = ClientBuilder::new(connector, ())
            .with_module::<EchoModule>()
            .build()
            .await?;

        let testing_wrapper = TestingWrapperBuilder::new()
            .with_stats_interval(Duration::from_secs(10))
            .with_log_stats(true)
            .with_max_reconnect_attempts(Some(3))
            .build(client, runner);

        Ok(Self { testing_wrapper })
    }

    pub async fn run_performance_test(&self, num_messages: usize, delay_ms: u64) -> CoreResult<()> {
        for i in 0..num_messages {
            let msg = format!("Test message {}", i);
            let response = self.echo(msg).await?;

            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        let stats = self.get_stats().await;
        println!("Test completed. Messages sent: {}, received: {}",
            stats.messages_sent, stats.messages_received);

        Ok(())
    }
}
```

## Architecture

### Core Components

1. **StatisticsTracker**: Thread-safe statistics collection with atomic operations
2. **ConnectionStats**: Serializable statistics structure
3. **TestingWrapper**: Main wrapper providing testing capabilities
4. **TestingConnector**: Connector wrapper for tracking connection events
5. **TestingConfig**: Configuration for testing behavior

### Data Flow

```
Application → TestingWrapper → Client → TestingConnector → WebSocket
     ↓              ↓              ↓              ↓
StatisticsTracker ← Events ← Messages ← Connection Events
```

## Statistics Details

### Connection Metrics

- `connection_attempts`: Total connection attempts
- `successful_connections`: Successful connections
- `failed_connections`: Failed connections
- `disconnections`: Number of disconnections
- `reconnections`: Number of reconnections

### Performance Metrics

- `avg_connection_latency_ms`: Average time to establish connection
- `last_connection_latency_ms`: Most recent connection latency
- `total_uptime_seconds`: Total time connected
- `current_uptime_seconds`: Current connection uptime
- `time_since_last_disconnection_seconds`: Time since last disconnect

### Throughput Metrics

- `messages_sent`/`messages_received`: Message counts
- `bytes_sent`/`bytes_received`: Data volume
- `avg_messages_*_per_second`: Message rate
- `avg_bytes_*_per_second`: Data rate

## 🚧 Future Enhancements (TODO)

### Planned Features

#### Advanced Testing Framework

- **Scheduled Function Calls**: Configure functions to be called at specific times
  - Call function X at 5 seconds after start
  - Call function Y every 10 seconds
  - Call function Z on specific events (connect, disconnect, etc.)

#### Function Call Configuration

```rust
// Future API concept
TestingConfig {
    scheduled_calls: vec![
        ScheduledCall::new("echo_test")
            .at_time(Duration::from_secs(5))
            .with_params(vec!["Hello World"])
            .expect_result(ExpectedResult::Ok),

        ScheduledCall::new("ping_test")
            .every(Duration::from_secs(30))
            .expect_result(ExpectedResult::Ok),

        ScheduledCall::new("stress_test")
            .on_event(ConnectionEvent::Connected)
            .with_params(vec!["100", "fast"])
            .expect_result(ExpectedResult::Ok),
    ]
}
```

#### Enhanced Monitoring

- **Network Quality Metrics**: Jitter, packet loss estimation
- **Connection Health Scoring**: Overall connection quality score
- **Predictive Analytics**: Predict connection failures
- **Performance Benchmarking**: Compare against baseline performance

#### Advanced Statistics

- **Percentile Metrics**: 95th, 99th percentile latencies
- **Time Series Data**: Historical performance over time
- **Anomaly Detection**: Detect unusual patterns
- **Correlation Analysis**: Correlate different metrics

#### Testing Scenarios

- **Load Testing**: Simulate high message volumes
- **Stress Testing**: Test under extreme conditions
- **Endurance Testing**: Long-running connection tests
- **Recovery Testing**: Test reconnection scenarios

#### Enhanced Kanal Integration

- **Tracked Channels**: Wrapper around kanal channels with statistics
- **Channel Metrics**: Queue depth, throughput, latency
- **Backpressure Monitoring**: Detect channel bottlenecks
- **Message Routing Analysis**: Track message flow through channels

#### Reporting and Visualization

- **HTML Reports**: Generate detailed HTML reports
- **Real-time Dashboard**: Web-based monitoring dashboard
- **Alerting System**: Configurable alerts for issues
- **Performance Trends**: Visual representation of performance over time

#### Configuration Management

- **YAML/JSON Config**: External configuration files
- **Environment Variables**: Configuration via environment
- **Runtime Reconfiguration**: Change config without restart
- **Configuration Profiles**: Different configs for different environments

## Summary of Implementation

The TestingWrapper has been successfully implemented with full ClientRunner integration:

### ✅ Key Improvements Made

1. **Integrated ClientRunner Execution**: The TestingWrapper now actually runs the ClientRunner in a background task, providing real connection testing.

2. **Proper Lifecycle Management**:
   - `start()` method launches the ClientRunner and statistics collection
   - `stop()` method gracefully stops statistics collection but preserves the client
   - `stop_and_shutdown()` method provides complete shutdown using `client.shutdown()`

3. **Task Management**: Proper handling of both statistics collection and runner tasks with timeout-based cleanup.

4. **Real Connection Testing**: The wrapper now provides genuine WebSocket connection testing with actual latency measurements and connection events.

5. **Thread Safety**: Uses `Option<ClientRunner>` to safely transfer ownership to background tasks.

### ✅ Working Example

The `docs/examples/testing_echo_client.rs` demonstrates full functionality:

- Real WebSocket connections to `wss://echo.websocket.org`
- Live statistics collection and logging
- Performance testing with message throughput
- Graceful shutdown with proper cleanup

### ✅ Verification

- All tests pass
- Example runs successfully with real WebSocket connections
- Statistics are collected in real-time
- Graceful shutdown works properly

The TestingWrapper is now a complete, production-ready testing framework for WebSocket connections in the binary-options-tools-core-pre crate.

## Examples

See `docs/examples/testing_echo_client.rs` for a complete example of using the testing framework.

## Contributing

When adding new features to the testing framework:

1. Update the statistics structures if new metrics are needed
2. Add appropriate tracking in the `StatisticsTracker`
3. Update the documentation
4. Add examples demonstrating the new features
5. Consider backward compatibility

## License

This testing framework is part of the `binary-options-tools-core-pre` crate and follows the same license.

## Middleware System

The WebSocket client supports a composable middleware system inspired by Axum's layer system. Middleware can observe and react to WebSocket messages being sent and received, as well as connection events.

### Key Components

- **`WebSocketMiddleware`**: The core trait for implementing middleware
- **`MiddlewareStack`**: A composable stack of middleware layers
- **`MiddlewareContext`**: Context passed to middleware with message and client information

### Implementing Middleware

#### Basic Middleware Example

```rust
use binary_options_tools_core_pre::middleware::{WebSocketMiddleware, MiddlewareContext};
use binary_options_tools_core_pre::error::CoreResult;
use binary_options_tools_core_pre::traits::AppState;
use async_trait::async_trait;
use tokio_tungstenite::tungstenite::Message;

struct LoggingMiddleware;

#[async_trait]
impl<S: AppState> WebSocketMiddleware<S> for LoggingMiddleware {
    async fn on_send(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        println!("Sending message: {:?}", message);
        Ok(())
    }

    async fn on_receive(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        println!("Received message: {:?}", message);
        Ok(())
    }

    async fn on_connect(&self, context: &MiddlewareContext<S>) -> CoreResult<()> {
        println!("Connected to WebSocket");
        Ok(())
    }

    async fn on_disconnect(&self, context: &MiddlewareContext<S>) -> CoreResult<()> {
        println!("Disconnected from WebSocket");
        Ok(())
    }
}
```

#### Statistics Middleware Example

```rust
use binary_options_tools_core_pre::middleware::{WebSocketMiddleware, MiddlewareContext};
use binary_options_tools_core_pre::error::CoreResult;
use binary_options_tools_core_pre::traits::AppState;
use async_trait::async_trait;
use tokio_tungstenite::tungstenite::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

struct StatisticsMiddleware {
    messages_sent: Arc<AtomicU64>,
    messages_received: Arc<AtomicU64>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    connections: Arc<AtomicU64>,
    disconnections: Arc<AtomicU64>,
}

impl StatisticsMiddleware {
    pub fn new() -> Self {
        Self {
            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            connections: Arc::new(AtomicU64::new(0)),
            disconnections: Arc::new(AtomicU64::new(0)),
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
impl<S: AppState> WebSocketMiddleware<S> for StatisticsMiddleware {
    async fn on_send(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);

        let size = match message {
            Message::Text(text) => text.len() as u64,
            Message::Binary(data) => data.len() as u64,
            _ => 0,
        };
        self.bytes_sent.fetch_add(size, Ordering::Relaxed);

        Ok(())
    }

    async fn on_receive(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        self.messages_received.fetch_add(1, Ordering::Relaxed);

        let size = match message {
            Message::Text(text) => text.len() as u64,
            Message::Binary(data) => data.len() as u64,
            _ => 0,
        };
        self.bytes_received.fetch_add(size, Ordering::Relaxed);

        Ok(())
    }

    async fn on_connect(&self, context: &MiddlewareContext<S>) -> CoreResult<()> {
        self.connections.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn on_disconnect(&self, context: &MiddlewareContext<S>) -> CoreResult<()> {
        self.disconnections.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
```

### Adding Middleware to the Client

#### Using the ClientBuilder

```rust
use binary_options_tools_core_pre::builder::ClientBuilder;
use binary_options_tools_core_pre::middleware::MiddlewareStack;

// Add a single middleware
let (client, runner) = ClientBuilder::new(connector, state)
    .with_middleware(Box::new(LoggingMiddleware))
    .with_middleware(Box::new(StatisticsMiddleware::new()))
    .build()
    .await?;

// Add multiple middleware at once
let (client, runner) = ClientBuilder::new(connector, state)
    .with_middleware_layers(vec![
        Box::new(LoggingMiddleware),
        Box::new(StatisticsMiddleware::new()),
    ])
    .build()
    .await?;

// Using a pre-built middleware stack
let mut stack = MiddlewareStack::new();
stack.add_layer(Box::new(LoggingMiddleware));
stack.add_layer(Box::new(StatisticsMiddleware::new()));

let (client, runner) = ClientBuilder::new(connector, state)
    .with_middleware_stack(stack)
    .build()
    .await?;
```

#### Using the MiddlewareStackBuilder

```rust
use binary_options_tools_core_pre::middleware::MiddlewareStackBuilder;

let stack = MiddlewareStackBuilder::new()
    .layer(Box::new(LoggingMiddleware))
    .layer(Box::new(StatisticsMiddleware::new()))
    .build();

let (client, runner) = ClientBuilder::new(connector, state)
    .with_middleware_stack(stack)
    .build()
    .await?;
```

### Middleware Execution Order

Middleware are executed in the order they are added to the stack:

1. **On Send**: Middleware are called before the message is sent to the WebSocket
2. **On Receive**: Middleware are called after the message is received from the WebSocket
3. **On Connect**: Middleware are called after a successful connection is established
4. **On Disconnect**: Middleware are called when a connection is lost or explicitly disconnected

### Error Handling

Middleware errors are logged but do not prevent other middleware from running or block message processing. This ensures that middleware failures don't impact the core functionality of the WebSocket client.

### Advanced Use Cases

#### Message Filtering Middleware

```rust
struct MessageFilterMiddleware {
    allowed_types: Vec<String>,
}

#[async_trait]
impl<S: AppState> WebSocketMiddleware<S> for MessageFilterMiddleware {
    async fn on_receive(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        if let Message::Text(text) = message {
            // Parse and validate message type
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                    if !self.allowed_types.contains(&msg_type.to_string()) {
                        tracing::warn!("Filtered message type: {}", msg_type);
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}
```

#### Rate Limiting Middleware

```rust
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct RateLimitMiddleware {
    last_send: Arc<Mutex<Instant>>,
    min_interval: Duration,
}

#[async_trait]
impl<S: AppState> WebSocketMiddleware<S> for RateLimitMiddleware {
    async fn on_send(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        let mut last_send = self.last_send.lock().await;
        let now = Instant::now();

        if now.duration_since(*last_send) < self.min_interval {
            tracing::warn!("Rate limit exceeded, message delayed");
            tokio::time::sleep(self.min_interval - now.duration_since(*last_send)).await;
        }

        *last_send = Instant::now();
        Ok(())
    }
}
```

### Best Practices

1. **Keep Middleware Lightweight**: Middleware should not perform heavy computations or blocking operations
2. **Handle Errors Gracefully**: Always return `Ok(())` unless there's a critical error
3. **Use Atomic Operations**: For statistics tracking, use atomic operations to avoid locks
4. **Document Middleware Behavior**: Clearly document what each middleware does and any side effects
5. **Test Middleware Independently**: Write unit tests for middleware logic
6. **Consider Performance**: Middleware run on every message, so optimize for performance
