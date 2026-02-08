//! Middleware system for WebSocket client operations.
//!
//! This module provides a composable middleware system inspired by Axum's middleware/layer system.
//! Middleware can be used to observe, modify, or control the flow of WebSocket messages
//! being sent and received by the client.
//!
//! # Key Components
//!
//! - [`WebSocketMiddleware`]: The core trait for implementing middleware
//! - [`MiddlewareStack`]: A composable stack of middleware layers
//! - [`MiddlewareContext`]: Context passed to middleware with message and client information
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use binary_options_tools_core_pre::middleware::{WebSocketMiddleware, MiddlewareContext};
//! use binary_options_tools_core_pre::traits::AppState;
//! use binary_options_tools_core_pre::error::CoreResult;
//! use async_trait::async_trait;
//! use tokio_tungstenite::tungstenite::Message;
//! use std::sync::Arc;
//!
//! #[derive(Debug)]
//! struct MyState;
//! impl AppState for MyState {
//!     fn clear_temporal_data(&self) {}
//! }
//!
//! // Example statistics middleware
//! struct StatisticsMiddleware {
//!     sent_count: Arc<std::sync::atomic::AtomicU64>,
//!     received_count: Arc<std::sync::atomic::AtomicU64>,
//! }
//!
//! #[async_trait]
//! impl WebSocketMiddleware<MyState> for StatisticsMiddleware {
//!     async fn on_send(&self, message: &Message, context: &MiddlewareContext<MyState>) -> CoreResult<()> {
//!         self.sent_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
//!         println!("Sending message: {:?}", message);
//!         Ok(())
//!     }
//!
//!     async fn on_receive(&self, message: &Message, context: &MiddlewareContext<MyState>) -> CoreResult<()> {
//!         self.received_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
//!         println!("Received message: {:?}", message);
//!         Ok(())
//!     }
//! }
//! ```

use crate::error::CoreResult;
use crate::traits::AppState;
use async_trait::async_trait;
use kanal::AsyncSender;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, warn};

/// Context information passed to middleware during message processing.
///
/// This struct provides middleware with access to the application state
/// and the WebSocket sender channel for sending messages.
#[derive(Debug)]
pub struct MiddlewareContext<S: AppState> {
    /// The shared application state
    pub state: Arc<S>,
    /// The WebSocket sender for outgoing messages
    pub ws_sender: AsyncSender<Message>,
}

impl<S: AppState> MiddlewareContext<S> {
    /// Creates a new middleware context.
    pub fn new(state: Arc<S>, ws_sender: AsyncSender<Message>) -> Self {
        Self { state, ws_sender }
    }
}

/// Trait for implementing WebSocket middleware.
///
/// Middleware can observe and react to WebSocket messages being sent and received.
/// This trait provides hooks for both outgoing and incoming messages.
///
/// # Type Parameters
/// - `S`: The application state type that implements [`AppState`]
///
/// # Methods
/// - [`on_send`]: Called before a message is sent to the WebSocket
/// - [`on_receive`]: Called after a message is received from the WebSocket
/// - [`on_connect`]: Called when a WebSocket connection is established
/// - [`on_disconnect`]: Called when a WebSocket connection is lost
///
/// # Error Handling
/// Middleware should be designed to be resilient. If middleware returns an error,
/// it will be logged but will not prevent the message from being processed or
/// other middleware from running.
#[async_trait]
pub trait WebSocketMiddleware<S: AppState>: Send + Sync + 'static {
    /// Called before a message is sent to the WebSocket.
    ///
    /// # Arguments
    /// - `message`: The message that will be sent
    /// - `context`: Context information including state and sender
    ///
    /// # Returns
    /// - `Ok(())` if the middleware processed successfully
    /// - `Err(_)` if an error occurred (will be logged but not block processing)
    async fn on_send(&self, message: &Message, context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Default implementation does nothing
        let _ = (message, context);
        Ok(())
    }

    /// Called after a message is received from the WebSocket.
    ///
    /// # Arguments
    /// - `message`: The message that was received
    /// - `context`: Context information including state and sender
    ///
    /// # Returns
    /// - `Ok(())` if the middleware processed successfully
    /// - `Err(_)` if an error occurred (will be logged but not block processing)
    async fn on_receive(
        &self,
        message: &Message,
        context: &MiddlewareContext<S>,
    ) -> CoreResult<()> {
        // Default implementation does nothing
        let _ = (message, context);
        Ok(())
    }

    /// Called when a WebSocket connection is established.
    ///
    /// # Arguments
    /// - `context`: Context information including state and sender
    ///
    /// # Returns
    /// - `Ok(())` if the middleware processed successfully
    /// - `Err(_)` if an error occurred (will be logged but not block processing)
    async fn on_connect(&self, context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Default implementation does nothing
        let _ = context;
        Ok(())
    }

    /// Called when a WebSocket connection is lost.
    ///
    /// # Arguments
    /// - `context`: Context information including state and sender
    ///
    /// # Returns
    /// - `Ok(())` if the middleware processed successfully
    /// - `Err(_)` if an error occurred (will be logged but not block processing)
    async fn on_disconnect(&self, context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Default implementation does nothing
        let _ = context;
        Ok(())
    }

    /// Called when a connection attempt is made (before actual connection)
    async fn on_connection_attempt(&self, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        Ok(())
    }

    /// Called when a connection attempt fails
    async fn on_connection_failure(
        &self,
        _context: &MiddlewareContext<S>,
        _reason: Option<String>,
    ) -> CoreResult<()> {
        Ok(())
    }
}

