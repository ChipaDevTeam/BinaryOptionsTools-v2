use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools::pocketoption::candle::SubscriptionType;
use binary_options_tools::pocketoption::modules::raw::{
    Outgoing, RawHandle as InnerRawHandle, RawHandler as InnerRawHandler,
};
use binary_options_tools::pocketoption::pocket_client::PocketOption as InnerClient;
use binary_options_tools::utils::f64_to_decimal;
use binary_options_tools::validator::Validator as CrateValidator;
use futures_util::future::{BoxFuture, Shared};
use futures_util::{FutureExt, StreamExt};
use napi_derive::napi;
use rust_decimal::prelude::ToPrimitive;
use serde_json::Value;
use uuid::Uuid;

use crate::config::ClientConfig;
use crate::error::{napi_err, BinaryErrorNode};
use crate::stream::{shared_stream, CandleStream, RawStream};
use crate::validator::Validator;

/// How long the initial handshake is allowed to take before the pending
/// connection is reported as a timeout. Matches the Python bindings.
const CONNECTION_TIMEOUT_SECS: u64 = 20;

/// A connection attempt shared by every method of a [`PocketOption`] instance.
///
/// The future is polled by a background task as soon as the client is created,
/// so `new PocketOption(ssid)` starts connecting immediately even though it
/// returns synchronously. Awaiting any method resolves the same attempt.
type ConnectFuture = Shared<BoxFuture<'static, Result<InnerClient, String>>>;

fn message_to_string(message: &tungstenite::Message) -> String {
    match message {
        tungstenite::Message::Text(text) => text.to_string(),
        tungstenite::Message::Binary(data) => String::from_utf8_lossy(data).into_owned(),
        _ => String::new(),
    }
}

fn arc_message_to_string(message: &Arc<tungstenite::Message>) -> String {
    message_to_string(message.as_ref())
}

/// Wraps a connection attempt so that it is driven eagerly and can be awaited
/// any number of times.
fn drive(future: BoxFuture<'static, Result<InnerClient, String>>) -> ConnectFuture {
    let shared = future.shared();
    let driver = shared.clone();
    napi::bindgen_prelude::spawn(async move {
        let _ = driver.await;
    });
    shared
}

async fn with_timeout<F>(future: F) -> Result<InnerClient, String>
where
    F: std::future::Future<
        Output = binary_options_tools::pocketoption::error::PocketResult<InnerClient>,
    >,
{
    match tokio::time::timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS), future).await {
        Ok(Ok(client)) => Ok(client),
        Ok(Err(e)) => Err(BinaryErrorNode::from(e).to_string()),
        Err(_) => Err(BinaryErrorNode::Timeout(format!(
            "the connection was not established within {CONNECTION_TIMEOUT_SECS} seconds"
        ))
        .to_string()),
    }
}

/// Sends `message` through a one-off raw handler and waits for the first
/// response accepted by `validator`.
async fn send_raw_message_and_wait(
    client: &InnerClient,
    validator: CrateValidator,
    message: String,
) -> napi::Result<String> {
    let handler = client
        .create_raw_handler(validator, None)
        .await
        .map_err(napi_err)?;
    let response = handler
        .send_and_wait(Outgoing::Text(message))
        .await
        .map_err(napi_err)?;
    Ok(arc_message_to_string(&response))
}

/// PocketOption trading client.
///
/// The constructor returns immediately and starts connecting in the
/// background; every method awaits that connection before doing its work, so
/// there is no need to sleep after construction. Use the static `create`
/// helpers when a connection failure should be reported up front.
#[napi]
pub struct PocketOption {
    connect: ConnectFuture,
}

impl PocketOption {
    async fn client(&self) -> napi::Result<InnerClient> {
        self.connect
            .clone()
            .await
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e))
    }
}

#[napi]
impl PocketOption {
    /// Connects to PocketOption with `ssid`, in the background.
    #[napi(constructor)]
    pub fn new(ssid: String) -> Self {
        Self {
            connect: drive(with_timeout(InnerClient::new(ssid)).boxed()),
        }
    }

