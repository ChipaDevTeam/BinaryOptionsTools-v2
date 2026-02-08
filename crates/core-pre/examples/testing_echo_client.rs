use async_trait::async_trait;
use binary_options_tools_core_pre::builder::ClientBuilder;
use binary_options_tools_core_pre::connector::ConnectorResult;
use binary_options_tools_core_pre::connector::{Connector, WsStream};
use binary_options_tools_core_pre::error::{CoreError, CoreResult};
use binary_options_tools_core_pre::testing::{TestingWrapper, TestingWrapperBuilder};
use binary_options_tools_core_pre::traits::{ApiModule, Rule};
use kanal::{AsyncReceiver, AsyncSender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

struct DummyConnector {
    url: String,
}

impl DummyConnector {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

#[async_trait::async_trait]
impl Connector<()> for DummyConnector {
    async fn connect(&self, _: Arc<()>) -> ConnectorResult<WsStream> {
        println!("Connecting to {}", self.url);
        let wsstream = connect_async(&self.url).await.unwrap();
        Ok(wsstream.0)
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        println!("Disconnecting from {}", self.url);
        Ok(())
    }
}

// --- ApiModule 1: EchoModule ---
pub struct EchoModule {
    to_ws: AsyncSender<Message>,
    cmd_rx: AsyncReceiver<String>,
    cmd_tx: AsyncSender<String>,
    msg_rx: AsyncReceiver<Arc<Message>>,
    echo: AtomicBool,
}

#[async_trait]
impl ApiModule<()> for EchoModule {
    type Command = String;
    type CommandResponse = String;
    type Handle = EchoHandle;

    fn new(
        _state: Arc<()>,
        cmd_rx: AsyncReceiver<Self::Command>,
        cmd_ret_tx: AsyncSender<Self::CommandResponse>,
        msg_rx: AsyncReceiver<Arc<Message>>,
        to_ws: AsyncSender<Message>,
    ) -> Self {
        Self {
            to_ws,
            cmd_rx,
            cmd_tx: cmd_ret_tx,
            msg_rx,
            echo: AtomicBool::new(false),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        EchoHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            tokio::select! {
                Ok(cmd) = self.cmd_rx.recv() => {
                    let _ = self.to_ws.send(Message::text(cmd)).await;
                    self.echo.store(true, Ordering::SeqCst);
                }
                Ok(msg) = self.msg_rx.recv() => {
                    if let Message::Text(txt) = &*msg && self.echo.load(Ordering::SeqCst) {
                        let _ = self.cmd_tx.send(txt.to_string()).await;
                        self.echo.store(false, Ordering::SeqCst);
                    }
                }
            }
        }
    }

    fn rule(_: Arc<()>) -> Box<dyn Rule + Send + Sync> {
        Box::new(move |msg: &Message| {
            println!("Routing rule for EchoModule: {msg:?}");
            msg.is_text()
        })
    }
}

#[derive(Clone)]
pub struct EchoHandle {
    sender: AsyncSender<String>,
    receiver: AsyncReceiver<String>,
}

impl EchoHandle {
    pub async fn echo(&self, msg: String) -> CoreResult<String> {
        let _ = self.sender.send(msg).await;
        println!("In side echo handle, waiting for response...");
        Ok(self.receiver.recv().await?)
    }
}
// Testing Platform with integrated testing wrapper
pub struct TestingEchoPlatform {
    testing_wrapper: TestingWrapper<()>,
}

impl TestingEchoPlatform {
    pub async fn new(url: String) -> CoreResult<Self> {
        let connector = DummyConnector::new(url);

        let builder = ClientBuilder::new(connector, ()).with_module::<EchoModule>();

        // // Create testing wrapper with custom configuration
        // let testing_config = TestingConfig {
        //     stats_interval: Duration::from_secs(10), // Log stats every 10 seconds
        //     log_stats: true,
        //     track_events: true,
        //     max_reconnect_attempts: Some(3),
        //     reconnect_delay: Duration::from_secs(5),
        //     connection_timeout: Duration::from_secs(30),
        //     auto_reconnect: true,
        // };

        let testing_wrapper = TestingWrapperBuilder::new()
            .with_stats_interval(Duration::from_secs(10))
            .with_log_stats(true)
            .with_track_events(true)
            .with_max_reconnect_attempts(Some(3))
            .with_reconnect_delay(Duration::from_secs(5))
            .with_connection_timeout(Duration::from_secs(30))
            .with_auto_reconnect(true)
            .build_with_middleware(builder)
            .await?;

        Ok(Self { testing_wrapper })
    }

    pub async fn start(&mut self) -> CoreResult<()> {
        self.testing_wrapper.start().await
    }

    pub async fn stop(self) -> CoreResult<()> {
        self.testing_wrapper.stop().await?;
        Ok(())
    }

    pub async fn echo(&self, msg: String) -> CoreResult<String> {
        match self
            .testing_wrapper
            .client()
            .get_handle::<EchoModule>()
            .await
        {
            Some(echo_handle) => echo_handle.echo(msg).await,
            None => Err(CoreError::ModuleNotFound("EchoModule".to_string())),
        }
    }

    pub async fn get_stats(&self) -> binary_options_tools_core_pre::statistics::ConnectionStats {
        self.testing_wrapper.get_stats().await
    }

    pub async fn export_stats_json(&self) -> CoreResult<String> {
        self.testing_wrapper.export_stats_json().await
    }

    pub async fn export_stats_csv(&self) -> CoreResult<String> {
        self.testing_wrapper.export_stats_csv().await
    }

    pub async fn run_performance_test(&self, num_messages: usize, delay_ms: u64) -> CoreResult<()> {
        println!("Starting performance test with {num_messages} messages");

        let start_time = std::time::Instant::now();

        for i in 0..num_messages {
            let msg = format!("Test message {i}");
            match self.echo(msg.clone()).await {
                Ok(response) => {
                    println!("Message {i}: sent '{msg}', received '{response}'");
                }
                Err(e) => {
                    println!("Message {i} failed: {e}");
                }
            }

            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        let elapsed = start_time.elapsed();
        println!("Performance test completed in {elapsed:?}");

        // Print final statistics
        let stats = self.get_stats().await;
        println!("=== Performance Test Results ===");
        println!("Total messages sent: {}", stats.messages_sent);
        println!("Total messages received: {}", stats.messages_received);
        println!(
            "Average messages per second: {:.2}",
            stats.avg_messages_sent_per_second
        );
        println!("Total bytes sent: {}", stats.bytes_sent);
        println!("Total bytes received: {}", stats.bytes_received);
        println!("================================");

        Ok(())
    }
}

// fn test(msg: Message) -> bool {
//     if let Message::Binary(bin) = msg {
//         return bin.as_ref().starts_with(b"needle")
//     }
//     false
// }

// Demonstration of usage
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> CoreResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let mut platform = TestingEchoPlatform::new("wss://echo.websocket.org".to_string()).await?;

    // Start the platform (this will begin collecting statistics)
    platform.start().await?;

    println!("Platform started! Running tests...");

    // Give some time for the connection to establish
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Run a simple echo test
    println!("Testing basic echo functionality...");
    let response = platform.echo("Hello, Testing World!".to_string()).await?;
    println!("Echo response: {response}");

    // Run a performance test
    println!("Running performance test...");
    platform.run_performance_test(10, 1000).await?; // 10 messages, 1 second delay

    // Wait a bit more to collect statistics
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Export statistics
    println!("Exporting statistics...");
    // let json_stats = platform.export_stats_json().await?;
    // println!("JSON Stats:\n{json_stats}");

    let csv_stats = platform.export_stats_csv().await?;
    println!("CSV Stats:\n{csv_stats}");

    // Stop the platform using the new shutdown method
    platform.stop().await?;

    println!("Testing complete!");
    Ok(())
}
