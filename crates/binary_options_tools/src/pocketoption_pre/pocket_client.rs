use binary_options_tools_core_pre::{builder::ClientBuilder, client::Client};

use crate::pocketoption_pre::{connect::PocketConnect, error::PocketResult, modules::{keep_alive::{InitModule, KeepAliveModule}, print_handler}, ssid::Ssid, state::{State, StateBuilder}};


pub struct PocketOption {
    client: Client<State>,
    _runner: tokio::task::JoinHandle<()>
}

impl PocketOption {
    pub async fn new(ssid: impl ToString) -> PocketResult<Self> {
        let state = StateBuilder::default().ssid(Ssid::parse(ssid)?).build()?;
        let (client, mut runner) = ClientBuilder::new(PocketConnect, state)
            .with_lightweight_module::<KeepAliveModule>()
            .with_lightweight_module::<InitModule>()
            .with_lightweight_handler(|msg, state, _| Box::pin(print_handler(msg, state)))
            .build().await?;

        let _runner = tokio::spawn(async move { runner.run().await });

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
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::PocketOption;

    #[tokio::test]
    async fn test_pocket_option_new() {
        tracing_subscriber::fmt::init();
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"be8de3a8cb5fed23efebb631902263e2\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751057233;}b9d0db50cb32d406f935c63a41484f27","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#;
        PocketOption::new(ssid).await.unwrap();
        tokio::time::sleep(Duration::from_secs(120)).await; // Allow time for the client to run
    }
}