    /// Connects using an explicit WebSocket endpoint, in the background.
    #[napi(factory)]
    pub fn with_url(ssid: String, url: String) -> Self {
        Self {
            connect: drive(with_timeout(InnerClient::new_with_url(ssid, url)).boxed()),
        }
    }

    /// Connects using a custom configuration, in the background.
    #[napi(factory)]
    pub fn with_config(ssid: String, config: ClientConfig) -> napi::Result<Self> {
        let config = binary_options_tools::config::Config::try_from(config)?;
        Ok(Self {
            connect: drive(with_timeout(InnerClient::new_with_config(ssid, config)).boxed()),
        })
    }

    /// Connects to PocketOption and resolves once the connection is ready.
    #[napi(factory)]
    pub async fn create(ssid: String) -> napi::Result<Self> {
        let client = Self::new(ssid);
        client.ready().await?;
        Ok(client)
    }

    /// Connects to an explicit endpoint and resolves once the connection is ready.
    #[napi(factory)]
    pub async fn create_with_url(ssid: String, url: String) -> napi::Result<Self> {
        let client = Self::with_url(ssid, url);
        client.ready().await?;
        Ok(client)
    }

    /// Connects with a custom configuration and resolves once ready.
    #[napi(factory)]
    pub async fn create_with_config(ssid: String, config: ClientConfig) -> napi::Result<Self> {
        let client = Self::with_config(ssid, config)?;
        client.ready().await?;
        Ok(client)
    }

    /// Resolves when the initial connection succeeded, rejects otherwise.
    #[napi]
    pub async fn ready(&self) -> napi::Result<()> {
        self.client().await?;
        Ok(())
    }

    /// Waits until the asset list has been received from the server.
    #[napi]
    pub async fn wait_for_assets(&self, timeout_secs: f64) -> napi::Result<()> {
        let client = self.client().await?;
        client
            .wait_for_assets(Duration::from_secs_f64(timeout_secs))
            .await
            .map_err(napi_err)
    }

    /// True when the session is a demo account.
    #[napi]
    pub async fn is_demo(&self) -> napi::Result<bool> {
        Ok(self.client().await?.is_demo())
    }

    /// True while the WebSocket connection is up.
    #[napi]
    pub async fn is_connected(&self) -> napi::Result<bool> {
        Ok(self.client().await?.is_connected())
    }

    /// Current account balance.
    #[napi]
    pub async fn balance(&self) -> napi::Result<f64> {
        let client = self.client().await?;
        Ok(client.balance().await.to_f64().unwrap_or_default())
    }

    /// Opens a call trade. Resolves to `[dealId, deal]`.
    #[napi(ts_return_type = "Promise<[string, Record<string, any>]>")]
    pub async fn buy(&self, asset: String, amount: f64, time: u32) -> napi::Result<Value> {
        let client = self.client().await?;
        let amount = f64_to_decimal(amount).ok_or_else(|| {
            napi_err(BinaryErrorNode::InvalidParameter(format!(
                "invalid amount: {amount}"
            )))
        })?;
        let (id, deal) = client.buy(asset, time, amount).await.map_err(napi_err)?;
        Ok(serde_json::json!([id.to_string(), deal]))
    }

    /// Opens a put trade. Resolves to `[dealId, deal]`.
    #[napi(ts_return_type = "Promise<[string, Record<string, any>]>")]
    pub async fn sell(&self, asset: String, amount: f64, time: u32) -> napi::Result<Value> {
        let client = self.client().await?;
        let amount = f64_to_decimal(amount).ok_or_else(|| {
            napi_err(BinaryErrorNode::InvalidParameter(format!(
                "invalid amount: {amount}"
            )))
        })?;
        let (id, deal) = client.sell(asset, time, amount).await.map_err(napi_err)?;
        Ok(serde_json::json!([id.to_string(), deal]))
    }

    /// Waits for `tradeId` to settle and resolves with the closed deal.
    #[napi]
    pub async fn result(&self, trade_id: String) -> napi::Result<Value> {
        let client = self.client().await?;
        let uuid = Uuid::parse_str(&trade_id).map_err(napi_err)?;
        let deal = client.result(uuid).await.map_err(napi_err)?;
        serde_json::to_value(&deal).map_err(napi_err)
    }

