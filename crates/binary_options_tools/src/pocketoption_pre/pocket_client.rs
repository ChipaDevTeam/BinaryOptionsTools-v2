use std::{collections::HashMap, sync::Arc, time::Duration};

use binary_options_tools_core_pre::{
    builder::ClientBuilder,
    client::Client,
    error::CoreError,
    testing::{TestingWrapper, TestingWrapperBuilder},
    traits::ApiModule,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::pocketoption_pre::{
    candle::SubscriptionType,
    connect::PocketConnect,
    error::{PocketError, PocketResult},
    modules::{
        assets::AssetsModule,
        balance::BalanceModule,
        deals::DealsApiModule,
        keep_alive::{InitModule, KeepAliveModule},
        print_handler,
        server_time::ServerTimeModule,
        subscriptions::{SubscriptionStream, SubscriptionsApiModule},
        trades::TradesApiModule,
    },
    ssid::Ssid,
    state::{State, StateBuilder},
    types::{Action, Assets, Deal},
};

const MINIMUM_TRADE_AMOUNT: f64 = 1.0;
const MAXIMUM_TRADE_AMOUNT: f64 = 20000.0;

/// PocketOption client for interacting with the PocketOption trading platform.
///
/// This client provides methods for trading, checking balances, subscribing to
/// asset updates, and managing the connection to the PocketOption platform.
///
/// # Example
/// ```
/// use binary_options_tools_pocketoption_pre::PocketOption;
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
}

