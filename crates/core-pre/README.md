# Binary Options Tools - Core Pre - Testing Framework

A comprehensive WebSocket testing and monitoring framework for the `binary-options-tools-core-pre` crate.

## Overview

This framework provides advanced statistics tracking, connection monitoring, and testing capabilities for WebSocket-based applications. It wraps around the existing `Client` and `ClientRunner` architecture to provide detailed insights into connection performance and reliability.

## Quick Start

### 1. Basic Usage

```rust
use binary_options_tools_core_pre::testing::{TestingWrapper, TestingWrapperBuilder};
use binary_options_tools_core_pre::builder::ClientBuilder;
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

// Start the wrapper (this will run the ClientRunner and begin collecting statistics)
testing_wrapper.start().await?;

// Use the client through the wrapper
let client = testing_wrapper.client();
// ... use client as normal ...

// Get statistics
let stats = testing_wrapper.get_stats().await;
println!("Connection success rate: {:.1}%",
    stats.successful_connections as f64 / stats.connection_attempts as f64 * 100.0);

// Stop the wrapper (graceful shutdown)
testing_wrapper.stop_and_shutdown().await?;
```

### 2. Run the Example

```bash
cargo run --example testing_echo_client
```

### 3. Run Tests

```bash
cargo test testing_wrapper
```

## Features

### ✅ Currently Implemented

- **Connection Statistics**: Track attempts, successes, failures, disconnections
- **Performance Metrics**: Latency, uptime, throughput measurements
- **Message Tracking**: Count and data volume of sent/received messages
- **Event History**: Detailed log of connection events with timestamps
- **Statistics Export**: JSON and CSV export formats
- **Real-time Monitoring**: Configurable periodic statistics logging
- **Testing Configuration**: Flexible configuration for different testing scenarios

### Statistics Collected

- Connection attempts, successes, failures, disconnections
- Average and last connection latency
- Total and current connection uptime
- Time since last disconnection
- Message counts and data volumes
- Throughput rates (messages/second, bytes/second)
- Connection success rate
- Event history with timestamps

### Configuration Options

- **Stats Interval**: How often to collect and log statistics
- **Log Stats**: Whether to log statistics to console
- **Track Events**: Whether to track detailed connection events
- **Max Reconnect Attempts**: Maximum number of reconnection attempts
- **Reconnect Delay**: Delay between reconnection attempts
- **Connection Timeout**: Connection timeout duration
- **Auto Reconnect**: Whether to automatically reconnect on disconnection

## API Reference

### TestingWrapper

The main wrapper class that provides testing capabilities:

```rust
pub struct TestingWrapper<S: AppState> {
    // Internal fields
}

impl<S: AppState> TestingWrapper<S> {
    pub async fn start(&mut self) -> CoreResult<()>
    pub async fn stop(&mut self) -> CoreResult<()>
    pub async fn stop_and_shutdown(self) -> CoreResult<()>
    pub async fn get_stats(&self) -> ConnectionStats
    pub async fn export_stats_json(&self) -> CoreResult<String>
    pub async fn export_stats_csv(&self) -> CoreResult<String>
    pub fn client(&self) -> &Client<S>
    pub fn client_mut(&mut self) -> &mut Client<S>
}
```

### TestingWrapperBuilder

Builder pattern for creating testing wrappers:

```rust
pub struct TestingWrapperBuilder<S: AppState> {
    // Internal fields
}

impl<S: AppState> TestingWrapperBuilder<S> {
    pub fn new() -> Self
    pub fn with_stats_interval(self, interval: Duration) -> Self
    pub fn with_log_stats(self, log_stats: bool) -> Self
    pub fn with_track_events(self, track_events: bool) -> Self
    pub fn with_max_reconnect_attempts(self, max_attempts: Option<u32>) -> Self
    pub fn with_reconnect_delay(self, delay: Duration) -> Self
    pub fn with_connection_timeout(self, timeout: Duration) -> Self
    pub fn with_auto_reconnect(self, auto_reconnect: bool) -> Self
    pub fn build(self, client: Client<S>, runner: ClientRunner<S>) -> TestingWrapper<S>
}
```

### ConnectionStats

Statistics structure with comprehensive metrics:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub connection_attempts: u64,
    pub successful_connections: u64,
    pub failed_connections: u64,
    pub disconnections: u64,
    pub reconnections: u64,
    pub avg_connection_latency_ms: f64,
    pub last_connection_latency_ms: f64,
    pub total_uptime_seconds: f64,
    pub current_uptime_seconds: f64,
    pub time_since_last_disconnection_seconds: f64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_messages_sent_per_second: f64,
    pub avg_messages_received_per_second: f64,
    pub avg_bytes_sent_per_second: f64,
    pub avg_bytes_received_per_second: f64,
    pub is_connected: bool,
    pub connection_history: Vec<ConnectionEvent>,
}
```

## Advanced Usage

### Creating a Custom Testing Platform

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

### Exporting Statistics

```rust
// Export to JSON
let json_stats = testing_wrapper.export_stats_json().await?;
println!("JSON Stats:\n{}", json_stats);

// Export to CSV
let csv_stats = testing_wrapper.export_stats_csv().await?;
println!("CSV Stats:\n{}", csv_stats);
```

## Examples

- `docs/examples/testing_echo_client.rs` - Complete example with performance testing
- `tests/testing_wrapper_tests.rs` - Unit tests demonstrating usage

## Future Enhancements

See `docs/testing-framework.md` for planned features including:

- Scheduled function calls
- Advanced monitoring capabilities
- Performance benchmarking
- Enhanced kanal integration
- Reporting and visualization
- Configuration management

## Contributing

When adding new features:

1. Update the statistics structures if new metrics are needed
2. Add appropriate tracking in the `StatisticsTracker`
3. Update the documentation
4. Add examples demonstrating new features
5. Consider backward compatibility

## License

This framework is part of the `binary-options-tools-core-pre` crate and follows the same license.
