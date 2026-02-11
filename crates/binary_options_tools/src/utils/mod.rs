use std::sync::Arc;
use std::sync::Once;

use binary_options_tools_core_pre::{
    error::CoreResult,
    middleware::{MiddlewareContext, WebSocketMiddleware},
    reimports::Message,
    traits::AppState,
};

pub mod serialize;

static INIT: Once = Once::new();

pub fn init_crypto_provider() {
    INIT.call_once(|| {
        rustls::crypto::ring::default_provider()
            .install_default()
            .ok();
    });
}

/// Lightweight message printer for debugging purposes
///
/// This handler logs all incoming WebSocket messages for debugging
/// and development purposes. It can be useful for understanding
/// the message flow and troubleshooting connection issues.
///
/// # Usage
///
/// This is typically used during development to monitor all WebSocket
/// traffic. It should be disabled in production due to performance
/// and log volume concerns.
///
/// # Arguments
/// * `msg` - WebSocket message to log
///
/// # Returns
/// Always returns Ok(())
///
/// # Examples
///
/// ```rust,ignore
/// // Add as a lightweight handler to the client
/// client.with_lightweight_handler(|msg, _, _| Box::pin(print_handler(msg)));
/// ```
pub async fn print_handler(msg: Arc<Message>) -> CoreResult<()> {
    tracing::info!(target: "Lightweight", "Received: {msg:?}");
    Ok(())
}

pub struct PrintMiddleware;

#[async_trait::async_trait]
impl<S: AppState> WebSocketMiddleware<S> for PrintMiddleware {
    async fn on_send(&self, message: &Message, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Default implementation does nothing

        tracing::debug!(target: "Middleware", "Sending: {message:?}");
        Ok(())
    }

    async fn on_receive(
        &self,
        message: &Message,
        _context: &MiddlewareContext<S>,
    ) -> CoreResult<()> {
        // Default implementation does nothing
        tracing::debug!(target: "Middleware", "Receiving: {message:?}");
        Ok(())
    }
}
