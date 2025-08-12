use async_trait::async_trait;
use binary_options_tools_core_pre::builder::ClientBuilder;
use binary_options_tools_core_pre::client::Client;
use binary_options_tools_core_pre::connector::ConnectorResult;
use binary_options_tools_core_pre::connector::{Connector, WsStream};
use binary_options_tools_core_pre::error::{CoreError, CoreResult};
use binary_options_tools_core_pre::traits::{ApiModule, Rule};
use futures_util::stream::unfold;
use futures_util::{Stream, StreamExt};
use kanal::{AsyncReceiver, AsyncSender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
        // Simulate a WebSocket connection
        println!("Connecting to {}", self.url);
        let wsstream = connect_async(&self.url).await.unwrap();
        Ok(wsstream.0)
    }

    async fn disconnect(&self) -> ConnectorResult<()> {
        // Simulate disconnection
        println!("Disconnecting from {}", self.url);
        Ok(())
    }
}

// --- Lightweight Handlers ---
async fn print_handler(msg: Arc<Message>, _state: Arc<()>) -> CoreResult<()> {
    println!("[Lightweight] Received: {msg:?}");
    Ok(())
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
        Box::new(move |msg: &Message| msg.is_text())
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

// --- ApiModule 2: StreamModule ---
pub struct StreamModule {
    msg_rx: AsyncReceiver<Arc<Message>>,
    cmd_rx: AsyncReceiver<bool>,
    cmd_tx: AsyncSender<String>,
    send: AtomicBool,
}

#[async_trait]
impl ApiModule<()> for StreamModule {
    type Command = bool;
    type CommandResponse = String;
    type Handle = StreamHandle;

    fn new(
        _state: Arc<()>,
        cmd_rx: AsyncReceiver<Self::Command>,
        cmd_ret_tx: AsyncSender<Self::CommandResponse>,
        msg_rx: AsyncReceiver<Arc<Message>>,
        _to_ws: AsyncSender<Message>,
    ) -> Self {
        Self {
            msg_rx,
            cmd_tx: cmd_ret_tx,
            cmd_rx,
            send: AtomicBool::new(false),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        StreamHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            tokio::select! {
                Ok(cmd) = self.cmd_rx.recv() => {
                    // Update the send flag based on the received command
                    self.send.store(cmd, Ordering::SeqCst);
                }
                Ok(msg) = self.msg_rx.recv() => {
                    if let Message::Text(txt) = &*msg
                        && self.send.load(Ordering::SeqCst) {
                            // Process the message if send is true
                            println!("[StreamModule] Received: {txt}");
                            let _ = self.cmd_tx.send(txt.to_string()).await;
                        }
                }
                else => {
                    println!("[Error] StreamModule: Channel closed");
                },
            }
        }
    }

    fn rule(_: Arc<()>) -> Box<dyn Rule + Send + Sync> {
        Box::new(move |_msg: &Message| {
            // Accept all messages
            true
        })
    }
}

#[derive(Clone)]
pub struct StreamHandle {
    receiver: AsyncReceiver<String>,
    sender: AsyncSender<bool>,
}

impl StreamHandle {
    pub async fn stream(self) -> CoreResult<impl Stream<Item = CoreResult<String>>> {
        self.sender.send(true).await?;
        println!("StreamHandle: Waiting for messages...");
        Ok(Box::pin(unfold(self.receiver, |state| async move {
            let item = state.recv().await.map_err(CoreError::from);
            Some((item, state))
        })))
    }
}

// --- ApiModule 3: PeriodicSenderModule ---
pub struct PeriodicSenderModule {
    cmd_rx: AsyncReceiver<bool>,
    to_ws: AsyncSender<Message>,
    running: AtomicBool,
}

#[async_trait]
impl ApiModule<()> for PeriodicSenderModule {
    type Command = bool; // true = start, false = stop
    type CommandResponse = ();
    type Handle = PeriodicSenderHandle;

    fn new(
        _state: Arc<()>,
        cmd_rx: AsyncReceiver<Self::Command>,
        _cmd_ret_tx: AsyncSender<Self::CommandResponse>,
        _msg_rx: AsyncReceiver<Arc<Message>>,
        to_ws: AsyncSender<Message>,
    ) -> Self {
        Self {
            cmd_rx,
            to_ws,
            running: AtomicBool::new(false),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        _receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        PeriodicSenderHandle { sender }
    }

    async fn run(&mut self) -> CoreResult<()> {
        let to_ws = self.to_ws.clone();
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                Ok(cmd) = self.cmd_rx.recv() => {
                    self.running.store(cmd, Ordering::SeqCst);
                }
                _ = interval.tick() => {
                    if self.running.load(Ordering::SeqCst) {
                        let _ = to_ws.send(Message::text("Ping from periodic sender")).await;
                    }
                }
            }
        }
    }

    fn rule(_: Arc<()>) -> Box<dyn Rule + Send + Sync> {
        Box::new(move |_msg: &Message| {
            // This module does not process incoming messages
            false
        })
    }
}