/// A composable stack of middleware layers.
///
/// This struct holds a collection of middleware that will be executed in order.
/// Middleware are executed in the order they are added to the stack.
///
/// # Example
/// ```rust,no_run
/// use binary_options_tools_core_pre::middleware::MiddlewareStack;
/// # use binary_options_tools_core_pre::middleware::WebSocketMiddleware;
/// # use binary_options_tools_core_pre::traits::AppState;
/// # use async_trait::async_trait;
/// # #[derive(Debug)]
/// # struct MyState;
/// # impl AppState for MyState {
/// #     fn clear_temporal_data(&self) {}
/// # }
/// # struct LoggingMiddleware;
/// # #[async_trait]
/// # impl WebSocketMiddleware<MyState> for LoggingMiddleware {}
/// # struct StatisticsMiddleware;
/// # impl StatisticsMiddleware {
/// #     fn new() -> Self { Self }
/// # }
/// # #[async_trait]
/// # impl WebSocketMiddleware<MyState> for StatisticsMiddleware {}
///
/// let mut stack = MiddlewareStack::new();
/// stack.add_layer(Box::new(LoggingMiddleware));
/// stack.add_layer(Box::new(StatisticsMiddleware::new()));
/// ```
pub struct MiddlewareStack<S: AppState> {
    layers: Vec<Box<dyn WebSocketMiddleware<S> + Send + Sync>>,
}

impl<S: AppState> MiddlewareStack<S> {
    /// Creates a new empty middleware stack.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Adds a middleware layer to the stack.
    ///
    /// Middleware will be executed in the order they are added.
    pub fn add_layer(&mut self, middleware: Box<dyn WebSocketMiddleware<S> + Send + Sync>) {
        self.layers.push(middleware);
    }

    /// Executes all middleware for an outgoing message.
    ///
    /// # Arguments
    /// - `message`: The message being sent
    /// - `context`: Context information
    ///
    /// # Behavior
    /// All middleware will be executed even if some fail. Errors are logged but
    /// do not prevent other middleware from running.
    pub async fn on_send(&self, message: &Message, context: &MiddlewareContext<S>) {
        for (index, middleware) in self.layers.iter().enumerate() {
            if let Err(e) = middleware.on_send(message, context).await {
                error!(
                    target: "Middleware",
                    "Error in middleware layer {} on_send: {:?}",
                    index, e
                );
            }
        }
    }

    /// Executes all middleware for an incoming message.
    ///
    /// # Arguments
    /// - `message`: The message that was received
    /// - `context`: Context information
    ///
    /// # Behavior
    /// All middleware will be executed even if some fail. Errors are logged but
    /// do not prevent other middleware from running.
    pub async fn on_receive(&self, message: &Message, context: &MiddlewareContext<S>) {
        for (index, middleware) in self.layers.iter().enumerate() {
            if let Err(e) = middleware.on_receive(message, context).await {
                error!(
                    target: "Middleware",
                    "Error in middleware layer {} on_receive: {:?}",
                    index, e
                );
            }
        }
    }

    /// Executes all middleware for connection establishment.
    ///
    /// # Arguments
    /// - `context`: Context information
    ///
    /// # Behavior
    /// All middleware will be executed even if some fail. Errors are logged but
    /// do not prevent other middleware from running.
    pub async fn on_connect(&self, context: &MiddlewareContext<S>) {
        for (index, middleware) in self.layers.iter().enumerate() {
            if let Err(e) = middleware.on_connect(context).await {
                error!(
                    target: "Middleware",
                    "Error in middleware layer {} on_connect: {:?}",
                    index, e
                );
            }
        }
    }

    /// Executes all middleware for connection loss.
    ///
    /// # Arguments
    /// - `context`: Context information
    ///
    /// # Behavior
    /// All middleware will be executed even if some fail. Errors are logged but
    /// do not prevent other middleware from running.
    pub async fn on_disconnect(&self, context: &MiddlewareContext<S>) {
        for (index, middleware) in self.layers.iter().enumerate() {
            if let Err(e) = middleware.on_disconnect(context).await {
                warn!(
                    target: "Middleware",
                    "Error in middleware layer {} on_disconnect: {:?}",
                    index, e
                );
            }
        }
    }

