use binary_options_tools_core_pre::traits::AppState;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use crate::expertoptions::modules::profile::Demo;

pub struct Config {
    pub user_agent: String,
}

pub struct State {
    /// Session ID for the account
    pub token: String,
    /// Balance of the account
    pub balance: RwLock<Option<Decimal>>,
    /// Indicates if the account is a demo account
    pub demo: RwLock<Demo>,
    /// Configuration for the ExpertOptions client
    pub config: RwLock<Config>,
}

impl Config {
    pub fn new(user_agent: String) -> Self {
        Config { user_agent }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36 OPR/120.0.0.0".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AppState for State {
    async fn clear_temporal_data(&self) {
        // Clear any temporary data associated with the state

    }
}

impl State {
    pub fn new(token: String, demo: bool) -> Self {
        State {
            token,
            balance: RwLock::new(None),
            demo: RwLock::new(Demo::new(demo)),
            config: RwLock::new(Config::default()),
        }
    }
    
    pub async fn set_demo(&self, demo: Demo) {
        *self.demo.write().await = demo;
    }

    pub async fn is_demo(&self) -> bool {
        self.demo.read().await.is_demo()
    }

    pub async fn user_agent(&self) -> String {
        self.config.read().await.user_agent.clone()
    }
}