    /// Alias of `result`, kept for parity with the other language bindings.
    #[napi]
    pub async fn check_win(&self, trade_id: String) -> napi::Result<Value> {
        self.result(trade_id).await
    }

    /// Unix timestamp at which `tradeId` expires, or `null` if unknown.
    #[napi]
    pub async fn get_deal_end_time(&self, trade_id: String) -> napi::Result<Option<i64>> {
        let client = self.client().await?;
        let uuid = Uuid::parse_str(&trade_id).map_err(napi_err)?;
        let deal = match client.get_closed_deal(uuid).await {
            Some(deal) => Some(deal),
            None => client.get_opened_deal(uuid).await,
        };
        Ok(deal.map(|deal| deal.close_timestamp.timestamp()))
    }

    /// Candles of `period` seconds compiled from tick history.
    #[napi]
    pub async fn candles(&self, asset: String, period: u32) -> napi::Result<Value> {
        let client = self.client().await?;
        let candles = client.candles(asset, period).await.map_err(napi_err)?;
        serde_json::to_value(&candles).map_err(napi_err)
    }

    /// Alias of `candles`.
    #[napi]
    pub async fn history(&self, asset: String, period: u32) -> napi::Result<Value> {
        self.candles(asset, period).await
    }

    /// Candles fetched directly from the server.
    #[napi]
    pub async fn get_candles(
        &self,
        asset: String,
        period: i64,
        offset: i64,
    ) -> napi::Result<Value> {
        let client = self.client().await?;
        let candles = client
            .get_candles(asset, period, offset)
            .await
            .map_err(napi_err)?;
        serde_json::to_value(&candles).map_err(napi_err)
    }

    /// Candles fetched from the server, anchored at `time`.
    #[napi]
    pub async fn get_candles_advanced(
        &self,
        asset: String,
        period: i64,
        offset: i64,
        time: i64,
    ) -> napi::Result<Value> {
        let client = self.client().await?;
        let candles = client
            .get_candles_advanced(asset, period, time, offset)
            .await
            .map_err(napi_err)?;
        serde_json::to_value(&candles).map_err(napi_err)
    }

    /// Raw tick history as `[timestamp, price]` pairs.
    #[napi(ts_return_type = "Promise<Array<[number, number]>>")]
    pub async fn get_ticks(&self, asset: String, lookback_seconds: u32) -> napi::Result<Value> {
        let client = self.client().await?;
        let ticks = client
            .ticks(asset, lookback_seconds)
            .await
            .map_err(napi_err)?;
        serde_json::to_value(&ticks).map_err(napi_err)
    }

    /// Compiles tick history into candles of an arbitrary duration.
    #[napi]
    pub async fn compile_candles(
        &self,
        asset: String,
        custom_period: u32,
        lookback_period: u32,
    ) -> napi::Result<Value> {
        if custom_period == 0 {
            return Err(napi_err(BinaryErrorNode::InvalidParameter(
                "customPeriod must be non-zero".to_string(),
            )));
        }
        if lookback_period == 0 {
            return Err(napi_err(BinaryErrorNode::InvalidParameter(
                "lookbackPeriod must be non-zero".to_string(),
            )));
        }
        let client = self.client().await?;
        let candles = client
            .compile_candles(asset, custom_period, lookback_period)
            .await
            .map_err(napi_err)?;
        serde_json::to_value(&candles).map_err(napi_err)
    }

    /// Payout percentage of every currently active asset.
    #[napi(ts_return_type = "Promise<Record<string, number>>")]
    pub async fn payout(&self) -> napi::Result<Value> {
        let client = self.client().await?;
        match client.assets().await {
            Some(assets) => {
                let payouts: HashMap<&String, i32> = assets
                    .0
                    .iter()
                    .filter_map(|(asset, symbol)| {
                        symbol.is_active.then_some((asset, symbol.payout))
                    })
                    .collect();
                serde_json::to_value(&payouts).map_err(napi_err)
            }
            None => Err(napi_err(BinaryErrorNode::Uninitialized(
                "assets not initialized yet".into(),
            ))),
        }
    }

