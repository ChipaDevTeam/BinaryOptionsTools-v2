use crate::callback::ConnectionCallback;
use crate::connector::Connector;
use crate::error::CoreResult;
use crate::middleware::{MiddlewareContext, MiddlewareStack};
use crate::signals::Signals;
use crate::traits::{ApiModule, AppState, ReconnectCallback, Rule};
use futures_util::{SinkExt, stream::StreamExt};
use kanal::{AsyncReceiver, AsyncSender};
use std::any::{Any, TypeId};
use std::future::Future;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
use rand::Rng;

/// A lightweight handler is a function that can process messages without being tied to a specific module.
/// It can be used for quick, non-blocking operations that don't require a full module lifecycle
/// or state management.
/// It takes a message, the shared application state, and a sender for outgoing messages.
/// It returns a future that resolves to a `CoreResult<()>`, indicating success or failure.
/// This is useful for handling messages that need to be processed quickly or in a lightweight manner,
/// such as logging, simple transformations, or forwarding messages to other parts of the system.
pub type LightweightHandler<S> = Box<
    dyn Fn(
            Arc<Message>,
            Arc<S>,
            &AsyncSender<Message>,
        ) -> futures_util::future::BoxFuture<'static, CoreResult<()>>
        + Send
        + Sync,
>;

type RuleTp = (Box<dyn Rule + Send + Sync>, AsyncSender<Arc<Message>>);
// --- Control Commands for the Runner ---

#[derive(Debug)]
pub enum RunnerCommand {
    Disconnect,
    Shutdown, // This can be used to gracefully shut down the runner
    Connect,
    Reconnect,
    // You can add more commands like Shutdown in the future
}

// --- Internal Router ---
pub struct Router<S: AppState> {
    pub(crate) state: Arc<S>,
    pub(crate) module_rules: Vec<RuleTp>,
    pub(crate) module_set: JoinSet<()>,
    pub(crate) lightweight_rules: Vec<RuleTp>,
    pub(crate) lightweight_handlers: Vec<LightweightHandler<S>>,
    pub(crate) lightweight_set: JoinSet<()>,
    pub(crate) middleware_stack: MiddlewareStack<S>,
}

impl<S: AppState> Router<S> {
    pub fn new(state: Arc<S>) -> Self {
        Self {
            state,
            module_rules: Vec::new(),
            module_set: JoinSet::new(),
            lightweight_rules: Vec::new(),
            lightweight_handlers: Vec::new(),
            lightweight_set: JoinSet::new(),
            middleware_stack: MiddlewareStack::new(),
        }
    }

    pub fn spawn_module<F: Future<Output = ()> + Send + 'static>(&mut self, task: F) {
        self.module_set.spawn(task);
    }

    pub fn add_module_rule(
        &mut self,
        rule: Box<dyn Rule + Send + Sync>,
        sender: AsyncSender<Arc<Message>>,
    ) {
        self.module_rules.push((rule, sender));
    }

    pub fn add_lightweight_rule(
        &mut self,
        rule: Box<dyn Rule + Send + Sync>,
        sender: AsyncSender<Arc<Message>>,
    ) {
        self.lightweight_rules.push((rule, sender));
    }

    pub fn add_lightweight_handler(&mut self, handler: LightweightHandler<S>) {
        self.lightweight_handlers.push(handler);
    }

    pub fn spawn_lightweight_module<F: Future<Output = ()> + Send + 'static>(&mut self, task: F) {
        self.lightweight_set.spawn(task);
    }

    /// Routes incoming WebSocket messages to appropriate handlers and modules.
    ///
    /// This method implements the core message routing logic with middleware integration:
    /// 1. **Middleware on_receive**: Called first for all incoming messages
    /// 2. **Lightweight handlers**: Processed for quick operations
    /// 3. **Lightweight modules**: Routed based on routing rules
    /// 4. **API modules**: Routed to matching modules
    ///
    /// # Middleware Integration
    /// The `on_receive` middleware hook is called at the beginning of message processing,
    /// allowing middleware to observe, log, or transform incoming messages before they
    /// reach the application logic.
    ///
    /// # Arguments
    /// - `message`: The incoming WebSocket message wrapped in Arc for sharing
    /// - `sender`: Channel for sending outgoing messages
    async fn route(&self, message: Arc<Message>, sender: &AsyncSender<Message>) -> CoreResult<()> {
        // Route to all lightweight handlers first
        debug!(target: "Router", "Routing message: {message:?}");

        // Create middleware context
        let middleware_context = MiddlewareContext::new(Arc::clone(&self.state), sender.clone());

        // 🎯 MIDDLEWARE HOOK: on_receive - called for ALL incoming messages
        // This is where middleware can observe, log, or process incoming messages
        self.middleware_stack
            .on_receive(&message, &middleware_context)
            .await;

        for handler in &self.lightweight_handlers {
            if let Err(err) = handler(Arc::clone(&message), Arc::clone(&self.state), sender).await {
                error!(target: "Router",
                     "Lightweight handler error: {err:#?}"
                );
            }
        }
        for (rule, sender) in &self.lightweight_rules {
            // If the rule matches, send the message to the lightweight handler
            if rule.call(&message) && sender.send(message.clone()).await.is_err() {
                error!(target: "Router", "A lightweight module has shut down and its channel is closed.");
            }
        }

        // Route to the first matching API module
        for (rule, sender) in &self.module_rules {
            if rule.call(&message) && sender.send(message.clone()).await.is_err() {
                error!(target: "Router", "A module has shut down and its channel is closed.");
            }
        }
        Ok(())
    }
}

