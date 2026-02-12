// src/builder.rs

use kanal::{bounded_async, AsyncSender};
use std::any::type_name;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use crate::callback::{ConnectionCallback, ReconnectCallbackStack};
use crate::client::{Client, ClientRunner, LightweightHandler, Router};
use crate::connector::Connector;
use crate::error::{CoreError, CoreResult};
use crate::middleware::{MiddlewareStack, WebSocketMiddleware};
use crate::signals::Signals;
use crate::traits::{ApiModule, AppState, LightweightModule, ReconnectCallback, RunnerCommand};

type HandlerMap = Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>;
type HandlersFn<S> = Box<
    dyn FnOnce(
            &mut Router<S>,
            &mut JoinSet<()>,
            HandlerMap,
            AsyncSender<Message>,
            AsyncSender<RunnerCommand>,
            &mut ReconnectCallbackStack<S>,
        ) + Send
        + Sync,
>;

type LightweightHandlersFn<S> =
    Box<dyn FnOnce(&mut Router<S>, AsyncSender<Message>, AsyncSender<RunnerCommand>) + Send + Sync>;

pub struct ClientBuilder<S: AppState> {
    state: Arc<S>,
    connector: Arc<dyn Connector<S>>,
    connection_callback: ConnectionCallback<S>,
    lightweight_handlers: Vec<LightweightHandler<S>>,
    // Stores functions that know how to create and register each module.
    module_factories: Vec<HandlersFn<S>>,
    lightweight_factories: Vec<LightweightHandlersFn<S>>,
    // Middleware stack for WebSocket message processing
    middleware_stack: MiddlewareStack<S>,

    max_allowed_loops: u32,
    reconnect_delay: Duration,
}

impl<S: AppState> ClientBuilder<S> {
    /// Creates a new builder with the essential components.
    pub fn new(connector: impl Connector<S> + 'static, state: S) -> Self {
        Self {
            state: Arc::new(state),
            connector: Arc::new(connector),
            // Provide empty default callbacks.
            connection_callback: ConnectionCallback {
                on_connect: Box::new(|_, _| Box::pin(async { Ok(()) })),
                on_reconnect: ReconnectCallbackStack::default(),
            },
            lightweight_handlers: Vec::new(),
            module_factories: Vec::new(),
            lightweight_factories: Vec::new(),
            middleware_stack: MiddlewareStack::new(),
            max_allowed_loops: 0,
            reconnect_delay: Duration::from_secs(5),
        }
    }

    /// Sets the callback for the initial connection.
    pub fn on_connect(
        mut self,
        callback: impl Fn(
                Arc<S>,
                &AsyncSender<Message>,
            ) -> futures_util::future::BoxFuture<'static, CoreResult<()>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.connection_callback.on_connect = Box::new(callback);
        self
    }

    /// Sets the callback for subsequent reconnections.
    pub fn on_reconnect(
        mut self,
        callback: Box<dyn ReconnectCallback<S> + Send + Sync + 'static>,
    ) -> Self {
        self.connection_callback.on_reconnect.add_layer(callback);
        self
    }

    /// Adds a lightweight handler that receives all messages.
    pub fn with_lightweight_handler(
        mut self,
        handler: impl Fn(
                Arc<Message>,
                Arc<S>,
                &AsyncSender<Message>,
            ) -> futures_util::future::BoxFuture<'static, CoreResult<()>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.lightweight_handlers.push(Box::new(handler));
        self
    }