    /// Every asset currently open for trading.
    #[napi]
    pub async fn active_assets(&self) -> napi::Result<Value> {
        let client = self.client().await?;
        match client.active_assets().await {
            Some(assets) => serde_json::to_value(&assets).map_err(napi_err),
            None => Err(napi_err(BinaryErrorNode::Uninitialized(
                "assets not initialized yet".into(),
            ))),
        }
    }

    /// Deals that have already settled, keyed by deal id.
    #[napi]
    pub async fn closed_deals(&self) -> napi::Result<Value> {
        let client = self.client().await?;
        serde_json::to_value(client.get_closed_deals().await).map_err(napi_err)
    }

    /// A single settled deal, or `null` when it is not known.
    #[napi]
    pub async fn get_closed_deal(&self, trade_id: String) -> napi::Result<Option<Value>> {
        let client = self.client().await?;
        let uuid = Uuid::parse_str(&trade_id).map_err(napi_err)?;
        match client.get_closed_deal(uuid).await {
            Some(deal) => Ok(Some(serde_json::to_value(&deal).map_err(napi_err)?)),
            None => Ok(None),
        }
    }

    /// Drops the cache of settled deals.
    #[napi]
    pub async fn clear_closed_deals(&self) -> napi::Result<()> {
        self.client().await?.clear_closed_deals().await;
        Ok(())
    }

    /// Deals that are still running, keyed by deal id.
    #[napi]
    pub async fn opened_deals(&self) -> napi::Result<Value> {
        let client = self.client().await?;
        serde_json::to_value(client.get_opened_deals().await).map_err(napi_err)
    }

    /// A single running deal, or `null` when it is not known.
    #[napi]
    pub async fn get_opened_deal(&self, trade_id: String) -> napi::Result<Option<Value>> {
        let client = self.client().await?;
        let uuid = Uuid::parse_str(&trade_id).map_err(napi_err)?;
        match client.get_opened_deal(uuid).await {
            Some(deal) => Ok(Some(serde_json::to_value(&deal).map_err(napi_err)?)),
            None => Ok(None),
        }
    }

    /// Places an order that opens at a future time or price.
    #[napi]
    #[allow(clippy::too_many_arguments)]
    pub async fn open_pending_order(
        &self,
        open_type: u32,
        amount: f64,
        asset: String,
        open_time: String,
        open_price: f64,
        timeframe: u32,
        min_payout: u32,
        command: u32,
    ) -> napi::Result<Value> {
        let client = self.client().await?;
        let amount = f64_to_decimal(amount).ok_or_else(|| {
            napi_err(BinaryErrorNode::InvalidParameter(format!(
                "invalid amount: {amount}"
            )))
        })?;
        let open_price = f64_to_decimal(open_price).ok_or_else(|| {
            napi_err(BinaryErrorNode::InvalidParameter(format!(
                "invalid open price: {open_price}"
            )))
        })?;
        let order = client
            .open_pending_order(
                open_type, amount, asset, open_time, open_price, timeframe, min_payout, command,
            )
            .await
            .map_err(napi_err)?;
        serde_json::to_value(&order).map_err(napi_err)
    }

    /// Cancels a single pending order.
    #[napi]
    pub async fn cancel_pending_order(&self, ticket: String) -> napi::Result<Value> {
        let client = self.client().await?;
        let ticket = client
            .cancel_pending_order(ticket)
            .await
            .map_err(napi_err)?;
        Ok(serde_json::json!({ "ticket": ticket, "status": "cancelled" }))
    }

    /// Cancels several pending orders at once.
    #[napi]
    pub async fn cancel_pending_orders(&self, tickets: Vec<String>) -> napi::Result<Value> {
        let client = self.client().await?;
        let cancelled = client
            .cancel_pending_orders(tickets)
            .await
            .map_err(napi_err)?;
        Ok(serde_json::json!({ "cancelled": cancelled }))
    }

    /// Sends a message over the WebSocket without waiting for a response.
    #[napi]
    pub async fn send_raw(&self, message: String) -> napi::Result<()> {
        let client = self.client().await?;
        client.send_raw(message).await.map_err(napi_err)
    }

    /// Sends a message through a throw-away raw handler.
    #[napi]
    pub async fn send_raw_message(&self, message: String) -> napi::Result<()> {
        let client = self.client().await?;
        let handler = client
            .create_raw_handler(CrateValidator::None, None)
            .await
            .map_err(napi_err)?;
        handler.send_text(message).await.map_err(napi_err)
    }

