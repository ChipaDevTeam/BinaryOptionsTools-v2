pub mod market;
pub mod virtual_market;

use std::sync::Arc;
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::select_all;
use crate::pocketoption::candle::{Candle, SubscriptionType};
use crate::pocketoption::pocket_client::PocketOption;
use crate::pocketoption::error::PocketResult;
use crate::pocketoption::types::Deal;
use crate::framework::market::Market;
use tracing::{info, error};

/// The Context provides strategies with access to the trading market and other utilities.
pub struct Context {
    pub market: Arc<dyn Market>,
    pub client: Arc<PocketOption>,
}

impl Context {
    pub fn new(client: Arc<PocketOption>) -> Self {
        Self {
            market: client.clone(),
            client,
        }
    }
}

/// The Strategy trait defines the interface for trading strategies.
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Called when the bot starts.
    async fn on_start(&self, _ctx: &Context) -> PocketResult<()> {
        Ok(())
    }

    /// Called when a new candle is received.
    async fn on_candle(&self, _ctx: &Context, _asset: &str, _candle: &Candle) -> PocketResult<()> {
        Ok(())
    }

    /// Called when a new tick (price update) is received.
    async fn on_tick(&self, _ctx: &Context, _asset: &str, _price: f64) -> PocketResult<()> {
        Ok(())
    }

    /// Called when a deal status changes or a new deal is opened/closed.
    async fn on_deal_update(&self, _ctx: &Context, _deal: &Deal) -> PocketResult<()> {
        Ok(())
    }

    /// Called when the balance changes.
    async fn on_balance_update(&self, _ctx: &Context, _balance: f64) -> PocketResult<()> {
        Ok(())
    }
}

/// The Bot manages the execution of a strategy.
pub struct Bot {
    ctx: Context,
    strategy: Box<dyn Strategy>,
    assets: Vec<(String, SubscriptionType)>,
}

impl Bot {
    pub fn new(client: PocketOption, strategy: Box<dyn Strategy>) -> Self {
        Self {
            ctx: Context::new(Arc::new(client)),
            strategy,
            assets: Vec::new(),
        }
    }

    /// Sets a custom market implementation (e.g., VirtualMarket for backtesting).
    pub fn with_market(mut self, market: Arc<dyn Market>) -> Self {
        self.ctx.market = market;
        self
    }

    /// Adds an asset to monitor with a specific subscription type.
    pub fn add_asset(&mut self, asset: impl Into<String>, sub_type: SubscriptionType) {
        self.assets.push((asset.into(), sub_type));
    }

    /// Starts the bot and its strategy loop.
    pub async fn run(&self) -> PocketResult<()> {
        info!("Starting bot...");
        self.strategy.on_start(&self.ctx).await?;

        let mut streams = Vec::new();

        for (asset, sub_type) in &self.assets {
            info!("Subscribing to {}...", asset);
            let stream = self.ctx.client.subscribe(asset.clone(), sub_type.clone()).await?;
            streams.push(stream.to_stream().map({
                let asset = asset.clone();
                move |res| (asset.clone(), res)
            }));
        }

        if streams.is_empty() {
            error!("No assets added to the bot. Exiting.");
            return Ok(());
        }

        let mut combined_stream = select_all(streams);

        info!("Bot is now running.");
        while let Some((asset, result)) = combined_stream.next().await {
            match result {
                Ok(candle) => {
                    if let Err(e) = self.strategy.on_candle(&self.ctx, &asset, &candle).await {
                        error!("Strategy on_candle error for {}: {:?}", asset, e);
                    }
                }
                Err(e) => {
                    error!("Stream error for {}: {:?}", asset, e);
                }
            }
        }

        Ok(())
    }
}