#[derive(Clone)]
pub struct PeriodicSenderHandle {
    sender: AsyncSender<bool>,
}

impl PeriodicSenderHandle {
    /// Start periodic sending
    pub async fn start(&self) {
        let _ = self.sender.send(true).await;
    }
    /// Stop periodic sending
    pub async fn stop(&self) {
        let _ = self.sender.send(false).await;
    }
}

// --- EchoPlatform Struct ---
pub struct EchoPlatform {
    client: Client<()>,
    _runner: tokio::task::JoinHandle<()>,
}

impl EchoPlatform {
    pub async fn new(url: String) -> CoreResult<Self> {
        // Use a simple connector (implement your own if needed)
        let connector = DummyConnector::new(url);

        let mut builder = ClientBuilder::new(connector, ());
        builder =
            builder.with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state)));
        let (client, mut runner) = builder
            .with_module::<EchoModule>()
            .with_module::<StreamModule>()
            .with_module::<PeriodicSenderModule>()
            .build()
            .await?;

        // let echo_handle = client.get_handle::<EchoModule>().await.unwrap();
        // let stream_handle = client.get_handle::<StreamModule>().await.unwrap();

        // Start runner in background
        let _runner = tokio::spawn(async move { runner.run().await });

        Ok(Self { client, _runner })
    }

    pub async fn echo(&self, msg: String) -> CoreResult<String> {
        match self.client.get_handle::<EchoModule>().await {
            Some(echo_handle) => echo_handle.echo(msg).await,
            None => Err(CoreError::ModuleNotFound("EchoModule".to_string())),
        }
    }

    pub async fn stream(&self) -> CoreResult<impl Stream<Item = CoreResult<String>>> {
        let stream_handle = self.client.get_handle::<StreamModule>().await.unwrap();
        println!("Starting stream...");
        stream_handle.stream().await
    }

    pub async fn start(&self) -> CoreResult<()> {
        match self.client.get_handle::<PeriodicSenderModule>().await {
            Some(handle) => {
                handle.start().await;
                Ok(())
            }
            None => Err(CoreError::ModuleNotFound(
                stringify!(PeriodicSenderModule).to_string(),
            )),
        }
    }
}

// --- Main Example ---
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> CoreResult<()> {
    let platform = EchoPlatform::new("wss://echo.websocket.org".to_string()).await?;
    platform.start().await?;
    println!("Platform started, ready to echo!");
    println!("{}", platform.echo("Hello, Echo!".to_string()).await?);

    // Wait to receive the echo
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut stream = platform.stream().await?;
    while let Some(Ok(msg)) = stream.next().await {
        println!("Streamed message: {msg}");
    }
    Ok(())
}
// can you make some kind of new implementation / wrapper around a client / runner that tests it a lot like check the connection lattency, checks the time since las disconnection, the time the system kept connected before calling the connect or reconnect functions, also i want it to work like for structs like the EchoPlatform like with a cupple of lines i pass the configuration of the struct (like functions to call espected return )
