use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use binary_options_tools_core_pre::traits::AppState;

use crate::pocketoption_pre::types::{Assets, Deal};
use crate::pocketoption_pre::types::ServerTimeState;
use crate::pocketoption_pre::{
    error::{PocketError, PocketResult},
    ssid::Ssid,
};

/// Application state for PocketOption client
///
/// This structure holds all the shared state for the PocketOption client,
/// including session information, connection settings, and real-time data
/// like balance and server time synchronization.
///
/// # Thread Safety
///
/// All fields are designed to be thread-safe, allowing concurrent access
/// from multiple modules and tasks.
pub struct State {
    /// Unique identifier for the session.
    /// This is used to identify the session across different operations.
    pub ssid: Ssid,
    /// Default connection URL, if none is specified.
    pub default_connection_url: Option<String>,
    /// Default symbol to use if none is specified.
    pub default_symbol: String,
    /// Current balance, if available.
    pub balance: RwLock<Option<f64>>,
    /// Server time synchronization state
    pub server_time: ServerTimeState,
    /// Assets information
    pub assets: RwLock<Option<Assets>>,
    /// Holds the state for all trading-related data.
    pub trade_state: Arc<TradeState>,
}

/// Builder pattern for creating State instances
///
/// This builder provides a fluent interface for constructing State objects
/// with proper validation and defaults.
#[derive(Default)]
pub struct StateBuilder {
    ssid: Option<Ssid>,
    default_connection_url: Option<String>,
    default_symbol: Option<String>,
}

impl StateBuilder {
    /// Set the session ID for the state
    ///
    /// # Arguments
    /// * `ssid` - Valid session ID for PocketOption
    pub fn ssid(mut self, ssid: Ssid) -> Self {
        self.ssid = Some(ssid);
        self
    }

    /// Set the default connection URL
    ///
    /// # Arguments
    /// * `url` - Default WebSocket URL to use for connections
    pub fn default_connection_url(mut self, url: String) -> Self {
        self.default_connection_url = Some(url);
        self
    }

    /// Set the default trading symbol
    ///
    /// # Arguments
    /// * `symbol` - Default symbol to use for trading operations
    pub fn default_symbol(mut self, symbol: String) -> Self {
        self.default_symbol = Some(symbol);
        self
    }

    /// Build the final State instance
    ///
    /// # Returns
    /// Result containing the State or an error if required fields are missing
    pub fn build(self) -> PocketResult<State> {
        Ok(State {
            ssid: self
                .ssid
                .ok_or(PocketError::StateBuilder("SSID is required".into()))?,
            default_connection_url: self.default_connection_url,
            default_symbol: self
                .default_symbol
                .unwrap_or_else(|| "EURUSD_otc".to_string()),
            balance: RwLock::new(None),
            server_time: ServerTimeState::default(),
            assets: RwLock::new(None),
            trade_state: Arc::new(TradeState::default()),
        })
    }
}

#[async_trait]
impl AppState for State {
    async fn clear_temporal_data(&self) {
        // Clear any temporary data associated with the state
        let mut balance = self
            .balance
            .write()
            .await;
        *balance = None; // Clear balance
        // Note: We don't clear server time as it's useful to maintain
        // time synchronization across reconnections
    }
}

impl State {
    /// Sets the current balance.
    /// This method updates the balance in a thread-safe manner.
    ///
    /// # Arguments
    /// * `balance` - New balance value
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn set_balance(&self, balance: f64) {
        let mut state = self
            .balance
            .write()
            .await;
        *state = Some(balance);
    }

    /// Get the current balance
    ///
    /// # Returns
    /// Current balance if available
    pub async fn get_balance(&self) -> Option<f64> {
        let state = self
            .balance
            .read()
            .await;
        *state
    }

    /// Check if the current account is a demo account
    ///
    /// # Returns
    /// True if using demo account, false for real account
    pub fn is_demo(&self) -> bool {
        self.ssid.demo()
    }

    /// Get current server time
    ///
    /// # Returns
    /// Current estimated server time as Unix timestamp
    pub async fn get_server_time(&self) -> f64 {
        self.server_time.read().await.get_server_time()
    }

    /// Update server time with new timestamp
    ///
    /// # Arguments
    /// * `timestamp` - New server timestamp to synchronize with
    pub async fn update_server_time(&self, timestamp: f64) {
        self.server_time.write().await.update(timestamp);
    }

    /// Check if server time data is stale
    ///
    /// # Returns
    /// True if server time hasn't been updated recently
    pub async fn is_server_time_stale(&self) -> bool {
        self.server_time.read().await.is_stale()
    }

    /// Get server time as DateTime<Utc>
    ///
    /// # Returns
    /// Current server time as DateTime<Utc>
    pub async fn get_server_datetime(&self) -> DateTime<Utc> {
        let timestamp = self.get_server_time().await;
        DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(Utc::now)
    }

    /// Convert local time to server time
    ///
    /// # Arguments
    /// * `local_time` - Local DateTime<Utc> to convert
    ///
    /// # Returns
    /// Estimated server timestamp
    pub async fn local_to_server(&self, local_time: DateTime<Utc>) -> f64 {
        self.server_time.read().await.local_to_server(local_time)
    }

    /// Convert server time to local time
    ///
    /// # Arguments
    /// * `server_timestamp` - Server timestamp to convert
    ///
    /// # Returns
    /// Local DateTime<Utc>
    pub async fn server_to_local(&self, server_timestamp: f64) -> DateTime<Utc> {
        self.server_time
            .read()
            .await
            .server_to_local(server_timestamp)
    }

    /// Set the current assets.
    /// This method updates the assets in a thread-safe manner.
    /// # Arguments
    /// * `assets` - New assets information
    /// # Returns
    /// Result indicating success or failure
    pub async fn set_assets(&self, assets: Assets) {
        let mut state = self
            .assets
            .write()
            .await;
            *state = Some(assets);
    }


}

/// Holds all state related to trades and deals.
#[derive(Debug, Default)]
pub struct TradeState {
    /// A map of currently opened deals, keyed by their UUID.
    pub opened_deals: RwLock<HashMap<Uuid, Deal>>,
    /// A map of recently closed deals, keyed by their UUID.
    pub closed_deals: RwLock<HashMap<Uuid, Deal>>,
}

impl TradeState {
    /// Adds or updates deals in the opened_deals map.
    pub async fn update_opened_deals(&self, deals: Vec<Deal>) {
        // TODO: Implement the logic to update the opened deals map.
    }

    /// Moves deals from opened to closed and adds new closed deals.
    pub async fn update_closed_deals(&self, deals: Vec<Deal>) {
        // TODO: Implement the logic to update opened and closed deal maps.
    }

    /// Removes all deals from the closed_deals map.
    pub async fn clear_closed_deals(&self) {
        // TODO: Implement the logic to clear the closed deals map.
    }
}