    /// Record a connection attempt across all middleware
    pub async fn record_connection_attempt(&self, context: &MiddlewareContext<S>) {
        for (index, middleware) in self.layers.iter().enumerate() {
            if let Err(e) = middleware.on_connection_attempt(context).await {
                warn!(
                    target: "Middleware",
                    "Error in middleware layer {} on_connection_attempt: {:?}",
                    index, e
                );
            }
        }
    }

    /// Record a connection failure across all middleware
    pub async fn record_connection_failure(
        &self,
        context: &MiddlewareContext<S>,
        reason: Option<String>,
    ) {
        for (index, middleware) in self.layers.iter().enumerate() {
            if let Err(e) = middleware
                .on_connection_failure(context, reason.clone())
                .await
            {
                warn!(
                    target: "Middleware",
                    "Error in middleware layer {} on_connection_failure: {:?}",
                    index, e
                );
            }
        }
    }

    /// Returns the number of middleware layers in the stack.
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Returns true if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

impl<S: AppState> Default for MiddlewareStack<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// A builder for creating middleware stacks in a fluent manner.
///
/// This provides a convenient way to chain middleware additions.
///
/// # Example
/// ```rust,no_run
/// use binary_options_tools_core_pre::middleware::MiddlewareStackBuilder;
/// # use binary_options_tools_core_pre::middleware::WebSocketMiddleware;
/// # use binary_options_tools_core_pre::traits::AppState;
/// # use async_trait::async_trait;
/// # #[derive(Debug)]
/// # struct MyState;
/// # impl AppState for MyState {
/// #     fn clear_temporal_data(&self) {}
/// # }
/// # struct LoggingMiddleware;
/// # #[async_trait]
/// # impl WebSocketMiddleware<MyState> for LoggingMiddleware {}
/// # struct StatisticsMiddleware;
/// # impl StatisticsMiddleware {
/// #     fn new() -> Self { Self }
/// # }
/// # #[async_trait]
/// # impl WebSocketMiddleware<MyState> for StatisticsMiddleware {}
///
/// let stack = MiddlewareStackBuilder::new()
///     .layer(Box::new(LoggingMiddleware))
///     .layer(Box::new(StatisticsMiddleware::new()))
///     .build();
/// ```
pub struct MiddlewareStackBuilder<S: AppState> {
    stack: MiddlewareStack<S>,
}

impl<S: AppState> MiddlewareStackBuilder<S> {
    /// Creates a new middleware stack builder.
    pub fn new() -> Self {
        Self {
            stack: MiddlewareStack::new(),
        }
    }

    /// Adds a middleware layer to the stack.
    pub fn layer(mut self, middleware: Box<dyn WebSocketMiddleware<S>>) -> Self {
        self.stack.add_layer(middleware);
        self
    }

    /// Builds and returns the middleware stack.
    pub fn build(self) -> MiddlewareStack<S> {
        self.stack
    }
}

impl<S: AppState> Default for MiddlewareStackBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug)]
    struct TestState;

    #[async_trait]
    impl AppState for TestState {
        async fn clear_temporal_data(&self) {}
    }

    struct TestMiddleware {
        #[allow(dead_code)]
        name: String,
        send_count: AtomicU64,
        receive_count: AtomicU64,
    }

    impl TestMiddleware {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                send_count: AtomicU64::new(0),
                receive_count: AtomicU64::new(0),
            }
        }
    }

    #[async_trait]
    impl WebSocketMiddleware<TestState> for TestMiddleware {
        async fn on_send(
            &self,
            _message: &Message,
            _context: &MiddlewareContext<TestState>,
        ) -> CoreResult<()> {
            self.send_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn on_receive(
            &self,
            _message: &Message,
            _context: &MiddlewareContext<TestState>,
        ) -> CoreResult<()> {
            self.receive_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_middleware_stack() {
        let (sender, _receiver) = kanal::bounded_async(10);
        let state = Arc::new(TestState);
        let context = MiddlewareContext::new(state, sender);

        let middleware1 = TestMiddleware::new("test1");
        let middleware2 = TestMiddleware::new("test2");

        let mut stack = MiddlewareStack::new();
        stack.add_layer(Box::new(middleware1));
        stack.add_layer(Box::new(middleware2));

        let message = Message::text("test");

        // Test on_send
        stack.on_send(&message, &context).await;

        // Test on_receive
        stack.on_receive(&message, &context).await;

        assert_eq!(stack.len(), 2);
        assert!(!stack.is_empty());
    }

    #[tokio::test]
    async fn test_middleware_stack_builder() {
        let stack = MiddlewareStackBuilder::new()
            .layer(Box::new(TestMiddleware::new("test1")))
            .layer(Box::new(TestMiddleware::new("test2")))
            .build();

        assert_eq!(stack.len(), 2);
    }
}