    /// Async iterator over every incoming WebSocket message.
    #[napi]
    pub async fn subscribe_raw(&self) -> napi::Result<RawStream> {
        let client = self.client().await?;
        let raw = client.subscribe_raw().await.map_err(napi_err)?;
        let stream = async_stream::stream! {
            tokio::pin!(raw);
            while let Some(message) = raw.next().await {
                yield Ok(arc_message_to_string(&message));
            }
        };
        Ok(RawStream {
            stream: shared_stream(stream.boxed()),
        })
    }

    /// Async iterator over the candles of `symbol`.
    ///
    /// Without `seconds` every tick is yielded as it arrives; with `seconds`
    /// the ticks are aggregated into candles of that duration.
    #[napi]
    pub async fn subscribe(
        &self,
        symbol: String,
        seconds: Option<u32>,
    ) -> napi::Result<CandleStream> {
        match seconds {
            Some(seconds) => self.subscribe_symbol_timed(symbol, seconds).await,
            None => self.subscribe_symbol(symbol).await,
        }
    }

    /// Async iterator yielding every candle update of `symbol`.
    #[napi]
    pub async fn subscribe_symbol(&self, symbol: String) -> napi::Result<CandleStream> {
        let client = self.client().await?;
        let subscription = client
            .subscribe(symbol, SubscriptionType::none())
            .await
            .map_err(napi_err)?;
        Ok(CandleStream {
            stream: shared_stream(subscription.to_stream().boxed()),
        })
    }

    /// Async iterator yielding a candle every `chunkSize` updates.
    #[napi]
    pub async fn subscribe_symbol_chunked(
        &self,
        symbol: String,
        chunk_size: u32,
    ) -> napi::Result<CandleStream> {
        let client = self.client().await?;
        let subscription = client
            .subscribe(symbol, SubscriptionType::chunk(chunk_size as usize))
            .await
            .map_err(napi_err)?;
        Ok(CandleStream {
            stream: shared_stream(subscription.to_stream().boxed()),
        })
    }

    /// Async iterator yielding a candle every `seconds`.
    #[napi]
    pub async fn subscribe_symbol_timed(
        &self,
        symbol: String,
        seconds: u32,
    ) -> napi::Result<CandleStream> {
        let client = self.client().await?;
        let subscription = client
            .subscribe(
                symbol,
                SubscriptionType::time(Duration::from_secs(seconds as u64)),
            )
            .await
            .map_err(napi_err)?;
        Ok(CandleStream {
            stream: shared_stream(subscription.to_stream().boxed()),
        })
    }

    /// Like `subscribeSymbolTimed`, but candles are aligned to UTC boundaries.
    #[napi]
    pub async fn subscribe_symbol_time_aligned(
        &self,
        symbol: String,
        seconds: u32,
    ) -> napi::Result<CandleStream> {
        let client = self.client().await?;
        let subscription_type = SubscriptionType::time_aligned(Duration::from_secs(seconds as u64))
            .map_err(napi_err)?;
        let subscription = client
            .subscribe(symbol, subscription_type)
            .await
            .map_err(napi_err)?;
        Ok(CandleStream {
            stream: shared_stream(subscription.to_stream().boxed()),
        })
    }

    /// Stops the real-time subscription of `asset`.
    #[napi]
    pub async fn unsubscribe(&self, asset: String) -> napi::Result<()> {
        let client = self.client().await?;
        client.unsubscribe(asset).await.map_err(napi_err)
    }

    /// Handle used to register ad-hoc raw message handlers.
    #[napi]
    pub async fn raw_handle(&self) -> napi::Result<RawHandle> {
        let client = self.client().await?;
        let handle = client.raw_handle().await.map_err(napi_err)?;
        Ok(RawHandle { handle })
    }

