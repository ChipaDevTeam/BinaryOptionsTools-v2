use binary_options_tools_core_pre::traits::AppState;
use chrono::Local;
use rust_decimal::{Decimal, dec};
use tokio::sync::RwLock;

use crate::expertoptions::{modules::profile::Demo, types::Assets};

pub struct Config {
    pub user_agent: String,
}


pub struct Balance {
    pub real: Decimal,
    pub demo: Decimal
}

pub struct State {
    /// Session ID for the account
    pub token: String,
    /// Balance of the account
    pub balance: RwLock<Option<Balance>>,
    /// Indicates if the account is a demo account
    pub demo: RwLock<Demo>,
    /// Configuration for the ExpertOptions client
    pub config: RwLock<Config>,
    /// Current timezone (UTC offset)
    pub timezone: RwLock<i32>,
    /// Get candles allowed timeframes
    pub get_candles_timeframes: RwLock<Vec<u32>>,
    /// Maps how often point data is returned by server
    pub points_timeframe: RwLock<Decimal>,
    /// Assets 
    pub assets: RwLock<Option<Assets>>,

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
        let timezone = Local::now()
            .offset()
            .local_minus_utc().div_euclid(60);
        dbg!(timezone);
        State {
            token,
            balance: RwLock::new(None),
            demo: RwLock::new(Demo::new(demo)),
            config: RwLock::new(Config::default()),
            timezone: RwLock::new(timezone), // Default to UTC
            get_candles_timeframes: RwLock::new(Vec::new()),
            assets: RwLock::new(None),
            points_timeframe: RwLock::new(dec!(0.5)), // Default to .5 seconds
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

    pub async fn set_assets(&self, assets: Assets) {
        *self.assets.write().await = Some(assets);
    }

    pub async fn set_balance(&self, balance: Balance) {
        *self.balance.write().await = Some(balance);
    }

    pub async fn set_timeframes(&self, candles: Vec<u32>, points: Decimal) {
        *self.get_candles_timeframes.write().await = candles;
        *self.points_timeframe.write().await = points;
    }

    pub async fn get_balance(&self) -> Decimal {
        let demo = self.is_demo().await;
        match &*self.balance.read().await {
            Some(balance) => {
                if demo {
                    balance.demo
                } else {
                    balance.real
                }
            },
            None => dec!(-1)
        }
    }

    pub async fn get_points_timeframe(&self) -> Decimal {
        *self.points_timeframe.read().await
    }

    pub async fn validate_candle_timeframe(&self, timeframe: u32) -> bool {
        self.get_candles_timeframes.read().await.contains(&timeframe)
    }
}
