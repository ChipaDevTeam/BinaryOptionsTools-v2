use async_trait::async_trait;
use binary_options_tools_core_pre::builder::ClientBuilder;
use binary_options_tools_core_pre::connector::{Connector, ConnectorResult, WsStream};
use binary_options_tools_core_pre::error::CoreResult;
use binary_options_tools_core_pre::testing::{
    TestingConfig, TestingWrapper, TestingWrapperBuilder,
};
use binary_options_tools_core_pre::traits::{ApiModule, Rule, RunnerCommand};
use kanal::{AsyncReceiver, AsyncSender};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

// Mock connector for testing
struct MockConnector;

#[async_trait]
impl Connector<()> for MockConnector {
    async fn connect(&self, _: Arc<()>) -> ConnectorResult<WsStream> {
        // This is a mock implementation, it would fail in real usage
        // but it's sufficient for testing the wrapper structure
        Err(
            binary_options_tools_core_pre::connector::ConnectorError::Custom(
                "Mock connector".to_string(),
            ),
        )
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        Ok(())
    }
}

// Simple test module
pub struct TestModule {
    _msg_rx: AsyncReceiver<Arc<Message>>,
}

#[async_trait]
impl ApiModule<()> for TestModule {
    type Command = String;
    type CommandResponse = String;
    type Handle = TestHandle;

    fn new(
        _state: Arc<()>,
        _cmd_rx: AsyncReceiver<Self::Command>,
        _cmd_ret_tx: AsyncSender<Self::CommandResponse>,
        msg_rx: AsyncReceiver<Arc<Message>>,
        _to_ws: AsyncSender<Message>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self { _msg_rx: msg_rx }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        TestHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        // Mock implementation that never actually runs
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    fn rule(_: Arc<()>) -> Box<dyn Rule + Send + Sync> {
        Box::new(move |_msg: &Message| false) // This rule never matches
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct TestHandle {
    sender: AsyncSender<String>,
    receiver: AsyncReceiver<String>,
}

#[tokio::test]
async fn test_testing_wrapper_creation() {
    let connector = MockConnector;
    let (client, runner) = ClientBuilder::new(connector, ())
        .with_module::<TestModule>()
        .build()
        .await
        .expect("Failed to build client");

    let config = TestingConfig {
        stats_interval: Duration::from_secs(1),
        log_stats: false, // Don't log during tests
        track_events: true,
        max_reconnect_attempts: Some(1),
        reconnect_delay: Duration::from_secs(1),
        connection_timeout: Duration::from_secs(5),
        auto_reconnect: false,
    };

    let wrapper = TestingWrapper::new(client, runner, config);

    // Test that we can get initial statistics
    let stats = wrapper.get_stats().await;
    assert_eq!(stats.connection_attempts, 0);
    assert_eq!(stats.successful_connections, 0);
    assert_eq!(stats.messages_sent, 0);
    assert_eq!(stats.messages_received, 0);
    assert!(!stats.is_connected);
}

#[tokio::test]
async fn test_testing_wrapper_builder() {
    let connector = MockConnector;
    let (client, runner) = ClientBuilder::new(connector, ())
        .with_module::<TestModule>()
        .build()
        .await
        .expect("Failed to build client");

    let wrapper = TestingWrapperBuilder::new()
        .with_stats_interval(Duration::from_secs(5))
        .with_log_stats(false)
        .with_track_events(true)
        .with_max_reconnect_attempts(Some(3))
        .with_reconnect_delay(Duration::from_secs(2))
        .with_connection_timeout(Duration::from_secs(10))
        .with_auto_reconnect(false)
        .build(client, runner);

    // Test that we can get initial statistics
    let stats = wrapper.get_stats().await;
    assert_eq!(stats.connection_attempts, 0);
    assert_eq!(stats.successful_connections, 0);
}

#[tokio::test]
async fn test_testing_wrapper_with_runner() {
    let connector = MockConnector;
    let (client, runner) = ClientBuilder::new(connector, ())
        .with_module::<TestModule>()
        .build()
        .await
        .expect("Failed to build client");

    let config = TestingConfig {
        stats_interval: Duration::from_millis(100), // Very short interval for testing
        log_stats: false,                           // Don't log during tests
        track_events: true,
        max_reconnect_attempts: Some(1),
        reconnect_delay: Duration::from_millis(100),
        connection_timeout: Duration::from_millis(500),
        auto_reconnect: false,
    };

    let mut wrapper = TestingWrapper::new(client, runner, config);

    // Test that we can start the wrapper
    // Note: This will fail to connect due to MockConnector, but that's expected for testing
    let start_result = wrapper.start().await;
    assert!(start_result.is_ok());

    // Give it a short time to attempt connection
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test that we can get statistics
    let stats = wrapper.get_stats().await;
    assert_eq!(stats.connection_attempts, 1);

    // Test shutdown
    let shutdown_result = wrapper.stop().await;
    assert!(shutdown_result.is_ok());
}

#[tokio::test]
async fn test_statistics_export() {
    let connector = MockConnector;
    let (client, runner) = ClientBuilder::new(connector, ())
        .with_module::<TestModule>()
        .build()
        .await
        .expect("Failed to build client");

    let wrapper = TestingWrapper::new(client, runner, TestingConfig::default());

    // Test JSON export
    let json_result = wrapper.export_stats_json().await;
    assert!(json_result.is_ok());
    let json_stats = json_result.unwrap();
    assert!(json_stats.contains("connection_attempts"));
    assert!(json_stats.contains("successful_connections"));

    // Test CSV export
    let csv_result = wrapper.export_stats_csv().await;
    assert!(csv_result.is_ok());
    let csv_stats = csv_result.unwrap();
    assert!(csv_stats.contains("connection_attempts"));
    assert!(csv_stats.contains("successful_connections"));
}