    /// Registers a handler that keeps every message accepted by `validator`.
    #[napi]
    pub async fn create_raw_handler(
        &self,
        validator: &Validator,
        keep_alive: Option<String>,
    ) -> napi::Result<RawHandler> {
        let client = self.client().await?;
        let validator: CrateValidator = validator.inner.clone().into();
        let handler = client
            .create_raw_handler(validator, keep_alive.map(Outgoing::Text))
            .await
            .map_err(napi_err)?;
        Ok(RawHandler { handler })
    }

    /// Sends `message` and resolves with the first response `validator` accepts.
    #[napi]
    pub async fn create_raw_order(
        &self,
        message: String,
        validator: &Validator,
    ) -> napi::Result<String> {
        let client = self.client().await?;
        send_raw_message_and_wait(&client, validator.inner.clone().into(), message).await
    }

    /// `createRawOrder` with a timeout in milliseconds.
    #[napi]
    pub async fn create_raw_order_with_timeout(
        &self,
        message: String,
        validator: &Validator,
        timeout_ms: f64,
    ) -> napi::Result<String> {
        let client = self.client().await?;
        let validator: CrateValidator = validator.inner.clone().into();
        let timeout = Duration::from_secs_f64(timeout_ms / 1000.0);
        tokio::time::timeout(
            timeout,
            send_raw_message_and_wait(&client, validator, message),
        )
        .await
        .map_err(|_| napi_err(BinaryErrorNode::Timeout("the operation timed out".into())))?
    }

    /// `createRawOrderWithTimeout` retried up to three times with backoff.
    #[napi]
    pub async fn create_raw_order_with_timeout_and_retry(
        &self,
        message: String,
        validator: &Validator,
        timeout_ms: f64,
    ) -> napi::Result<String> {
        let client = self.client().await?;
        let validator: CrateValidator = validator.inner.clone().into();
        let timeout = Duration::from_secs_f64(timeout_ms / 1000.0);
        let max_retries = 3;
        let mut delay = Duration::from_millis(100);

        for retry in 0..max_retries {
            let attempt = send_raw_message_and_wait(&client, validator.clone(), message.clone());
            match tokio::time::timeout(timeout, attempt).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(e)) => {
                    if retry + 1 == max_retries {
                        return Err(e);
                    }
                }
                Err(_) => {
                    if retry + 1 == max_retries {
                        return Err(napi_err(BinaryErrorNode::Timeout(
                            "the operation timed out after retries".into(),
                        )));
                    }
                }
            }
            tokio::time::sleep(delay).await;
            delay = delay.saturating_mul(2);
        }

        Err(napi_err(BinaryErrorNode::NotAllowed(
            "the operation failed after all retries".into(),
        )))
    }

    /// Sends `message` and returns an async iterator of matching responses.
    ///
    /// When `timeoutMs` is given the iterator ends after that much time.
    #[napi]
    pub async fn create_raw_iterator(
        &self,
        message: String,
        validator: &Validator,
        timeout_ms: Option<f64>,
    ) -> napi::Result<RawStream> {
        let client = self.client().await?;
        let validator: CrateValidator = validator.inner.clone().into();
        let timeout = timeout_ms.map(|ms| Duration::from_secs_f64(ms / 1000.0));
        let handler = client
            .create_raw_handler(validator, None)
            .await
            .map_err(napi_err)?;
        handler.send_text(message).await.map_err(napi_err)?;
        let receiver = handler.subscribe();

        let stream = async_stream::stream! {
            let start = std::time::Instant::now();
            loop {
                let message = match timeout {
                    Some(timeout) => {
                        let Some(remaining) = timeout.checked_sub(start.elapsed()) else {
                            break;
                        };
                        match tokio::time::timeout(remaining, receiver.recv()).await {
                            Ok(Ok(message)) => message,
                            _ => break,
                        }
                    }
                    None => match receiver.recv().await {
                        Ok(message) => message,
                        Err(_) => break,
                    },
                };
                yield Ok(arc_message_to_string(&message));
            }
        };

        Ok(RawStream {
            stream: shared_stream(stream.boxed()),
        })
    }

    /// Server time as a Unix timestamp in seconds.
    #[napi]
    pub async fn server_time(&self) -> napi::Result<i64> {
        let client = self.client().await?;
        Ok(client.server_time().await.timestamp())
    }

    /// Alias of `serverTime`.
    #[napi]
    pub async fn get_server_time(&self) -> napi::Result<i64> {
        self.server_time().await
    }

    /// Stops the background runner and closes the connection for good.
    #[napi]
    pub async fn shutdown(&self) -> napi::Result<()> {
        let client = self.client().await?;
        client.shutdown().await.map_err(napi_err)
    }

    /// Closes the connection but keeps the configuration.
    #[napi]
    pub async fn disconnect(&self) -> napi::Result<()> {
        let client = self.client().await?;
        client.disconnect().await.map_err(napi_err)
    }

    /// Re-opens the connection after `disconnect`.
    #[napi]
    pub async fn connect(&self) -> napi::Result<()> {
        let client = self.client().await?;
        client.connect().await.map_err(napi_err)
    }

    /// Disconnects and connects again.
    #[napi]
    pub async fn reconnect(&self) -> napi::Result<()> {
        let client = self.client().await?;
        client.reconnect().await.map_err(napi_err)
    }
}

