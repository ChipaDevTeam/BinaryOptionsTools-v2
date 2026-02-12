use std::sync::Arc;

use crate::pocketoption::{state::State, types::Assets};
use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use tracing::{debug, warn};

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
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            match &*msg {
                Message::Binary(data) => {
                    if let Ok(assets) = serde_json::from_slice::<Assets>(data) {
                        debug!("Loaded assets (binary): {:?}", assets.names());
                        self.state.set_assets(assets).await;
                    } else {
                        warn!("Failed to parse assets message (binary): {:?}", data);
                    }
                }
                Message::Text(text) => {
                    if let Ok(assets) = serde_json::from_str::<Assets>(text) {
                        debug!("Loaded assets (text): {:?}", assets.names());
                        self.state.set_assets(assets).await;
                    } else {
                        // Try to parse as a 1-step Socket.IO message: 42["updateAssets", [...]]
                        let mut parsed_1step = false;
                        if let Some(start) = text.find('[') {
                            if let Ok(value) =
                                serde_json::from_str::<serde_json::Value>(&text[start..])
                            {
                                if let Some(arr) = value.as_array() {
                                    if arr.len() >= 2 && arr[0] == "updateAssets" {
                                        if let Ok(assets) =
                                            serde_json::from_value::<Assets>(arr[1].clone())
                                        {
                                            debug!(
                                                "Loaded assets (text 1-step): {:?}",
                                                assets.names()
                                            );
                                            self.state.set_assets(assets).await;
                                            parsed_1step = true;
                                        }
                                    }
                                }
                            }
                        }
                        if !parsed_1step {
                            // It might be the header message, which we ignore in the run loop
                            // since TwoStepRule already matched it.
                        }
                    }
                }
                _ => {}
            }
        }
        Err(CoreError::LightweightModuleLoop("AssetsModule".into()))
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(crate::pocketoption::types::MultiPatternRule::new(vec![
            "updateAssets",
        ]))
    }
}

#[cfg(test)]
mod tests {
    use crate::pocketoption::types::{Asset, AssetType, Assets, CandleLength};

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
        assert_eq!(asset.id, 5);
        assert_eq!(asset.symbol, "AAPL");
        assert_eq!(asset.name, "Apple");
        assert!(!asset.is_otc);
        assert_eq!(asset.payout, 50);
        assert_eq!(asset.allowed_candles.len(), 12);
        assert_eq!(asset.allowed_candles[0].duration(), 60);
    }

    #[test]
    fn test_assets_active_filtering() {
        // Create a mix of active and inactive assets
        let asset1 = Asset {
            id: 1,
            symbol: "AAPL".to_string(),
            name: "Apple".to_string(),
            asset_type: AssetType::Stock,
            payout: 50,
            is_otc: false,
            is_active: true,
            allowed_candles: vec![CandleLength::new(60)],
        };
        let asset2 = Asset {
            id: 2,
            symbol: "GOOGL".to_string(),
            name: "Google".to_string(),
            asset_type: AssetType::Stock,
            payout: 50,
            is_otc: false,
            is_active: false,
            allowed_candles: vec![CandleLength::new(60)],
        };
        let asset3 = Asset {
            id: 3,
            symbol: "MSFT".to_string(),
            name: "Microsoft".to_string(),
            asset_type: AssetType::Stock,
            payout: 50,
            is_otc: false,
            is_active: true,
            allowed_candles: vec![CandleLength::new(60)],
        };
        let asset4 = Asset {
            id: 4,
            symbol: "AMZN".to_string(),
            name: "Amazon".to_string(),
            asset_type: AssetType::Stock,
            payout: 50,
            is_otc: false,
            is_active: false,
            allowed_candles: vec![CandleLength::new(60)],
        };

        let mut assets_map = std::collections::HashMap::new();
        assets_map.insert("AAPL".to_string(), asset1.clone());
        assets_map.insert("GOOGL".to_string(), asset2.clone());
        assets_map.insert("MSFT".to_string(), asset3.clone());
        assets_map.insert("AMZN".to_string(), asset4.clone());
        let assets = Assets(assets_map);

        // Test active_count
        assert_eq!(assets.active_count(), 2);

        // Test active_iter
        let active_assets: Vec<&Asset> = assets.active_iter().collect();
        assert_eq!(active_assets.len(), 2);
        assert!(active_assets.iter().any(|a| a.symbol == "AAPL"));
        assert!(active_assets.iter().any(|a| a.symbol == "MSFT"));
        assert!(!active_assets.iter().any(|a| a.symbol == "GOOGL"));
        assert!(!active_assets.iter().any(|a| a.symbol == "AMZN"));

        // Test active() - returns new Assets collection
        let active_assets_collection = assets.active();
        assert_eq!(active_assets_collection.0.len(), 2);
        assert!(active_assets_collection.get("AAPL").is_some());
        assert!(active_assets_collection.get("MSFT").is_some());
        assert!(active_assets_collection.get("GOOGL").is_none());
        assert!(active_assets_collection.get("AMZN").is_none());

        // Verify that the original assets collection is unchanged
        assert_eq!(assets.0.len(), 4);
    }

    #[test]
    fn test_assets_active_empty() {
        let assets = Assets(std::collections::HashMap::new());
        assert_eq!(assets.active_count(), 0);
        let active_collection = assets.active();
        assert_eq!(active_collection.0.len(), 0);
    }

    #[test]
    fn test_assets_active_all_active() {
        let asset = Asset {
            id: 1,
            symbol: "TEST".to_string(),
            name: "Test".to_string(),
            asset_type: AssetType::Stock,
            payout: 50,
            is_otc: false,
            is_active: true,
            allowed_candles: vec![CandleLength::new(60)],
        };
        let mut map = std::collections::HashMap::new();
        map.insert("TEST".to_string(), asset);
        let assets = Assets(map);

        assert_eq!(assets.active_count(), 1);
        let active = assets.active();
        assert_eq!(active.0.len(), 1);
    }

    #[test]
    fn test_assets_active_all_inactive() {
        let asset = Asset {
            id: 1,
            symbol: "TEST".to_string(),
            name: "Test".to_string(),
            asset_type: AssetType::Stock,
            payout: 50,
            is_otc: false,
            is_active: false,
            allowed_candles: vec![CandleLength::new(60)],
        };
        let mut map = std::collections::HashMap::new();
        map.insert("TEST".to_string(), asset);
        let assets = Assets(map);

        assert_eq!(assets.active_count(), 0);
        let active = assets.active();
        assert_eq!(active.0.len(), 0);
    }
}
