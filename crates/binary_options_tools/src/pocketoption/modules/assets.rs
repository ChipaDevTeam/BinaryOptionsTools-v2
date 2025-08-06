use std::sync::Arc;

use crate::pocketoption::{
    state::State,
    types::{Assets, TwoStepRule},
};
use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule},
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
    ) -> Self {
        Self { state, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            if let Message::Binary(text) = &*msg {
                if let Ok(assets) = serde_json::from_slice::<Assets>(text) {
                    debug!("Loaded assets: {:?}", assets.names());
                    self.state.set_assets(assets).await;
                } else {
                    warn!("Failed to parse assets message: {:?}", text);
                }
            }
        }
        Err(CoreError::LightweightModuleLoop("AssetsModule".into()))
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(TwoStepRule::new(r#"451-["updateAssets","#))
    }
}

#[cfg(test)]
mod tests {
    use crate::pocketoption::types::Asset;

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
