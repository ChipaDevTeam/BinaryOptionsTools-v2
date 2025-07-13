use std::sync::Arc;

use binary_options_tools_core_pre::{error::CoreResult, reimports::Message};

use crate::pocketoption_pre::state::State;

pub mod assets;
pub mod balance;
pub mod deals;
/// Module implementations for PocketOption client
///
/// This module provides specialized handlers for different aspects of the
/// PocketOption trading platform:
///
/// # Modules
///
/// ## keep_alive
/// Contains modules for maintaining the WebSocket connection alive:
/// - `InitModule`: Handles initial authentication and setup
/// - `KeepAliveModule`: Sends periodic ping messages to prevent disconnection
///
/// ## balance
/// Manages account balance tracking and updates from the server.
///
/// ## server_time
/// Lightweight module for synchronizing local time with server time.
/// Automatically processes incoming price data to maintain accurate time sync.
///
/// ## subscriptions
/// Full-featured subscription management system:
/// - Symbol subscription/unsubscription
/// - Multiple aggregation strategies (Direct, Duration, Chunk)
/// - Real-time candle generation and emission
/// - Subscription statistics tracking
/// - Handles PocketOption's 4-subscription limit
///
/// # Architecture
///
/// Modules are designed using two patterns:
///
/// ## LightweightModule
/// For simple background processing without command-response mechanisms.
/// Examples: server_time, keep_alive
///
/// ## ApiModule
/// For full-featured modules with command-response patterns and public APIs.
/// Examples: subscriptions
///
/// Both patterns allow for clean separation of concerns and easy testing.
pub mod keep_alive;
pub mod server_time;
pub mod subscriptions;
pub mod trades;

// pub use subscriptions::{
//     CandleConfig, MAX_SUBSCRIPTIONS, SubscriptionCommand, SubscriptionHandle, SubscriptionModule,
//     SubscriptionResponse,
// };

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
/// * `_state` - Shared application state (unused)
///
/// # Returns
/// Always returns Ok(())
///
/// # Examples
///
/// ```rust
/// // Add as a lightweight handler to the client
/// client.with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state)));
/// ```
pub async fn print_handler(msg: Arc<Message>, _state: Arc<State>) -> CoreResult<()> {
    tracing::debug!(target: "Lightweight", "Received: {msg:?}");
    Ok(())
}