/// Entry point of the raw message module.
#[napi]
pub struct RawHandle {
    handle: InnerRawHandle,
}

#[napi]
impl RawHandle {
    /// Registers a new handler bound to `validator`.
    #[napi]
    pub async fn create(
        &self,
        validator: &Validator,
        keep_alive: Option<String>,
    ) -> napi::Result<RawHandler> {
        let validator: CrateValidator = validator.inner.clone().into();
        let handler = self
            .handle
            .create(validator, keep_alive.map(Outgoing::Text))
            .await
            .map_err(napi_err)?;
        Ok(RawHandler { handler })
    }

    /// Removes a previously registered handler. Resolves to `true` when the
    /// handler existed.
    #[napi]
    pub async fn remove(&self, id: String) -> napi::Result<bool> {
        let uuid = Uuid::parse_str(&id).map_err(napi_err)?;
        self.handle.remove(uuid).await.map_err(napi_err)
    }
}

/// A registered raw message handler.
#[napi]
pub struct RawHandler {
    handler: InnerRawHandler,
}

#[napi]
impl RawHandler {
    /// Identifier of this handler, used by `RawHandle.remove`.
    #[napi]
    pub fn id(&self) -> String {
        self.handler.id().to_string()
    }

    /// Sends a text message.
    #[napi]
    pub async fn send_text(&self, text: String) -> napi::Result<()> {
        self.handler.send_text(text).await.map_err(napi_err)
    }

    /// Alias of `sendText`.
    #[napi]
    pub async fn send(&self, text: String) -> napi::Result<()> {
        self.send_text(text).await
    }

    /// Sends a binary message.
    #[napi]
    pub async fn send_binary(&self, data: Vec<u8>) -> napi::Result<()> {
        self.handler.send_binary(data).await.map_err(napi_err)
    }

    /// Sends `message` and resolves with the first accepted response.
    #[napi]
    pub async fn send_and_wait(&self, message: String) -> napi::Result<String> {
        let response = self
            .handler
            .send_and_wait(Outgoing::Text(message))
            .await
            .map_err(napi_err)?;
        Ok(arc_message_to_string(&response))
    }

    /// `sendAndWait` with a timeout in milliseconds.
    #[napi]
    pub async fn send_and_wait_with_timeout(
        &self,
        message: String,
        timeout_ms: f64,
    ) -> napi::Result<String> {
        let timeout = Duration::from_secs_f64(timeout_ms / 1000.0);
        tokio::time::timeout(timeout, self.send_and_wait(message))
            .await
            .map_err(|_| napi_err(BinaryErrorNode::Timeout("the operation timed out".into())))?
    }

    /// Resolves with the next accepted message.
    #[napi]
    pub async fn wait_next(&self) -> napi::Result<String> {
        let response = self.handler.wait_next().await.map_err(napi_err)?;
        Ok(arc_message_to_string(&response))
    }

    /// Async iterator over every accepted message.
    #[napi]
    pub fn subscribe(&self) -> RawStream {
        let receiver = self.handler.subscribe();
        let stream = async_stream::stream! {
            while let Ok(message) = receiver.recv().await {
                yield Ok(arc_message_to_string(&message));
            }
        };
        RawStream {
            stream: shared_stream(stream.boxed()),
        }
    }
}