    /// Registers a lightweight module
    pub fn with_lightweight_module<M: LightweightModule<S>>(mut self) -> Self {
        let factory =
            |router: &mut Router<S>, to_ws_tx: AsyncSender<Message>, runner_tx: AsyncSender<RunnerCommand>| {
                let (msg_tx, msg_rx) = bounded_async(256);

                let state = router.state.clone();
                // Spawn the lightweight module task.
                router.spawn_lightweight_module(async move {
                    let mut failures = 0;
                    // make the first timestamp far enough in the past
                    let mut last_fail = Instant::now()
                        .checked_sub(Duration::from_secs(3600))
                        .unwrap_or(Instant::now());

                    loop {
                        // create the module once
                        let mut module = M::new(
                            state.clone(),
                            to_ws_tx.clone(),
                            msg_rx.clone(),
                            runner_tx.clone(),
                        );
                        match module.run().await {
                            Ok(()) => {
                                info!(target: "LightweightModule", "[Lightweight {}] exited cleanly", type_name::<M>());
                                break;
                            }
                            Err(e) => {
                                let now = Instant::now();
                                if now.duration_since(last_fail) < Duration::from_secs(30) {
                                    failures += 1;
                                } else {
                                    failures = 1;
                                }
                                last_fail = now;

                                if failures >= 5 {
                                    error!(target: "LightweightModule",
                                    "[Lightweight {}] failing {}× rapidly: {:?}, backing off 60s",
                                    type_name::<M>(),
                                    failures,
                                    e
                                );
                                    tokio::time::sleep(Duration::from_secs(60)).await;
                                } else {
                                    warn!(target: "LightweightModule", "[Lightweight {}] error: {:?}", type_name::<M>(), e);
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                }
                            }
                        }
                    }
                });
                router.add_lightweight_rule(M::rule(), msg_tx);
            };

        self.lightweight_factories.push(Box::new(factory));
        self
    }

    /// Registers a full API module with the client.
    pub fn with_module<M: ApiModule<S>>(mut self) -> Self {
        let factory = |router: &mut Router<S>,
                       join_set: &mut JoinSet<()>,
                       handles: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
                       to_ws_tx: AsyncSender<Message>,
                       runner_tx: AsyncSender<RunnerCommand>,
                       reconnect_callback_stack: &mut ReconnectCallbackStack<S>| {
            let (cmd_tx, cmd_rx) = bounded_async(32);
            let (cmd_ret_tx, cmd_ret_rx) = bounded_async(32);
            let (msg_tx, msg_rx) = bounded_async(256);

            let state = router.state.clone();
            let handle = M::create_handle(cmd_tx, cmd_ret_rx);

            // Must spawn this write to avoid blocking if called from an async context.
            join_set.spawn(async move {
                handles
                    .write()
                    .await
                    .insert(TypeId::of::<M>(), Box::new(handle));
            });

            match M::callback(
                state.clone(),
                cmd_rx.clone(),
                cmd_ret_tx.clone(),
                msg_rx.clone(),
                to_ws_tx.clone(),
            ) {
                Ok(Some(callback)) => {
                    reconnect_callback_stack.add_layer(callback);
                }
                Ok(None) => {
                    // No callback needed, continue.
                }
                Err(e) => {
                    error!(target: "ApiModule", "Failed to get callback for module {}: {:?}", type_name::<M>(), e);
                }
            }
            let state_clone = state.clone();
            router.spawn_module(async move {
                let mut failures = 0;
                let mut last_fail = Instant::now()
                    .checked_sub(Duration::from_secs(3600))
                    .unwrap_or(Instant::now());
                loop {
                    let mut module = M::new(
                        state.clone(),
                        cmd_rx.clone(),
                        cmd_ret_tx.clone(),
                        msg_rx.clone(),
                        to_ws_tx.clone(),
                        runner_tx.clone(),
                    );
                    match module.run().await {
                        Ok(_) => {
                            info!(target: "ApiModule", "[Module {}] exited cleanly", type_name::<M>());
                            break;
                        }
                        Err(e) => {
                            let now = Instant::now();
                            if now.duration_since(last_fail) < Duration::from_secs(30) {
                                failures += 1;
                            } else {
                                failures = 1;
                            }
                            last_fail = now;

                            let wait = if failures >= 5 {
                                error!(target: "ApiModule", "Module [{}] failed too many times, check module integrity: {:?}", type_name::<M>(), e);
                                60
                            } else {
                                warn!(target: "ApiModule", "[{}] err={:?}", type_name::<M>(), e);
                                1
                            };
                            tokio::time::sleep(Duration::from_secs(wait)).await;
                        }
                    }
                }
            });

            router.add_module_rule(M::rule(state_clone), msg_tx);
        };

        self.module_factories.push(Box::new(factory));
        self
    }