// --- The Public-Facing Handle ---
#[derive(Debug)]
pub struct Client<S: AppState> {
    pub signal: Signals,
    /// The shared application state, which can be used by modules and handlers.
    pub state: Arc<S>,
    pub module_handles: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
    pub to_ws_sender: AsyncSender<Message>,

    runner_command_tx: AsyncSender<RunnerCommand>,
}

impl<S: AppState> Clone for Client<S> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            state: Arc::clone(&self.state),
            module_handles: Arc::clone(&self.module_handles),
            runner_command_tx: self.runner_command_tx.clone(),
            to_ws_sender: self.to_ws_sender.clone(),
        }
    }
}

impl<S: AppState> Client<S> {
    // In a real implementation, this would be created by the builder.
    pub fn new(
        signal: Signals,
        runner_command_tx: AsyncSender<RunnerCommand>,
        state: Arc<S>,
        sender: AsyncSender<Message>,
    ) -> Self {
        Self {
            signal,
            state,
            module_handles: Arc::new(RwLock::new(HashMap::new())),
            runner_command_tx,
            to_ws_sender: sender,
        }
    }

    /// Waits until the client is connected to the WebSocket server.
    /// This method will block until the connection is established.
    /// It is useful for ensuring that the client is ready to send and receive messages.
    pub async fn wait_connected(&self) {
        self.signal.wait_connected().await
    }

    /// Checks if the client is connected to the WebSocket server.
    pub fn is_connected(&self) -> bool {
        self.signal.is_connected()
    }

    /// Retrieves a clonable, typed handle to an already-registered module.
    pub async fn get_handle<M: ApiModule<S>>(&self) -> Option<M::Handle> {
        let handles = self.module_handles.read().await;
        handles
            .get(&TypeId::of::<M>())
            .and_then(|boxed_handle| boxed_handle.downcast_ref::<M::Handle>())
            .cloned()
    }

    /// Commands the runner to disconnect, clear state, and perform a "hard" reconnect.
    pub async fn disconnect(&self) -> CoreResult<()> {
        Ok(self
            .runner_command_tx
            .send(RunnerCommand::Disconnect)
            .await?)
    }

    /// Commands the runner to disconnect, and perform a "soft" reconnect.
    pub async fn reconnect(&self) -> CoreResult<()> {
        Ok(self
            .runner_command_tx
            .send(RunnerCommand::Reconnect)
            .await?)
    }

    /// Commands the runner to shutdown, this action is final as the runner and client will stop working and will be dropped.
    pub async fn shutdown(self) -> CoreResult<()> {
        self.runner_command_tx
            .send(RunnerCommand::Shutdown)
            .await
            .inspect_err(|e| {
                error!(target: "Client", "Failed to send shutdown command: {e}");
            })?;
        drop(self);
        info!(target: "Client", "Runner shutdown command sent.");
        Ok(())
    }

    /// Send a message to the WebSocket
    pub async fn send_message(&self, message: Message) -> CoreResult<()> {
        self.to_ws_sender.send(message).await.inspect_err(|e| {
            error!(target: "Client", "Failed to send message to WebSocket: {e}");
        })?;
        Ok(())
    }

    /// Send a text message to the WebSocket
    pub async fn send_text(&self, text: String) -> CoreResult<()> {
        self.send_message(Message::text(text)).await
    }

    /// Send a binary message to the WebSocket
    pub async fn send_binary(&self, data: Vec<u8>) -> CoreResult<()> {
        self.send_message(Message::binary(data)).await
    }
}

// --- The Background Worker ---
/// Implementation of the `ClientRunner` for managing WebSocket client connections and session lifecycle.
pub struct ClientRunner<S: AppState> {
    /// Notify the client of connection status changes.
    pub(crate) signal: Signals,
    pub(crate) connector: Arc<dyn Connector<S>>,
    pub(crate) router: Arc<Router<S>>,
    pub(crate) state: Arc<S>,
    // Flag to determine if the next connection is a fresh one.
    pub(crate) is_hard_disconnect: bool,
    // Flag to terminate the main run loop.
    pub(crate) shutdown_requested: bool,

