pub mod assets;
pub mod balance;
pub mod deals;
pub mod get_candles;
pub mod historical_data;
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
pub mod pending_trades;
pub mod raw;
pub mod server_time;
pub mod subscriptions;
pub mod trades;
// pub use subscriptions::{
//     CandleConfig, MAX_SUBSCRIPTIONS, SubscriptionCommand, SubscriptionHandle, SubscriptionModule,
//     SubscriptionResponse,
// };
