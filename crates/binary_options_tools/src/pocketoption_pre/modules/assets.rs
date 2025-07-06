use std::collections::HashMap;
use std::sync::Arc;

use crate::pocketoption_pre::{state::State, types::TwoStepRule};
use async_trait::async_trait;
use binary_options_tools_core::reimports::Message;
use binary_options_tools_core_pre::{
    error::CoreResult,
    reimports::{AsyncReceiver, AsyncSender},
    traits::{LightweightModule, Rule},
};
use serde::{Deserialize, Deserializer};

/// CandleLength is a wrapper around u32 for allowed candle durations (in seconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct CandleLength {
    time: u32
}

impl From<u32> for CandleLength {
    fn from(val: u32) -> Self {
        CandleLength { time: val }
    }
}
impl From<CandleLength> for u32 {
    fn from(val: CandleLength) -> u32 {
        val.time
    }
}

/// Asset struct for processed asset data
#[derive(Debug, Clone)]
pub struct Asset {
    pub id: i32, // This field is not used in the current implementation but can be useful for debugging
    pub name: String,
    pub symbol: String,
    pub is_otc: bool,
    pub payout: i32,
    pub allowed_candles: Vec<CandleLength>,
    pub asset_type: AssetType,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Stock,
    Currency,
    Commodity,
    Cryptocurrency,
    Index,
}


impl<'de> Deserialize<'de> for Asset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Asset {
            id: i32,
            symbol: String,
            name: String,
            asset_type: AssetType,
            in1: i32,
            payout: i32,
            in3: i32,
            in4: i32,
            in5: i32,
            in6: i32,
            in7: i32,
            in8: i32,
            arr: Vec<String>,
            in9: i64,
            val: bool,
            times: Vec<CandleLength>,
            in10: i32,
            in11: i32,
            in12: i64,
        }

        todo!()
    }
}

/// Helper struct for parsing allowed_candles
#[derive(Debug, Deserialize)]
struct RawCandleTime {
    time: u32,
}

/// Wrapper around HashMap<String, Asset>
#[derive(Debug, Default, Clone)]
pub struct Assets(pub HashMap<String, Asset>);

impl Assets {
    pub fn get(&self, symbol: &str) -> Option<&Asset> {
        self.0.get(symbol)
    }
}

impl<'de> Deserialize<'de> for Assets {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let assets: Vec<Asset> = Vec::deserialize(deserializer)?;
        let map = assets.into_iter().map(|a| (a.symbol.clone(), a)).collect();
        Ok(Assets(map))
    }
}

/// Module for handling asset updates in PocketOption
/// This module listens for asset-related messages and processes them accordingly.
/// It is designed to work with the PocketOption trading platform's WebSocket API.
/// It checks from the assets payouts, the length of the candles it can have, if the asset is opened or not, etc...
pub struct AssetsModule {
    state: Arc<State>,
    receiver: AsyncReceiver<Arc<Message>>,
}

#[async_trait]
impl LightweightModule<State> for AssetsModule {
    fn new(
        state: Arc<State>,
        _: AsyncSender<Message>,
        receiver: AsyncReceiver<Arc<Message>>,
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        // Example: receive a message, parse assets, and update state.assets
        // let msg = self.receiver.recv().await?;
        // let assets: Assets = serde_json::from_value(msg.payload.clone())?;
        // self.state.assets.write().unwrap().replace(assets);
        todo!()
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(TwoStepRule::new(r#"451-["updateAssets","#))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_deserialization() {
        let json = r#"[
    5,
    "AAPL",
    "Apple",
    "stock",
    2,
    50,
    60,
    30,
    3,
    0,
    170,
    0,
    [],
    1751906100,
    false,
    [
      { "time": 60 },
      { "time": 120 },
      { "time": 180 },
      { "time": 300 },
      { "time": 600 },
      { "time": 900 },
      { "time": 1800 },
      { "time": 2700 },
      { "time": 3600 },
      { "time": 7200 },
      { "time": 10800 },
      { "time": 14400 }
    ],
    -1,
    60,
    1751906100
  ]"#;

        let asset: Asset = dbg!(serde_json::from_str(json).unwrap());
        assert_eq!(asset.id, 1);
        assert_eq!(asset.symbol, "AAPL");
        assert_eq!(asset.name, "Apple");
        assert!(!asset.is_otc);
        assert_eq!(asset.payout, 60);
        assert_eq!(asset.allowed_candles.len(), 3);
        // assert_eq!(asset.allowed_candles[0].0, 60);
    }
}