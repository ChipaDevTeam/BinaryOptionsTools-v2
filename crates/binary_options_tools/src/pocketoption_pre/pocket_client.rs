use std::time::Duration;

use binary_options_tools_core_pre::{builder::ClientBuilder, client::Client, error::CoreError, testing::{TestingWrapper, TestingWrapperBuilder}};

use crate::pocketoption_pre::{connect::PocketConnect, error::{PocketError, PocketResult}, modules::{balance::BalanceModule, keep_alive::{InitModule, KeepAliveModule}, print_handler, server_time::ServerTimeModule}, ssid::Ssid, state::{State, StateBuilder}};


pub struct PocketOption {
    client: Client<State>,
    _runner: tokio::task::JoinHandle<()>
}

impl PocketOption {
    fn builder(ssid: impl ToString) -> PocketResult<ClientBuilder<State>> {
    let state = StateBuilder::default().ssid(Ssid::parse(ssid)?).build()?;

    Ok(ClientBuilder::new(PocketConnect, state)
                .with_lightweight_module::<KeepAliveModule>()
                .with_lightweight_module::<InitModule>()
                .with_lightweight_module::<BalanceModule>()
                .with_lightweight_module::<ServerTimeModule>()
                .with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state))))

    }

    pub async fn new(ssid: impl ToString) -> PocketResult<Self> {
        let (client, mut runner) = Self::builder(ssid)?
            .build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });
        client.wait_connected().await;

        Ok(Self {
                    client,
                    _runner,
                })
    }

    pub async fn new_with_url(ssid: impl ToString, url: String) -> PocketResult<Self> {
        let state = StateBuilder::default()
            .ssid(Ssid::parse(ssid)?)
            .default_connection_url(url)
            .build()?;
        let (client, mut runner) = ClientBuilder::new(PocketConnect, state)
            .build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });

        Ok(Self {
                    client,
                    _runner,
                })
    }

    /// Gets the current balance of the user.
    /// If the balance is not set, it returns -1.
    ///
    pub fn balance(&self) -> PocketResult<f64> {
        let state = &self.client.state;
        let balance = state.balance.read().map_err(|e| PocketError::from(CoreError::Poison(e.to_string())))?;
        if let Some(balance) = *balance {
            return Ok(balance);
        }
        Ok(-1.0)
    }

    /// Shuts down the client and stops the runner.
    ///
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
    use std::time::Duration;

    use super::PocketOption;

    #[tokio::test]
    async fn test_pocket_option_new() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751057233;}b9d0db50cb32d406f935c63a41484f27","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let mut tester = PocketOption::new_testing_wrapper(ssid).await.unwrap();
        tester.start().await.unwrap();
        tokio::time::sleep(Duration::from_secs(120)).await; // Wait for 2 minutes to allow the client to run and process messages
        println!("{}", tester.stop().await.unwrap().summary());
    }

    #[tokio::test]
    async fn test_pocket_option_balance() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"3e1823a796c1fe61491e309136cf9861\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751681442;}e2cf2ff21c927851dbb4a781aa81a10e","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let balance = api.balance().unwrap();
        println!("Balance: {balance}");
        api.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_pocket_option_server_time() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"3e1823a796c1fe61491e309136cf9861\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751681442;}e2cf2ff21c927851dbb4a781aa81a10e","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]"#; // 42["auth",{"session":"g011qsjgsbgnqcfaj54rkllk6m","isDemo":1,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	
        let api = PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await; // Wait for the client to connect and process messages
        let server_time = api.client.state.get_server_datetime().await;
        println!("Server Time: {server_time}");
        println!("Server time complete: {}", api.client.state.server_time.read().await);
        api.shutdown().await.unwrap();
    }
}