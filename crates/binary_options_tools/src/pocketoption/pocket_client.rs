use std::{collections::HashMap, sync::Arc, time::Duration};

use binary_options_tools_core_pre::{
    builder::ClientBuilder,
    client::Client,
    testing::{TestingWrapper, TestingWrapperBuilder},
    traits::{ApiModule, ReconnectCallback},
    error::CoreResult,
    reimports::AsyncSender,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::pocketoption::types::Outgoing;
use crate::config::Config;
use crate::{
    error::BinaryOptionsError,
    pocketoption::{
        candle::{Candle, SubscriptionType},
        connect::PocketConnect,
        error::{PocketError, PocketResult},
        modules::{
            assets::AssetsModule,
            balance::BalanceModule,
            deals::DealsApiModule,
            get_candles::GetCandlesApiModule,
            historical_data::HistoricalDataApiModule,
            keep_alive::{InitModule, KeepAliveModule},
            pending_trades::PendingTradesApiModule,
            raw::{RawApiModule, RawHandle as InnerRawHandle, RawHandler as InnerRawHandler},
            server_time::ServerTimeModule,
            subscriptions::{SubscriptionStream, SubscriptionsApiModule},
            trades::TradesApiModule,
        },
        ssid::Ssid,
        state::{State, StateBuilder},
        types::{Action, Assets, Deal, PendingOrder},
    },
    utils::print_handler,
};

const MINIMUM_TRADE_AMOUNT: f64 = 1.0;
const MAXIMUM_TRADE_AMOUNT: f64 = 20000.0;

/// Reconnection callback to verify potential lost trades
struct TradeReconciliationCallback;

#[async_trait::async_trait]
impl ReconnectCallback<State> for TradeReconciliationCallback {
    async fn call(&self, state: Arc<State>, _ws_sender: &AsyncSender<binary_options_tools_core_pre::reimports::Message>) -> CoreResult<()> {
        let pending = state.trade_state.pending_market_orders.read().await;

        for (req_id, (order, created_at)) in pending.iter() {
            // If order was sent >5 seconds ago, verify it
            if created_at.elapsed() > Duration::from_secs(5) {
                tracing::warn!(target: "TradeReconciliation", "Verifying potentially lost trade: {} (sent {:?} ago). Order: {:?}", req_id, created_at.elapsed(), order);
                // In a real implementation, we would try to fetch the trade status from the API if possible
            }
        }

        // Clean up orders >120 seconds old (failed/timed out)
        drop(pending); // Drop read lock before acquiring write lock
        let mut pending = state.trade_state.pending_market_orders.write().await;
        pending.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(120));

        Ok(())
    }
}

/// PocketOption client for interacting with the PocketOption trading platform.
///
/// This client provides methods for trading, checking balances, subscribing to
/// asset updates, and managing the connection to the PocketOption platform.
///
/// # Example
/// ```
/// use binary_options_tools_pocketoption::PocketOption;
///
/// #[tokio::main]
/// async fn main() -> binary_options_tools_core_pre::error::CoreResult<()> {
///     let pocket_option = PocketOption::new("your_session_id").await?;
///     let balance = pocket_option.balance().await;
///     println!("Current balance: {}", balance);
///     Ok(())
/// }
/// ```
#[derive(Clone)]

pub struct PocketOption {
    client: Client<State>,
    _runner: Arc<tokio::task::JoinHandle<()>>,
    pub config: Config,
}

impl PocketOption {
    fn configure_common_modules(builder: ClientBuilder<State>) -> ClientBuilder<State> {
        builder
            .with_lightweight_module::<KeepAliveModule>()
            .with_lightweight_module::<InitModule>()
            .with_lightweight_module::<BalanceModule>()
            .with_lightweight_module::<ServerTimeModule>()
            .with_lightweight_module::<AssetsModule>()
            .with_module::<TradesApiModule>()
            .with_module::<DealsApiModule>()
            .with_module::<SubscriptionsApiModule>()
            .with_module::<GetCandlesApiModule>()
            .with_module::<PendingTradesApiModule>()
            .with_module::<HistoricalDataApiModule>()
            .with_module::<RawApiModule>()
            .with_lightweight_handler(|msg, _, _| Box::pin(print_handler(msg)))
            .on_reconnect(Box::new(TradeReconciliationCallback))
    }

