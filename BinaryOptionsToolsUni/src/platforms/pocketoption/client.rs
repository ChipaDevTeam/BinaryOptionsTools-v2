use bo2_macros::uniffi_doc;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use binary_options_tools::pocketoption::{
    candle::SubscriptionType, types::Action as OriginalAction, PocketOption as OriginalPocketOption,
};
use binary_options_tools::utils::f64_to_decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

use crate::error::UniError;
use binary_options_tools::error::BinaryOptionsError;

use super::{
    raw_handler::RawHandler,
    stream::SubscriptionStream,
    types::{Action, Asset, Candle, Deal, PendingOrder},
    validator::Validator,
};

#[uniffi_doc(
    name = "PocketOption",
    path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
)]
#[derive(uniffi::Object)]
pub struct PocketOption {
    inner: OriginalPocketOption,
}

#[uniffi::export]
impl PocketOption {
    /// Creates a new `PocketOption` client, authenticating with the given session ID.
    ///
    /// This is the primary constructor. Alias: `new`.
    #[uniffi::constructor]
    pub async fn init(ssid: String) -> Result<Arc<Self>, UniError> {
        let inner = OriginalPocketOption::new(ssid)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Arc::new(Self { inner }))
    }

    /// Creates a new `PocketOption` client, authenticating with the given session ID.
    ///
    /// Alias for `init`.
    #[uniffi::constructor]
    pub async fn new(ssid: String) -> Result<Arc<Self>, UniError> {
        let inner = OriginalPocketOption::new(ssid)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Arc::new(Self { inner }))
    }

    #[uniffi_doc(
        name = "new_with_url",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
    #[uniffi::constructor]
    pub async fn new_with_url(ssid: String, url: String) -> Result<Arc<Self>, UniError> {
        let inner = OriginalPocketOption::new_with_url(ssid, url)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Arc::new(Self { inner }))
    }

    /// Gets the current account balance.
    #[uniffi::method]
    pub async fn balance(&self) -> f64 {
        self.inner.balance().await.to_f64().unwrap_or_default()
    }

    /// Returns `true` if the current session is a demo account.
    #[uniffi::method]
    pub fn is_demo(&self) -> bool {
        self.inner.is_demo()
    }

    #[uniffi_doc(
        name = "trade",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| UniError::General(format!("Invalid amount: {}", amount)))?;
        let (_id, deal) = self
            .inner
            .trade(asset, original_action, time, decimal_amount)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(Deal::from(deal))
    }

    /// Places a Call (buy) trade. Shorthand for `trade(asset, Action::Call, time, amount)`.
    #[uniffi::method]
    pub async fn buy(&self, asset: String, time: u32, amount: f64) -> Result<Deal, UniError> {
        self.trade(asset, Action::Call, time, amount).await
    }

    /// Places a Put (sell) trade. Shorthand for `trade(asset, Action::Put, time, amount)`.
    #[uniffi::method]
    pub async fn sell(&self, asset: String, time: u32, amount: f64) -> Result<Deal, UniError> {
        self.trade(asset, Action::Put, time, amount).await
    }

    /// Returns the current server time as a Unix timestamp.
    #[uniffi::method]
    pub async fn server_time(&self) -> i64 {
        self.inner.server_time().await.timestamp()
    }

    /// Returns all available trading assets, or `None` if the asset list has not yet loaded.
    #[uniffi::method]
    pub async fn assets(&self) -> Option<Vec<Asset>> {
        self.inner
            .assets()
            .await
            .map(|assets_map| assets_map.0.values().cloned().map(Asset::from).collect())
    }

    #[uniffi_doc(
        name = "result",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    #[uniffi_doc(
        name = "result",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    /// Returns all currently open deals.
    #[uniffi::method]
    pub async fn get_opened_deals(&self) -> Vec<Deal> {
        self.inner
            .get_opened_deals()
            .await
            .into_values()
            .map(Deal::from)
            .collect()
    }

    /// Returns all closed deals stored in the client's state.
    #[uniffi::method]
    pub async fn get_closed_deals(&self) -> Vec<Deal> {
        self.inner
            .get_closed_deals()
            .await
            .into_values()
            .map(Deal::from)
            .collect()
    }

    #[uniffi_doc(
        name = "open_pending_order",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
    #[allow(clippy::too_many_arguments)]
    #[uniffi::method]
    pub async fn open_pending_order(
        &self,
        open_type: u32,
        amount: f64,
        asset: String,
        open_time: u32,
        open_price: f64,
        timeframe: u32,
        min_payout: u32,
        command: u32,
    ) -> Result<PendingOrder, UniError> {
        let decimal_amount = f64_to_decimal(amount)
            .ok_or_else(|| UniError::General(format!("Invalid amount: {}", amount)))?;
        let decimal_open_price = f64_to_decimal(open_price)
            .ok_or_else(|| UniError::General(format!("Invalid open price: {}", open_price)))?;

        let order = self
            .inner
            .open_pending_order(
                open_type,
                decimal_amount,
                asset,
                open_time,
                decimal_open_price,
                timeframe,
                min_payout,
                command,
            )
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
        Ok(PendingOrder::from(order))
    }

    /// Returns all currently pending orders.
    #[uniffi::method]
    pub async fn get_pending_deals(&self) -> Vec<PendingOrder> {
        self.inner
            .get_pending_deals()
            .await
            .into_values()
            .map(PendingOrder::from)
            .collect()
    }

    /// Clears the closed-deals list from the client's in-memory state.
    #[uniffi::method]
    pub async fn clear_closed_deals(&self) {
        self.inner.clear_closed_deals().await
    }

    #[uniffi_doc(
        name = "subscribe",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    /// Stops the real-time candle subscription for the given asset.
    #[uniffi::method]
    pub async fn unsubscribe(&self, asset: String) -> Result<(), UniError> {
        self.inner
            .unsubscribe(asset)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    #[uniffi_doc(
        name = "candles",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    #[uniffi_doc(
        name = "candles",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    #[uniffi_doc(
        name = "candles",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    /// Disconnects and reconnects the WebSocket client.
    #[uniffi::method]
    pub async fn reconnect(&self) -> Result<(), UniError> {
        self.inner
            .reconnect()
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    /// Shuts down the client and stops all background tasks.
    ///
    /// Call this when you are done with the client to ensure a clean exit.
    #[uniffi::method]
    pub async fn shutdown(&self) -> Result<(), UniError> {
        self.inner
            .shutdown()
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    #[uniffi_doc(
        name = "create_raw_handler",
        path = "BinaryOptionsToolsUni/docs_json/pocket_option.json"
    )]
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

    /// Returns the payout percentage for the given asset symbol, or `None` if unavailable.
    ///
    /// A value of `0.8` means 80% profit on a winning trade.
    #[uniffi::method]
    pub async fn payout(&self, asset: String) -> Option<f64> {
        let assets = self.inner.assets().await?;
        let asset_info = assets.0.get(&asset)?;
        Some(asset_info.payout as f64 / 100.0)
    }

    /// Returns all closed deals. Alias for `get_closed_deals`.
    #[uniffi::method]
    pub async fn get_trade_history(&self) -> Vec<Deal> {
        self.get_closed_deals().await
    }

    /// Returns the close timestamp (Unix) of a deal by its UUID string, or `None` if not found.
    #[uniffi::method]
    pub async fn get_deal_end_time(&self, id: String) -> Option<i64> {
        let deal_id = Uuid::parse_str(&id).ok()?;
        if let Some(d) = self.inner.get_closed_deal(deal_id).await {
            return Some(d.close_timestamp.timestamp());
        }
        self.inner
            .get_opened_deal(deal_id)
            .await
            .map(|d| d.close_timestamp.timestamp())
    }
}