    pub(crate) connection_callback: ConnectionCallback<S>,
    pub(crate) to_ws_sender: AsyncSender<Message>,
    pub(crate) to_ws_receiver: AsyncReceiver<Message>,
    pub(crate) runner_command_rx: AsyncReceiver<RunnerCommand>,

    // Track reconnection attempts for exponential backoff
    pub(crate) reconnect_attempts: u32,

    pub(crate) max_allowed_loops: u32,
    pub(crate) reconnect_delay: std::time::Duration,
}

impl<S: AppState> ClientRunner<S> {
    /// Main client runner loop that manages WebSocket connections and message processing.
    pub async fn run(&mut self) {
        // TODO: Add a way to disconnect and keep the connection closed intill specified otherwhise
        // The outermost loop runs until a shutdown is commanded.
        while !self.shutdown_requested {
            // Execute middleware on_connect hook
            let middleware_context =
                MiddlewareContext::new(Arc::clone(&self.state), self.to_ws_sender.clone());
            info!(target: "Runner", "Starting connection cycle...");

            // Call middleware to record connection attempt
            self.router
                .middleware_stack
                .record_connection_attempt(&middleware_context)
                .await;

            // Use the correct connection method based on the flag.
            let stream_result = if self.is_hard_disconnect {
                self.connector.connect(self.state.clone()).await
            } else {
                self.connector.reconnect(self.state.clone()).await
            };

            let ws_stream = match stream_result {
                Ok(stream) => {
                    self.reconnect_attempts = 0; // Reset attempts on success
                    stream
                },
                Err(e) => {
                    self.reconnect_attempts += 1;

                    if self.max_allowed_loops > 0 && self.reconnect_attempts >= self.max_allowed_loops {
                        error!(target: "Runner", "Maximum reconnection attempts ({}) reached. Shutting down.", self.max_allowed_loops);
                        self.shutdown_requested = true;
                        break;
                    }

                    // Use configured reconnect_delay with exponential backoff if it's > 0, else use a default
                    let base_delay = if self.reconnect_delay.as_secs() > 0 {
                        self.reconnect_delay.as_secs()
                    } else {
                        5
                    };

                    let delay_secs = std::cmp::min(base_delay.saturating_mul(2u64.saturating_pow(self.reconnect_attempts.min(10))), 300);
                    // Add jitter
                    let jitter = rand::rng().random_range(0.8..1.2);
                    let delay = std::time::Duration::from_secs_f64(delay_secs as f64 * jitter);

                    warn!(target: "Runner", "Connection failed (attempt {}/{}): {e}. Retrying in {:?}...",
                        self.reconnect_attempts,
                        if self.max_allowed_loops > 0 { self.max_allowed_loops.to_string() } else { "∞".to_string() },
                        delay);
                    tokio::time::sleep(delay).await;
                    // On failure, the next attempt is a reconnect, not a hard connect.
                    self.is_hard_disconnect = false;
                    continue; // Restart the connection cycle.
                }
            };

            // 🎯 MIDDLEWARE HOOK: on_connect - called after successful connection
            // Location: After WebSocket connection is established
            info!(target: "Runner", "Connection successful.");
            self.signal.set_connected();
            self.router
                .middleware_stack
                .on_connect(&middleware_context)
                .await;

            // Execute the correct callback.
            if self.is_hard_disconnect {
                info!(target: "Runner", "Executing on_connect callback.");
                // Handle any error from on_connect
                if let Err(err) =
                    (self.connection_callback.on_connect)(self.state.clone(), &self.to_ws_sender)
                        .await
                {
                    warn!(
                        target: "Runner",
                        "on_connect callback failed: {err:#?}"
                    );
                }
            } else {
                info!(target: "Runner", "Executing on_reconnect callback.");
                // Handle any error from on_reconnect
                if let Err(err) = self
                    .connection_callback
                    .on_reconnect
                    .call(self.state.clone(), &self.to_ws_sender)
                    .await
                {
                    warn!(
                        target: "Runner",
                        "on_reconnect callback failed: {err:#?}"
                    );
                }
            } // A successful connection means the next one is a "reconnect" unless told otherwise.
            self.is_hard_disconnect = false;

            let (mut ws_writer, mut ws_reader) = ws_stream.split();

            // 🎯 MIDDLEWARE HOOK: on_send - called in writer task for outgoing messages
            let writer_task = tokio::spawn({
                let to_ws_rx = self.to_ws_receiver.clone();
                let router = Arc::clone(&self.router);
                let state = Arc::clone(&self.state);
                let to_ws_sender = self.to_ws_sender.clone();
                async move {
                    let middleware_context = MiddlewareContext::new(state, to_ws_sender);
                    while let Ok(msg) = to_ws_rx.recv().await {
                        // Execute middleware on_send hook
                        router
                            .middleware_stack
                            .on_send(&msg, &middleware_context)
                            .await;
                        if ws_writer.send(msg).await.is_err() {
                            error!(target: "Runner", "WebSocket writer task failed to send message.");
                            break;
                        }
                    }
                }
            });

            let reader_task = tokio::spawn({
                let to_ws_sender = self.to_ws_sender.clone();
                let router = Arc::clone(&self.router); // Use Arc for sharing
                async move {
                    while let Some(Ok(msg)) = ws_reader.next().await {
                        if let Err(e) = router.route(Arc::new(msg), &to_ws_sender).await {
                            warn!(target: "Router", "Error routing message: {:?}", e);
                        }
                    }
                }
            });

            // --- Active Session Loop ---
            // This loop runs as long as the connection is stable or no commands are received.
            let mut writer_task_opt = Some(writer_task);
            let mut reader_task_opt: Option<tokio::task::JoinHandle<()>> = Some(reader_task);

            let mut session_active = true;

            // Temporal timer so we i can check the duration of a connection
            // let temporal_timer = std::time::Instant::now();
            while session_active {
                tokio::select! {
                    biased;

                    Ok(cmd) = self.runner_command_rx.recv() => {
                        match cmd {
                            RunnerCommand::Disconnect => {
                                // 🎯 MIDDLEWARE HOOK: on_disconnect - manual disconnect

                                info!(target: "Runner", "Disconnect command received.");

                                // Execute middleware on_disconnect hook
                                let middleware_context = MiddlewareContext::new(Arc::clone(&self.state), self.to_ws_sender.clone());
                                self.router.middleware_stack.on_disconnect(&middleware_context).await;

                                // Call connector's disconnect method to properly close the connection
                                if let Err(e) = self.connector.disconnect().await {
                                    warn!(target: "Runner", "Connector disconnect failed: {e}");
                                }


                                self.state.clear_temporal_data().await;
                                self.is_hard_disconnect = true;
                                if let Some(writer_task) = writer_task_opt.take() {
                                    writer_task.abort();
                                }
                                if let Some(reader_task) = reader_task_opt.take() {
                                    reader_task.abort();
                                }
                                self.signal.set_disconnected();
                                session_active = false;
                            },
                            RunnerCommand::Shutdown => {
                                // 🎯 MIDDLEWARE HOOK: on_disconnect - shutdown

                                info!(target: "Runner", "Shutdown command received.");

                                // Execute middleware on_disconnect hook
                                let middleware_context = MiddlewareContext::new(Arc::clone(&self.state), self.to_ws_sender.clone());
                                self.router.middleware_stack.on_disconnect(&middleware_context).await;

                                // Call connector's disconnect method to properly close the connection
                                if let Err(e) = self.connector.disconnect().await {
                                    warn!(target: "Runner", "Connector disconnect failed: {e}");
                                }

                                self.shutdown_requested = true;
                                if let Some(writer_task) = writer_task_opt.take() {
                                    writer_task.abort();
                                }
                                if let Some(reader_task) = reader_task_opt.take() {
                                    reader_task.abort();
                                }
                                self.signal.set_disconnected();
                                session_active = false;
                            }
                            _ => {}
                        }
                    },
                    _ = async {
                        if let Some(reader_task) = &mut reader_task_opt {
                            let _ = reader_task.await;
                        }
                    } => {
                        // 🎯 MIDDLEWARE HOOK: on_disconnect - unexpected connection loss
                        warn!(target: "Runner", "Connection lost unexpectedly.");

                        // Execute middleware on_disconnect hook
                        let middleware_context = MiddlewareContext::new(Arc::clone(&self.state), self.to_ws_sender.clone());
                        self.router.middleware_stack.on_disconnect(&middleware_context).await;

                        if let Some(writer_task) = writer_task_opt.take() {
                            writer_task.abort();
                        }
                        if let Some(reader_task) = reader_task_opt.take() {
                            // Already finished, but abort for completeness
                            reader_task.abort();
                        }
                        self.signal.set_disconnected();
                        session_active = false;
                        // panic!("Connection lost unexpectedly, exiting session loop. Duration: {:?}", temporal_timer.elapsed());
                    }
                }
            }
        }

        info!(target: "Runner", "Shutdown complete.");
    }
}

// A proper builder would be used here to configure and create the Client and ClientRunner