    async fn require_handle<M: ApiModule<State>>(&self, module_name: &str) -> PocketResult<M::Handle> {
        self.client
            .get_handle::<M>()
            .await
            .ok_or_else(|| BinaryOptionsError::General(format!("{module_name} not found")).into())
    }

    fn builder(ssid: impl ToString) -> PocketResult<ClientBuilder<State>> {
        let state = StateBuilder::default().ssid(Ssid::parse(ssid)?).build()?;
        Ok(Self::configure_common_modules(ClientBuilder::new(
            PocketConnect,
            state,
        )))
    }

    /// Creates a new PocketOption client with the provided session ID.
    ///
    /// # Arguments
    /// * `ssid` - The session ID (SSID cookie value) for authenticating with PocketOption.
    ///
    /// # Returns
    /// A `PocketResult` containing the initialized `PocketOption` client.
    ///
    /// # Example
    /// ```no_run
    /// use binary_options_tools::PocketOption;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PocketOption::new("your-session-id").await?;
    ///     let balance = client.balance().await;
    ///     println!("Balance: {}", balance);
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(ssid: impl ToString) -> PocketResult<Self> {
        Self::new_with_config(ssid, Config::default()).await
    }

    /// Creates a new PocketOption client with a custom WebSocket URL.
    ///
    /// This method allows you to specify a custom WebSocket URL for connecting to the PocketOption platform,
    /// which can be useful for testing or connecting to alternative endpoints.
    ///
    /// # Arguments
    /// * `ssid` - The session ID (SSID cookie value) for authenticating with PocketOption.
    /// * `url` - The custom WebSocket URL to connect to.
    ///
    /// # Returns
    /// A `PocketResult` containing the initialized `PocketOption` client.
    pub async fn new_with_url(ssid: impl ToString, url: String) -> PocketResult<Self> {
        let mut config = Config::default();
        if let Ok(parsed_url) = url::Url::parse(&url) {
            config.urls.push(parsed_url);
        }
        
        // We still use the state builder for the initial connection URL
        // because ClientRunner uses the state's URL.
        // The config.urls are fallbacks or for future use.
        let state = StateBuilder::default()
            .ssid(Ssid::parse(ssid)?)
            .default_connection_url(url)
            .build()?;
            
        let builder = Self::configure_common_modules(ClientBuilder::new(PocketConnect, state));
        let (client, mut runner) = builder.build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });

        Ok(Self {
            client,
            _runner: Arc::new(_runner),
            config,
        })
    }

    /// Creates a new PocketOption client with the provided configuration.
    pub async fn new_with_config(ssid: impl ToString, config: Config) -> PocketResult<Self> {
        let mut builder = StateBuilder::default().ssid(Ssid::parse(ssid)?);

        // Use the first URL from config as default if available
        if let Some(url) = config.urls.first() {
            builder = builder.default_connection_url(url.to_string());
        }

        // Pass all URLs as fallbacks
        builder = builder.urls(config.urls.iter().map(|u| u.to_string()).collect());

        let state = builder.build()?;
        let client_builder = Self::configure_common_modules(ClientBuilder::new(PocketConnect, state))
            .with_max_allowed_loops(config.max_allowed_loops)
            .with_reconnect_delay(config.reconnect_time);

        let (client, mut runner): (Client<State>, binary_options_tools_core_pre::client::ClientRunner<State>) = client_builder.build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });

        match tokio::time::timeout(
            config.connection_initialization_timeout,
            client.wait_connected(),
        )
        .await
        {
            Ok(_) => {}
            Err(_) => {
                return Err(PocketError::General(
                    "Connection initialization timed out".into(),
                ));
            }
        }

        Ok(Self {
            client,
            _runner: Arc::new(_runner),
            config,
        })
    }

    /// Get a handle to the Raw module for ad-hoc validators and custom message processing.
    pub async fn raw_handle(&self) -> PocketResult<InnerRawHandle> {
        self.require_handle::<RawApiModule>("RawApiModule").await
    }

    /// Convenience: create a RawHandler bound to a validator, optionally sending a keep-alive message on reconnect.
    pub async fn create_raw_handler(
        &self,
        validator: crate::validator::Validator,
        keep_alive: Option<Outgoing>,
    ) -> PocketResult<InnerRawHandler> {
        let handle = self.require_handle::<RawApiModule>("RawApiModule").await?;
        handle
            .create(validator, keep_alive)
            .await
            .map_err(|e| e.into())
    }

    /// Gets the current balance of the user.
    /// If the balance is not set, it returns -1.
    ///
    pub async fn balance(&self) -> f64 {
        let state = &self.client.state;
        let balance = state.balance.read().await;
        if let Some(balance) = *balance {
            return balance;
        }
        -1.0
    }

    /// Checks if the account is a demo account.
    ///
    /// # Returns
    /// `true` if the account is a demo account, `false` if it's a real account.
    pub fn is_demo(&self) -> bool {
        let state = &self.client.state;
        state.ssid.demo()
    }

    /// Validates if an asset is active and supports the given timeframe without cloning the entire assets map.
    pub async fn validate_asset(&self, asset: &str, time: u32) -> PocketResult<()> {
        let state = &self.client.state;
        let assets = state.assets.read().await;
        if let Some(assets) = assets.as_ref() {
            assets.validate(asset, time)
        } else {
            Err(PocketError::General("Assets not loaded".to_string()))
        }
    }

    /// Executes a trade on the specified asset.
    /// # Arguments
    /// * `asset` - The asset to trade.
    /// * `action` - The action to perform (Call or Put).
    /// * `time` - The time to trade.
    /// * `amount` - The amount to trade.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if
    /// the trade fails.
    pub async fn trade(
        &self,
        asset: impl ToString,
        action: Action,
        time: u32,
        amount: f64,
    ) -> PocketResult<(Uuid, Deal)> {
        let asset_str = asset.to_string();

        // Fix #6: Input Validation
        if !amount.is_finite() {
            return Err(PocketError::General("Amount must be a finite number".into()));
        }
        if amount <= 0.0 {
             return Err(PocketError::General("Amount must be positive".into()));
        }

        self.validate_asset(&asset_str, time).await?;

        if amount < MINIMUM_TRADE_AMOUNT {
            return Err(PocketError::General(format!(
                "Amount must be at least {MINIMUM_TRADE_AMOUNT}"
            )));
        }
        if amount > MAXIMUM_TRADE_AMOUNT {
            return Err(PocketError::General(format!(
                "Amount must be at most {MAXIMUM_TRADE_AMOUNT}"
            )));
        }

        // Fix #4: Duplicate Trade Prevention
        let amount_cents = (amount * 100.0).round() as u64;
        let fingerprint = (asset_str.clone(), action, time, amount_cents);

        {
            let recent = self.client.state.trade_state.recent_trades.read().await;
            if let Some((existing_id, created_at)) = recent.get(&fingerprint) {
                if created_at.elapsed() < Duration::from_secs(2) {
                    return Err(PocketError::General(format!(
                        "Duplicate trade blocked (original ID: {})", existing_id
                    )));
                }
            }
        }

        let handle = self.require_handle::<TradesApiModule>("TradesApiModule").await?;

        let deal = handle
            .trade(asset_str.clone(), action, amount, time)
            .await?;

        // Store for deduplication
        {
            let mut recent = self.client.state.trade_state.recent_trades.write().await;
            recent.insert(fingerprint, (deal.id, std::time::Instant::now()));
            // Cleanup old entries (>5 seconds)
            recent.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(5));
        }

        Ok((deal.id, deal))
    }

    /// Places a new buy trade.
    /// This method is a convenience wrapper around the `trade` method.
    /// # Arguments
    /// * `asset` - The asset to trade.
    /// * `time` - The time to trade.
    /// * `amount` - The amount to trade.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if the trade fails.
    pub async fn buy(
        &self,
        asset: impl ToString,
        time: u32,
        amount: f64,
    ) -> PocketResult<(Uuid, Deal)> {
        self.trade(asset, Action::Call, time, amount).await
    }

    /// Places a new sell trade.
    /// This method is a convenience wrapper around the `trade` method.
    /// # Arguments
    /// * `asset` - The asset to trade.
    /// * `time` - The time to trade.
    /// * `amount` - The amount to trade.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if the trade fails.
    pub async fn sell(
        &self,
        asset: impl ToString,
        time: u32,
        amount: f64,
    ) -> PocketResult<(Uuid, Deal)> {
        self.trade(asset, Action::Put, time, amount).await
    }

    /// Gets the current server time.
    /// If the server time is not set, it returns None.
    pub async fn server_time(&self) -> DateTime<Utc> {
        self.client.state.get_server_datetime().await
    }

    /// Gets the current assets.
    pub async fn assets(&self) -> Option<Assets> {
        let state = &self.client.state;
        let assets = state.assets.read().await;
        if let Some(assets) = assets.as_ref() {
            return Some(assets.clone());
        }
        None
    }

    /// Waits for the assets to be loaded from the server.
    /// # Arguments
    /// * `timeout` - The maximum time to wait for assets to be loaded.
    /// # Returns
    /// `Ok(())` if assets are loaded, or an error if the timeout is reached.
    pub async fn wait_for_assets(&self, timeout: Duration) -> PocketResult<()> {
        let start = std::time::Instant::now();
        loop {
            if self.assets().await.is_some() {
                return Ok(());
            }
            if start.elapsed() > timeout {
                return Err(PocketError::General("Timeout waiting for assets".to_string()));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Checks the result of a trade by its ID.
    /// # Arguments
    /// * `id` - The ID of the trade to check.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if the trade fails.
    pub async fn result(&self, id: Uuid) -> PocketResult<Deal> {
        self.require_handle::<DealsApiModule>("DealsApiModule")
            .await?
            .check_result(id)
            .await
    }

    /// Checks the result of a trade by its ID with a timeout.
    /// # Arguments
    /// * `id` - The ID of the trade to check.
    /// * `timeout` - The duration to wait before timing out.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if the trade fails.
    pub async fn result_with_timeout(&self, id: Uuid, timeout: Duration) -> PocketResult<Deal> {
        self.require_handle::<DealsApiModule>("DealsApiModule")
            .await?
            .check_result_with_timeout(id, timeout)
            .await
    }

    /// Gets the currently opened deals.
    pub async fn get_opened_deals(&self) -> HashMap<Uuid, Deal> {
        self.client.state.trade_state.get_opened_deals().await
    }

    /// Gets the currently closed deals.
    pub async fn get_closed_deals(&self) -> HashMap<Uuid, Deal> {
        self.client.state.trade_state.get_closed_deals().await
    }
    /// Clears the currently closed deals.
    pub async fn clear_closed_deals(&self) {
        self.client.state.trade_state.clear_closed_deals().await
    }

    /// Gets a specific opened deal by its ID.
    pub async fn get_opened_deal(&self, deal_id: Uuid) -> Option<Deal> {
        self.client.state.trade_state.get_opened_deal(deal_id).await
    }

    /// Gets a specific closed deal by its ID.
    pub async fn get_closed_deal(&self, deal_id: Uuid) -> Option<Deal> {
        self.client.state.trade_state.get_closed_deal(deal_id).await
    }

    /// Opens a pending order.
    /// # Arguments
    /// * `open_type` - The type of the pending order.
    /// * `amount` - The amount to trade.
    /// * `asset` - The asset to trade.
    /// * `open_time` - The time to open the trade.
    /// * `open_price` - The price to open the trade at.
    /// * `timeframe` - The duration of the trade.
    /// * `min_payout` - The minimum payout percentage.
    /// * `command` - The trade direction (0 for Call, 1 for Put).
    /// # Returns
    /// A `PocketResult` containing the `PendingOrder` if successful, or an error if the trade fails.
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
    ) -> PocketResult<PendingOrder> {
        self.require_handle::<PendingTradesApiModule>("PendingTradesApiModule")
            .await?
            .open_pending_order(
                open_type, amount, asset, open_time, open_price, timeframe, min_payout, command,
            )
            .await
    }

    /// Gets the currently pending deals.
    /// # Returns
    /// A `HashMap` containing the pending deals, keyed by their UUID.
    pub async fn get_pending_deals(&self) -> HashMap<Uuid, PendingOrder> {
        self.client.state.trade_state.get_pending_deals().await
    }

    /// Gets a specific pending deal by its ID.
    /// # Arguments
    /// * `deal_id` - The ID of the pending deal to retrieve.
    /// # Returns
    /// An `Option` containing the `PendingOrder` if found, or `None` otherwise.
    pub async fn get_pending_deal(&self, deal_id: Uuid) -> Option<PendingOrder> {
        self.client
            .state
            .trade_state
            .get_pending_deal(deal_id)
            .await
    }

    /// Subscribes to a specific asset's updates.
    pub async fn subscribe(
        &self,
        asset: impl ToString,
        sub_type: SubscriptionType,
    ) -> PocketResult<SubscriptionStream> {
        let handle = self.require_handle::<SubscriptionsApiModule>("SubscriptionsApiModule").await?;
        let assets = self.assets().await.ok_or_else(|| {
            BinaryOptionsError::General("Assets not found".into())
        })?;

        if assets.get(&asset.to_string()).is_some() {
            handle.subscribe(asset.to_string(), sub_type).await
        } else {
            Err(PocketError::InvalidAsset(asset.to_string()))
        }
    }

    /// Unsubscribes from a specific asset's real-time updates.
    ///
    /// # Arguments
    /// * `asset` - The asset symbol to unsubscribe from.
    ///
    /// # Returns
    /// A `PocketResult` indicating success or an error if the unsubscribe operation fails.
    pub async fn unsubscribe(&self, asset: impl ToString) -> PocketResult<()> {
        let handle = self.require_handle::<SubscriptionsApiModule>("SubscriptionsApiModule").await?;
        let assets = self.assets().await.ok_or_else(|| {
            BinaryOptionsError::General("Assets not found".into())
        })?;

        if assets.get(&asset.to_string()).is_some() {
            handle.unsubscribe(asset.to_string()).await
        } else {
            Err(PocketError::InvalidAsset(asset.to_string()))
        }
    }

    /// Gets historical candle data for a specific asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD_otc")
    /// * `period` - Time period for each candle in seconds
    /// * `time` - Current time timestamp
    /// * `offset` - Number of periods to offset from current time
    ///
    /// # Returns
    /// A vector of Candle objects containing historical price data
    ///
    /// # Errors
    /// * Returns InvalidAsset if the asset is not found
    /// * Returns ModuleNotFound if GetCandlesApiModule is not available
    /// * Returns General error for other failures
    pub async fn get_candles_advanced(
        &self,
        asset: impl ToString,
        period: i64,
        time: i64,
        offset: i64,
    ) -> PocketResult<Vec<Candle>> {
        let handle = self.require_handle::<GetCandlesApiModule>("GetCandlesApiModule").await?;

        if let Some(assets) = self.assets().await {
            if assets.get(&asset.to_string()).is_none() {
                return Err(PocketError::InvalidAsset(asset.to_string()));
            }
        }
        // If assets are not loaded yet, still try to get candles
        handle
            .get_candles_advanced(asset, period, time, offset)
            .await
    }

    /// Gets historical candle data with advanced parameters.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD_otc")
    /// * `period` - Time period for each candle in seconds
    /// * `offset` - Number of periods to offset from current time
    ///
    /// # Returns
    /// A vector of Candle objects containing historical price data
    ///
    /// # Errors
    /// * Returns InvalidAsset if the asset is not found
    /// * Returns ModuleNotFound if GetCandlesApiModule is not available
    /// * Returns General error for other failures
    pub async fn get_candles(
        &self,
        asset: impl ToString,
        period: i64,
        offset: i64,
    ) -> PocketResult<Vec<Candle>> {
        let handle = self.require_handle::<GetCandlesApiModule>("GetCandlesApiModule").await?;

        if let Some(assets) = self.assets().await {
            if assets.get(&asset.to_string()).is_none() {
                return Err(PocketError::InvalidAsset(asset.to_string()));
            }
        }
        // If assets are not loaded yet, still try to get candles
        handle.get_candles(asset, period, offset).await
    }

    /// Gets historical tick data (timestamp, price) for a specific asset and period.
    /// # Arguments
    /// * `asset` - The asset to get historical data for.
    /// * `period` - The time period for each tick in seconds.
    /// # Returns
    /// A `PocketResult` containing a vector of `(timestamp, price)` if successful, or an error if the request fails.
    pub async fn ticks(&self, asset: impl ToString, period: u32) -> PocketResult<Vec<(f64, f64)>> {
        let handle = self.require_handle::<HistoricalDataApiModule>("HistoricalDataApiModule").await?;

        if let Some(assets) = self.assets().await {
            if assets.get(&asset.to_string()).is_none() {
                return Err(PocketError::InvalidAsset(asset.to_string()));
            }
        }
        handle.ticks(asset.to_string(), period).await
    }

    /// Gets historical candle data for a specific asset and period.
    /// # Arguments
    /// * `asset` - The asset to get historical data for.
    /// * `period` - The time period for each candle in seconds.
    /// # Returns
    /// A `PocketResult` containing a vector of `Candle` if successful, or an error if the request fails.
    pub async fn candles(&self, asset: impl ToString, period: u32) -> PocketResult<Vec<Candle>> {
        let handle = self.require_handle::<HistoricalDataApiModule>("HistoricalDataApiModule").await?;

        if let Some(assets) = self.assets().await {
            if assets.get(&asset.to_string()).is_none() {
                return Err(PocketError::InvalidAsset(asset.to_string()));
            }
        }
        handle.candles(asset.to_string(), period).await
    }

    /// Gets historical candle data for a specific asset and period.
    /// Deprecated: use `candles()` instead.
    pub async fn history(&self, asset: impl ToString, period: u32) -> PocketResult<Vec<Candle>> {
        self.candles(asset, period).await
    }

    pub async fn get_handle<M: ApiModule<State>>(&self) -> Option<M::Handle> {
        self.client.get_handle::<M>().await
    }

    /// Disconnects the client while keeping the configuration intact.
    /// The connection can be re-established later using `connect()`.
    /// This is useful for temporarily closing the connection without losing credentials or settings.
    pub async fn disconnect(&self) -> PocketResult<()> {
        self.client.disconnect().await.map_err(PocketError::from)
    }

    /// Establishes a connection after a manual disconnect.
    /// This will reconnect using the same configuration and credentials.
    pub async fn connect(&self) -> PocketResult<()> {
        self.client.reconnect().await.map_err(PocketError::from)
    }

    /// Disconnects and reconnects the client.
    pub async fn reconnect(&self) -> PocketResult<()> {
        self.client.reconnect().await.map_err(PocketError::from)
    }

    /// Shuts down the client and stops the runner.
    pub async fn shutdown(self) -> PocketResult<()> {
        self.client.shutdown().await.map_err(PocketError::from)
    }

    pub async fn new_testing_wrapper(ssid: impl ToString) -> PocketResult<TestingWrapper<State>> {
        let pocket_builder = Self::builder(ssid)?;
        let builder = TestingWrapperBuilder::new()
            .with_stats_interval(Duration::from_secs(10))
            .with_log_stats(true)
            .with_track_events(true)
            .with_max_reconnect_attempts(Some(3))
            .with_reconnect_delay(Duration::from_secs(5))
            .with_connection_timeout(Duration::from_secs(30))
            .with_auto_reconnect(true)
            .build_with_middleware(pocket_builder)
            .await?;

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use crate::pocketoption::candle::SubscriptionType;
    use core::time::Duration;
    use futures_util::StreamExt;

    use super::PocketOption;

    #[tokio::test]
    async fn test_pocket_option_tester() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"a0e4f10b17cd7a8125bece49f1364c28\";s:10:\"ip_address\";s:13:\"186.41.20.143\";s:10:\"user_agent\";s:101:\"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36\";s:13:\"last_activity\";i:1752283410;}0f18b73ad560f70cd3e02eb7b3242f9f","isDemo":0,"uid":79165265,"platform":3,"isFastHistory":true,"isOptimized":true}]	"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let mut tester = PocketOption::new_testing_wrapper(ssid).await.unwrap();
        tester.start().await.unwrap();
        tokio::time::sleep(Duration::from_secs(120)).await; // Wait for 2 minutes to allow the client to run and process messages
        println!("{}", tester.stop().await.unwrap().summary());
    }

    #[tokio::test]
    async fn test_pocket_option_balance() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"gchu4nm327s30oiglrenfshr96","isDemo":1,"uid":115353941,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let balance = api.balance().await;
        println!("Balance: {balance}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_server_time() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751681442;}e2cf2ff21c927851dbb4a781aa81a10e","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let server_time = api.client.state.get_server_datetime().await;
        println!("Server Time: {server_time}");
        println!(
            "Server time complete: {}",
            api.client.state.server_time.read().await
        );
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_buy_sell() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"gchu4nm327s30oiglrenfshr96","isDemo":1,"uid":115353941,"platform":2,"isFastHistory":true,"isOptimized":true}]		"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let buy_result = api.buy("EURUSD_otc", 3, 1.0).await.unwrap();
        println!("Buy Result: {buy_result:?}");
        let sell_result = api.sell("EURUSD_otc", 3, 1.0).await.unwrap();
        println!("Sell Result: {sell_result:?}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_result() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"gchu4nm327s30oiglrenfshr96","isDemo":1,"uid":115353941,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let (buy_id, _) = api.buy("EURUSD", 60, 1.0).await.unwrap();
        let (sell_id, _) = api.sell("EURUSD", 60, 1.0).await.unwrap();

        let buy_result = api.result(buy_id).await.unwrap();
        println!("Result ID: {buy_id}, Result: {buy_result:?}");
        tokio::time::sleep(Duration::from_secs(5)).await; // Wait for the trade to be complete to test retrieving the trade form the list of closed trades
        let sell_result = api.result(sell_id).await.unwrap();
        println!("Result ID: {sell_id}, Result: {sell_result:?}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_subscription() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"a0e4f10b17cd7a8125bece49f1364c28\";s:10:\"ip_address\";s:13:\"186.41.20.143\";s:10:\"user_agent\";s:101:\"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36\";s:13:\"last_activity\";i:1752283410;}0f18b73ad560f70cd3e02eb7b3242f9f","isDemo":0,"uid":79165265,"platform":3,"isFastHistory":true,"isOptimized":true}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages

        let subscription = api
            .subscribe(
                "AUDUSD_otc",
                SubscriptionType::time_aligned(Duration::from_secs(5)).unwrap(),
            )
            .await
            .unwrap();
        let mut stream = subscription.to_stream();
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(msg) => println!("Received subscription message: {msg:?}"),
                Err(e) => println!("Error in subscription: {e}"),
            }
        }
        api.unsubscribe("AUDUSD_otc").await.unwrap();
        println!("Unsubscribed from AUDUSD_otc");

        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_get_candles() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"a1b0c4986eb221b5530428dbbdb6b796\";s:10:\"ip_address\";s:14:\"191.113.147.46\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36 OPR/120.\";s:13:\"last_activity\";i:1754424804;}e3c483184de3f99e5f806db7d92c1cac","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages

        let current_time = chrono::Utc::now().timestamp();
        let candles = api
            .get_candles_advanced("EURCHF_otc", 5, current_time, 1000)
            .await
            .unwrap();
        println!("Received {} candles", candles.len());
        for (i, candle) in candles.iter().take(5).enumerate() {
            println!("Candle {i}: {candle:?}");
        }

        let candles_advanced = api.get_candles("EURCHF_otc", 5, 1000).await.unwrap();
        println!("Received {} candles (advanced)", candles_advanced.len());

        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_history() {
        let _ = tracing_subscriber::fmt::try_init();
        let ssid = r#"42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages

        let history = api.history("EURCHF_otc", 5).await.unwrap();
        println!("Received {} candles from history", history.len());
        for (i, candle) in history.iter().take(5).enumerate() {
            println!("Candle {i}: {candle:?}");
        }

        api.shutdown().await.unwrap();
    }
}