    /// Adds a middleware layer to the client.
    ///
    /// Middleware will be executed in the order they are added.
    /// They will be called for all WebSocket messages sent and received.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use binary_options_tools_core_pre::builder::ClientBuilder;
    /// # use binary_options_tools_core_pre::middleware::WebSocketMiddleware;
    /// # use binary_options_tools_core_pre::traits::AppState;
    /// # use binary_options_tools_core_pre::connector::{Connector, ConnectorResult, WsStream};
    /// # use async_trait::async_trait;
    /// # use std::sync::Arc;
    /// # #[derive(Debug)]
    /// # struct MyState;
    /// # impl AppState for MyState {
    /// #     fn clear_temporal_data(&self) {}
    /// # }
    /// # struct MyConnector;
    /// # #[async_trait]
    /// # impl Connector<MyState> for MyConnector {
    /// #     async fn connect(&self, _state: Arc<MyState>) -> ConnectorResult<WsStream> {
    /// #         unimplemented!()
    /// #     }
    /// #     async fn disconnect(&self) -> ConnectorResult<()> {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// # struct MyMiddleware;
    /// # #[async_trait]
    /// # impl WebSocketMiddleware<MyState> for MyMiddleware {}
    /// let builder = ClientBuilder::new(MyConnector, MyState)
    ///     .with_middleware(Box::new(MyMiddleware));
    /// ```
    pub fn with_middleware(mut self, middleware: Box<dyn WebSocketMiddleware<S>>) -> Self {
        self.middleware_stack.add_layer(middleware);
        self
    }

    /// Adds multiple middleware layers at once.
    ///
    /// This is a convenience method for adding multiple middleware layers.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use binary_options_tools_core_pre::builder::ClientBuilder;
    /// # use binary_options_tools_core_pre::middleware::WebSocketMiddleware;
    /// # use binary_options_tools_core_pre::traits::AppState;
    /// # use binary_options_tools_core_pre::connector::{Connector, ConnectorResult, WsStream};
    /// # use async_trait::async_trait;
    /// # use std::sync::Arc;
    /// # #[derive(Debug)]
    /// # struct MyState;
    /// # impl AppState for MyState {
    /// #     fn clear_temporal_data(&self) {}
    /// # }
    /// # struct MyConnector;
    /// # #[async_trait]
    /// # impl Connector<MyState> for MyConnector {
    /// #     async fn connect(&self, _state: Arc<MyState>) -> ConnectorResult<WsStream> {
    /// #         unimplemented!()
    /// #     }
    /// #     async fn disconnect(&self) -> ConnectorResult<()> {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// # struct MyMiddleware;
    /// # #[async_trait]
    /// # impl WebSocketMiddleware<MyState> for MyMiddleware {}
    /// let builder = ClientBuilder::new(MyConnector, MyState)
    ///     .with_middleware_layers(vec![
    ///         Box::new(MyMiddleware),
    ///         Box::new(MyMiddleware),
    ///     ]);
    /// ```
    pub fn with_middleware_layers(
        mut self,
        middleware: Vec<Box<dyn WebSocketMiddleware<S>>>,
    ) -> Self {
        for layer in middleware {
            self.middleware_stack.add_layer(layer);
        }
        self
    }