impl PocketOption {
    fn builder(ssid: impl ToString) -> PocketResult<ClientBuilder<State>> {
        let state = StateBuilder::default().ssid(Ssid::parse(ssid)?).build()?;

        Ok(ClientBuilder::new(PocketConnect, state)
            .with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state)))
            .with_lightweight_module::<KeepAliveModule>()
            .with_lightweight_module::<InitModule>()
            .with_lightweight_module::<BalanceModule>()
            .with_lightweight_module::<ServerTimeModule>()
            .with_lightweight_module::<AssetsModule>()
            .with_module::<TradesApiModule>()
            .with_module::<DealsApiModule>()
            .with_module::<SubscriptionsApiModule>()
            .with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state))))
    }

    pub async fn new(ssid: impl ToString) -> PocketResult<Self> {
        let state = StateBuilder::default()
            .ssid(Ssid::parse(ssid)?)
            .build()?;
        let builder = ClientBuilder::new(PocketConnect, state)
            .with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state)))
            .with_lightweight_module::<KeepAliveModule>()
            .with_lightweight_module::<InitModule>()
            .with_lightweight_module::<BalanceModule>()
            .with_lightweight_module::<ServerTimeModule>()
            .with_lightweight_module::<AssetsModule>()
            .with_module::<TradesApiModule>()
            .with_module::<DealsApiModule>()
            .with_module::<SubscriptionsApiModule>()
            .with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state)));
        let (client, mut runner) = builder.build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });
        client.wait_connected().await;

        Ok(Self {
            client,
            _runner: Arc::new(_runner),
        })
    }

    pub async fn new_with_url(ssid: impl ToString, url: String) -> PocketResult<Self> {
        let state = StateBuilder::default()
            .ssid(Ssid::parse(ssid)?)
            .default_connection_url(url)
            .build()?;
        let (client, mut runner) = ClientBuilder::new(PocketConnect, state).build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });

        Ok(Self {
            client,
            _runner: Arc::new(_runner),
        })
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

    pub fn is_demo(&self) -> bool {
        let state = &self.client.state;
        state.ssid.demo()
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
        if let Some(assets) = self.assets().await {
            assets.validate(&asset.to_string(), time)?;
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
            if let Some(handle) = self.client.get_handle::<TradesApiModule>().await {
                handle
                    .trade(asset.to_string(), action, amount, time)
                    .await
                    .map(|d| (d.id, d))
            } else {
                Err(CoreError::ModuleNotFound("TradesApiModule".into()).into())
            }
        } else {
            Err(PocketError::General("Assets not loaded".to_string()))
        }
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

    /// Checks the result of a trade by its ID.
    /// # Arguments
    /// * `id` - The ID of the trade to check.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if the trade fails.
    pub async fn result(&self, id: Uuid) -> PocketResult<Deal> {
        if let Some(handle) = self.client.get_handle::<DealsApiModule>().await {
            handle.check_result(id).await
        } else {
            Err(CoreError::ModuleNotFound("DealsApiModule".into()).into())
        }
    }

    /// Checks the result of a trade by its ID with a timeout.
    /// # Arguments
    /// * `id` - The ID of the trade to check.
    /// * `timeout` - The duration to wait before timing out.
    /// # Returns
    /// A `PocketResult` containing the `Deal` if successful, or an error if the trade fails.
    pub async fn result_with_timeout(&self, id: Uuid, timeout: Duration) -> PocketResult<Deal> {
        if let Some(handle) = self.client.get_handle::<DealsApiModule>().await {
            handle.check_result_with_timeout(id, timeout).await
        } else {
            Err(CoreError::ModuleNotFound("DealsApiModule".into()).into())
        }
    }

    /// Gets the currently opened deals.
    pub async fn get_opened_deals(&self) -> HashMap<Uuid, Deal> {
        self.client.state.trade_state.get_opened_deals().await
    }

    pub async fn subscribe(
        &self,
        asset: impl ToString,
        sub_type: SubscriptionType,
    ) -> PocketResult<SubscriptionStream> {
        if let Some(handle) = self.client.get_handle::<SubscriptionsApiModule>().await
            && let Some(assets) = self.assets().await
        {
            if assets.get(&asset.to_string()).is_some() {
                handle.subscribe(asset.to_string(), sub_type).await
            } else {
                Err(PocketError::InvalidAsset(asset.to_string()))
            }
        } else {
            Err(CoreError::ModuleNotFound("SubscriptionsApiModule".into()).into())
        }
    }

    pub async fn unsubscribe(&self, asset: impl ToString) -> PocketResult<()> {
        if let Some(handle) = self.client.get_handle::<SubscriptionsApiModule>().await
            && let Some(assets) = self.assets().await
        {
            if let Some(_) = assets.get(&asset.to_string()) {
                handle.unsubscribe(asset.to_string()).await
            } else {
                Err(PocketError::InvalidAsset(asset.to_string()))
            }
        } else {
            Err(CoreError::ModuleNotFound("SubscriptionsApiModuel".into()).into())
        }
    }

    pub async fn get_handle<M: ApiModule<State>>(&self) -> Option<M::Handle> {
        self.client.get_handle::<M>().await
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
    use crate::pocketoption_pre::candle::SubscriptionType;
    use core::time::Duration;
    use futures_util::StreamExt;

    use super::PocketOption;

    #[tokio::test]
    async fn test_pocket_option_tester() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"a0e4f10b17cd7a8125bece49f1364c28\";s:10:\"ip_address\";s:13:\"186.41.20.143\";s:10:\"user_agent\";s:101:\"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36\";s:13:\"last_activity\";i:1752283410;}0f18b73ad560f70cd3e02eb7b3242f9f","isDemo":0,"uid":79165265,"platform":3,"isFastHistory":true,"isOptimized":true}]	"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let mut tester = PocketOption::new_testing_wrapper(ssid).await.unwrap();
        tester.start().await.unwrap();
        tokio::time::sleep(Duration::from_secs(120)).await; // Wait for 2 minutes to allow the client to run and process messages
        println!("{}", tester.stop().await.unwrap().summary());
    }

    #[tokio::test]
    async fn test_pocket_option_balance() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751681442;}e2cf2ff21c927851dbb4a781aa81a10e","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let balance = api.balance().await;
        println!("Balance: {balance}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_server_time() {
        tracing_subscriber::fmt::init();
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
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let buy_result = api.buy("EURUSD_otc", 60, 1.0).await.unwrap();
        println!("Buy Result: {buy_result:?}");
        let sell_result = api.sell("EURUSD_otc", 60, 1.0).await.unwrap();
        println!("Sell Result: {sell_result:?}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_result() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"dttf3u62d2kb6v888pjkte4ug6","isDemo":1,"uid":79165265,"platform":3,"isFastHistory":true,"isOptimized":true}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let (buy_id, _) = api.buy("EURUSD_otc", 60, 1.0).await.unwrap();
        let (sell_id, _) = api.sell("EURUSD_otc", 60, 1.0).await.unwrap();

        let buy_result = api.result(buy_id).await.unwrap();
        println!("Result ID: {buy_id}, Result: {buy_result:?}");
        tokio::time::sleep(Duration::from_secs(5)).await; // Wait for the trade to be complete to test retrieving the trade form the list of closed trades
        let sell_result = api.result(sell_id).await.unwrap();
        println!("Result ID: {sell_id}, Result: {sell_result:?}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_subscription() {
        tracing_subscriber::fmt::init();
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
}
