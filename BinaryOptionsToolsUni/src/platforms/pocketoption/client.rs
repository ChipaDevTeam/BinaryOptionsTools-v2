use std::sync::Arc;
use std::time::Duration as StdDuration;

use binary_options_tools::pocketoption::{
    PocketOption as OriginalPocketOption, candle::SubscriptionType, types::Action as OriginalAction,
};
use uuid::Uuid;

use crate::error::UniError;
use binary_options_tools::error::BinaryOptionsError;

use super::{
    raw_handler::RawHandler,
    stream::SubscriptionStream,
    types::{Action, Asset, Candle, Deal},
    validator::Validator,
};

/// The main client for interacting with the PocketOption platform.
///
/// This object provides all the functionality needed to connect to PocketOption,
/// place trades, get account information, and subscribe to market data.
///
/// It is the primary entry point for using this library.
///
/// # Rationale
///
/// This struct wraps the underlying `binary_options_tools::pocketoption::PocketOption` client,
/// exposing its functionality in a way that is compatible with UniFFI for creating
/// multi-language bindings.
#[derive(uniffi::Object)]
pub struct PocketOption {
    inner: OriginalPocketOption,
}

#[uniffi::export]
impl PocketOption {
    /// Creates a new instance of the PocketOption client.
    ///
    /// This is the primary constructor for the client. It requires a session ID (ssid)
    /// to authenticate with the PocketOption servers.
    ///
    /// # Arguments
    ///
    /// * `ssid` - The session ID for your PocketOption account.
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// import asyncio
    /// from binaryoptionstoolsuni import PocketOption
    ///
    /// async def main():
    ///     ssid = "YOUR_SESSION_ID"
    ///     api = await PocketOption.init(ssid)
    ///     balance = await api.balance()
    ///     print(f"Balance: {balance}")
    ///
    /// asyncio.run(main())
    /// ```
    #[uniffi::constructor]
    pub async fn init(ssid: String) -> Result<Arc<Self>, UniError> {
        let inner = OriginalPocketOption::new(ssid)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Arc::new(Self { inner }))
    }

    /// Creates a new instance of the PocketOption client.
    ///
    /// This is the primary constructor for the client. It requires a session ID (ssid)
    /// to authenticate with the PocketOption servers.
    ///
    /// # Arguments
    ///
    /// * `ssid` - The session ID for your PocketOption account.
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// import asyncio
    /// from binaryoptionstoolsuni import PocketOption
    ///
    /// async def main():
    ///     ssid = "YOUR_SESSION_ID"
    ///     api = await PocketOption.new(ssid)
    ///     balance = await api.balance()
    ///     print(f"Balance: {balance}")
    ///
    /// asyncio.run(main())
    /// ```
    #[uniffi::constructor]
    pub async fn new(ssid: String) -> Result<Arc<Self>, UniError> {
        let inner = OriginalPocketOption::new(ssid)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Arc::new(Self { inner }))
    }

    /// Creates a new instance of the PocketOption client with a custom WebSocket URL.
    ///
    /// This constructor is useful for connecting to different PocketOption servers,
    /// for example, in different regions.
    ///
    /// # Arguments
    ///
    /// * `ssid` - The session ID for your PocketOption account.
    /// * `url` - The custom WebSocket URL to connect to.
    #[uniffi::constructor]
    pub async fn new_with_url(ssid: String, url: String) -> Result<Arc<Self>, UniError> {
        let inner = OriginalPocketOption::new_with_url(ssid, url)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Arc::new(Self { inner }))
    }

    /// Gets the current balance of the account.
    ///
    /// This method retrieves the current trading balance from the client's state.
    ///
    /// # Returns
    ///
    /// The current balance as a floating-point number.
    #[uniffi::method]
    pub async fn balance(&self) -> f64 {
        self.inner.balance().await
    }

    /// Checks if the current session is a demo account.
    ///
    /// # Returns
    ///
    /// `true` if the account is a demo account, `false` otherwise.
    #[uniffi::method]
    pub fn is_demo(&self) -> bool {
        self.inner.is_demo()
    }

    /// Places a trade.
    ///
    /// This is the core method for executing trades.
    ///
    /// # Arguments
    ///
    /// * `asset` - The symbol of the asset to trade (e.g., "EURUSD_otc").
    /// * `action` - The direction of the trade (`Action.Call` or `Action.Put`).
    /// * `time` - The duration of the trade in seconds.
    /// * `amount` - The amount to trade.
    ///
    /// # Returns
    ///
    /// A `Deal` object representing the completed trade.
    #[uniffi::method]

    pub async fn trade(
        &self,
        asset: String,
        action: Action,
        time: u32,
        amount: f64,
    ) -> Result<Deal, UniError> {
        let original_action = match action {
            Action::Call => OriginalAction::Call,
            Action::Put => OriginalAction::Put,
        };
        let (_id, deal) = self
            .inner
            .trade(asset, original_action, time, amount)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Deal::from(deal))
    }

    /// Places a "Call" (buy) trade.
    ///
    /// This is a convenience method that calls `trade` with `Action.Call`.
    #[uniffi::method]
    pub async fn buy(&self, asset: String, time: u32, amount: f64) -> Result<Deal, UniError> {
        self.trade(asset, Action::Call, time, amount).await
    }

    /// Places a "Put" (sell) trade.
    ///
    /// This is a convenience method that calls `trade` with `Action.Put`.
    #[uniffi::method]
    pub async fn sell(&self, asset: String, time: u32, amount: f64) -> Result<Deal, UniError> {
        self.trade(asset, Action::Put, time, amount).await
    }

    /// Gets the current server time as a Unix timestamp.
    #[uniffi::method]
    pub async fn server_time(&self) -> i64 {
        self.inner.server_time().await.timestamp()
    }

    /// Gets the list of available assets for trading.
    ///
    /// # Returns
    ///
    /// A list of `Asset` objects, or `None` if the assets have not been loaded yet.
    #[uniffi::method]
    pub async fn assets(&self) -> Option<Vec<Asset>> {
        self.inner
            .assets()
            .await
            .map(|assets_map| assets_map.0.values().cloned().map(Asset::from).collect())
    }

    /// Checks the result of a trade by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the trade to check (as a string).
    ///
    /// # Returns
    ///
    /// A `Deal` object representing the completed trade.
    #[uniffi::method]
    pub async fn result(&self, id: String) -> Result<Deal, UniError> {
        let uuid =
            Uuid::parse_str(&id).map_err(|e| UniError::Uuid(format!("Invalid UUID: {e}")))?;
        let deal = self
            .inner
            .result(uuid)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Deal::from(deal))
    }

    /// Checks the result of a trade by its ID with a timeout.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the trade to check (as a string).
    /// * `timeout_secs` - The maximum time to wait for the result in seconds.
    ///
    /// # Returns
    ///
    /// A `Deal` object representing the completed trade.
    #[uniffi::method]
    pub async fn result_with_timeout(
        &self,
        id: String,
        timeout_secs: u64,
    ) -> Result<Deal, UniError> {
        let uuid =
            Uuid::parse_str(&id).map_err(|e| UniError::Uuid(format!("Invalid UUID: {e}")))?;
        let deal = self
            .inner
            .result_with_timeout(uuid, StdDuration::from_secs(timeout_secs))
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Deal::from(deal))
    }

    /// Gets the list of currently opened deals.
    #[uniffi::method]
    pub async fn get_opened_deals(&self) -> Vec<Deal> {
        self.inner
            .get_opened_deals()
            .await
            .into_values()
            .map(Deal::from)
            .collect()
    }

    /// Gets the list of currently closed deals.
    #[uniffi::method]
    pub async fn get_closed_deals(&self) -> Vec<Deal> {
        self.inner
            .get_closed_deals()
            .await
            .into_values()
            .map(Deal::from)
            .collect()
    }

    /// Clears the list of closed deals from the client's state.
    #[uniffi::method]
    pub async fn clear_closed_deals(&self) {
        self.inner.clear_closed_deals().await
    }

    /// Subscribes to real-time candle data for a specific asset.
    ///
    /// # Arguments
    ///
    /// * `asset` - The symbol of the asset to subscribe to.
    /// * `duration_secs` - The duration of each candle in seconds.
    ///
    /// # Returns
    ///
    /// A `SubscriptionStream` object that can be used to receive candle data.
    #[uniffi::method]
    pub async fn subscribe(
        &self,
        asset: String,
        duration_secs: u64,
    ) -> Result<Arc<SubscriptionStream>, UniError> {
        let sub_type = SubscriptionType::time_aligned(StdDuration::from_secs(duration_secs))
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        let original_stream = self
            .inner
            .subscribe(asset, sub_type)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(SubscriptionStream::from_original(original_stream))
    }

    /// Unsubscribes from real-time candle data for a specific asset.
    #[uniffi::method]
    pub async fn unsubscribe(&self, asset: String) -> Result<(), UniError> {
        self.inner
            .unsubscribe(asset)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    /// Gets historical candle data for a specific asset with advanced parameters.
    #[uniffi::method]
    pub async fn get_candles_advanced(
        &self,
        asset: String,
        period: i64,
        time: i64,
        offset: i64,
    ) -> Result<Vec<Candle>, UniError> {
        let candles = self
            .inner
            .get_candles_advanced(asset, period, time, offset)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?
            .into_iter()
            .map(Candle::from)
            .collect();
        Ok(candles)
    }

    /// Gets historical candle data for a specific asset.
    #[uniffi::method]
    pub async fn get_candles(
        &self,
        asset: String,
        period: i64,
        offset: i64,
    ) -> Result<Vec<Candle>, UniError> {
        let candles = self
            .inner
            .get_candles(asset, period, offset)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?
            .into_iter()
            .map(Candle::from)
            .collect();
        Ok(candles)
    }

    /// Gets historical candle data for a specific asset and period.
    #[uniffi::method]
    pub async fn history(&self, asset: String, period: u32) -> Result<Vec<Candle>, UniError> {
        let candles = self
            .inner
            .history(asset, period)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?
            .into_iter()
            .map(Candle::from)
            .collect();
        Ok(candles)
    }

    /// Disconnects and reconnects the client.
    #[uniffi::method]
    pub async fn reconnect(&self) -> Result<(), UniError> {
        self.inner
            .reconnect()
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    /// Shuts down the client and stops all background tasks.
    ///
    /// This method should be called when you are finished with the client
    /// to ensure a graceful shutdown.
    #[uniffi::method]
    pub async fn shutdown(self: Arc<Self>) -> Result<(), UniError> {
        // Call shutdown on a clone of the inner client to consume it
        self.inner
            .clone()
            .shutdown()
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    /// Creates a raw handler for advanced WebSocket message operations.
    ///
    /// This allows you to send custom messages and receive filtered responses
    /// based on a validator. Useful for implementing custom protocols or
    /// accessing features not directly exposed by the API.
    ///
    /// # Arguments
    ///
    /// * `validator` - Validator to filter incoming messages
    /// * `keep_alive` - Optional message to send on reconnect (e.g., for re-subscribing)
    ///
    /// # Returns
    ///
    /// A `RawHandler` object for sending and receiving messages
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// # Create a validator for balance updates
    /// validator = Validator.contains('"balance"')
    /// handler = await client.create_raw_handler(validator, None)
    ///
    /// # Send a custom message
    /// await handler.send_text('42["getBalance"]')
    ///
    /// # Wait for response
    /// response = await handler.wait_next()
    /// print(f"Received: {response}")
    /// ```
    #[uniffi::method]
    pub async fn create_raw_handler(
        &self,
        validator: Arc<Validator>,
        keep_alive: Option<String>,
    ) -> Result<Arc<RawHandler>, UniError> {
        use binary_options_tools::pocketoption::modules::raw::Outgoing;
        
        let keep_alive_msg = keep_alive.map(Outgoing::Text);
        let inner_handler = self
            .inner
            .create_raw_handler(validator.inner().clone(), keep_alive_msg)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;

        Ok(RawHandler::from_inner(inner_handler))
    }

    /// Gets the payout percentage for a specific asset.
    ///
    /// Returns the profit percentage you'll receive if a trade on this asset wins.
    /// For example, 0.8 means 80% profit (if you bet $1, you get $1.80 back).
    ///
    /// # Arguments
    ///
    /// * `asset` - The symbol of the asset (e.g., "EURUSD_otc")
    ///
    /// # Returns
    ///
    /// The payout percentage as a float, or None if the asset is not available
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// payout = await client.payout("EURUSD_otc")
    /// if payout:
    ///     print(f"Payout: {payout * 100}%")
    ///     # Example output: "Payout: 80.0%"
    /// else:
    ///     print("Asset not available")
    /// ```
    #[uniffi::method]
    pub async fn payout(&self, asset: String) -> Option<f64> {
        let assets = self.inner.assets().await?;
        let asset_info = assets.0.get(&asset)?;
        Some(asset_info.payout as f64 / 100.0)
    }

    /// Gets the trade history.
    ///
    /// This is an alias for `get_closed_deals`.
    #[uniffi::method]
    pub async fn get_trade_history(&self) -> Vec<Deal> {
        self.get_closed_deals().await
    }

    /// Gets the end time of a deal by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the deal to check.
    ///
    /// # Returns
    ///
    /// The close timestamp as a Unix timestamp, or `None` if the deal is not found.
    #[uniffi::method]
    pub async fn get_deal_end_time(&self, id: String) -> Option<i64> {
let deal_id = Uuid::parse_str(&id).ok()?;
self.inner.get_closed_deal(deal_id)
    .await.map(|d| d.close_timestamp.timestamp())
    }
}