    /// Applies a middleware stack to the client.
    ///
    /// This replaces any existing middleware with the provided stack.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use binary_options_tools_core_pre::builder::ClientBuilder;
    /// # use binary_options_tools_core_pre::middleware::{MiddlewareStack, WebSocketMiddleware};
    /// # use binary_options_tools_core_pre::traits::AppState;
    /// # use binary_options_tools_core_pre::connector::{Connector, ConnectorResult, WsStream};
    /// # use async_trait::async_trait;
    /// # use std::sync::Arc;
    /// # #[derive(Debug)]
    /// # struct MyState;
    /// # impl AppState for MyState {
    /// #     fn clear_temporal_data(&self) {}
    /// # }
    /// # struct MyConnector;
    /// # #[async_trait]
    /// # impl Connector<MyState> for MyConnector {
    /// #     async fn connect(&self, _state: Arc<MyState>) -> ConnectorResult<WsStream> {
    /// #         unimplemented!()
    /// #     }
    /// #     async fn disconnect(&self) -> ConnectorResult<()> {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// # struct MyMiddleware;
    /// # #[async_trait]
    /// # impl WebSocketMiddleware<MyState> for MyMiddleware {}
    /// let mut stack = MiddlewareStack::new();
    /// stack.add_layer(Box::new(MyMiddleware));
    ///
    /// let builder = ClientBuilder::new(MyConnector, MyState)
    ///     .with_middleware_stack(stack);
    /// ```
    pub fn with_middleware_stack(mut self, stack: MiddlewareStack<S>) -> Self {
        self.middleware_stack = stack;
        self
    }

    /// Sets the maximum number of reconnection attempts.
    /// 0 means infinite attempts.
    pub fn with_max_allowed_loops(mut self, max_allowed_loops: u32) -> Self {
        self.max_allowed_loops = max_allowed_loops;
        self
    }

    /// Sets the base delay for reconnection attempts.
    pub fn with_reconnect_delay(mut self, reconnect_delay: Duration) -> Self {
        self.reconnect_delay = reconnect_delay;
        self
    }

    /// Assembles and returns the final `Client` handle and its `ClientRunner`.
    pub async fn build(self) -> CoreResult<(Client<S>, ClientRunner<S>)> {
        let (runner_cmd_tx, runner_cmd_rx) = bounded_async(8);
        let (to_ws_tx, to_ws_rx) = bounded_async(256);
        let signals = Signals::default();
        let client = Client::new(
            signals.clone(),
            runner_cmd_tx,
            self.state.clone(),
            to_ws_tx.clone(),
        );

        let mut router = Router::new(self.state.clone());
        router.lightweight_handlers = self.lightweight_handlers;
        router.middleware_stack = self.middleware_stack;

        let mut join_set = JoinSet::new();
        // Execute all the deferred module setup functions.
        let mut connection_callback = self.connection_callback;
        for factory in self.module_factories {
            factory(
                &mut router,
                &mut join_set,
                client.module_handles.clone(),
                to_ws_tx.clone(),
                runner_cmd_tx.clone(),
                &mut connection_callback.on_reconnect,
            );
        }

        for factory in self.lightweight_factories {
            factory(&mut router, to_ws_tx.clone(), runner_cmd_tx.clone());
        }

        // Wait for all the handles to be added to the handles hashmap.
        while let Some(h) = join_set.join_next().await {
            match h {
                Ok(_) => {} // Successfully added the module handle.
                Err(e) => {
                    error!("Failed to add module handle: {:?}", e);
                    return Err(CoreError::from(e));
                }
            }
        }

        let runner = ClientRunner {
            signal: signals,
            connector: self.connector,
            state: self.state,
            router: Arc::new(router),
            is_hard_disconnect: true,
            shutdown_requested: false,
            to_ws_sender: to_ws_tx,
            to_ws_receiver: to_ws_rx,
            runner_command_rx: runner_cmd_rx,
            connection_callback,
            reconnect_attempts: 0,
            max_allowed_loops: self.max_allowed_loops,
            reconnect_delay: self.reconnect_delay,
        };

        Ok((client, runner))
    }
}

// Add this test at the bottom of the file
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_client_builder_send_sync() {
        // This will fail to compile if ClientBuilder is not Send + Sync
        assert_send_sync::<ClientBuilder<()>>();
    }
}
