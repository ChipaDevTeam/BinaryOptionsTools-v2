use binary_options_tools::error::BinaryOptionsResult;
use binary_options_tools::pocketoption::error::PocketResult;
use binary_options_tools::pocketoption::pocket_client::PocketOption as Pocket;
use binary_options_tools::pocketoption::types::base::RawWebsocketMessage;
use binary_options_tools::pocketoption::types::update::DataCandle;
use binary_options_tools::pocketoption::ws::stream::StreamAsset;
use binary_options_tools::reimports::FilteredRecieverStream;
use futures_util::stream::{BoxStream, Fuse};
use futures_util::StreamExt;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

use crate::error::BinaryErrorJs;
use crate::runtime::get_runtime;
use crate::validator::Validator;

/// Iterator for receiving processed WebSocket messages.
/// Provides asynchronous iteration over parsed messages from the server.
///
/// # Examples
/// ```javascript
/// const stream = await client.subscribeSymbol('EUR/USD');
/// for await (const price of stream) {
///     console.log('Current price:', price);
/// }
/// ```
#[napi]
pub struct StreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, PocketResult<DataCandle>>>>>,
}

/// Iterator for receiving raw WebSocket messages.
/// Provides asynchronous iteration over raw messages from the server.
///
/// # Examples
/// ```javascript
/// const stream = await client.createRawIterator();
/// for await (const message of stream) {
///     console.log('Raw message:', message);
/// }
/// ```
#[napi]
pub struct RawStreamIterator {
    stream: Arc<Mutex<Fuse<BoxStream<'static, BinaryOptionsResult<RawWebsocketMessage>>>>>,
}

/// A client for interacting with the Pocket Option trading platform.
/// Provides methods for executing trades, managing positions, and streaming market data.
///
/// # Examples
/// ```javascript
/// const client = new PocketOption("your-ssid-here");
///
/// // Execute a buy order
/// const [orderId, details] = await client.buy("EUR/USD", 100, 60);
///
/// // Check trade result
/// const result = await client.checkWin(orderId);
/// ```
#[napi]
pub struct PocketOption {
    client: Pocket,
}

#[napi]
impl PocketOption {
    /// Creates a new PocketOption client instance using a session ID.
    ///
    /// # Arguments
    /// * `ssid` - A valid session ID string from Pocket Option
    ///
    /// # Examples
    /// ```javascript
    /// const client = new PocketOption("your-ssid-here");
    /// ```
    #[napi(constructor)]
    pub fn new(ssid: String) -> Result<Self> {
        let runtime = get_runtime()?;
        runtime.block_on(async move {
            let client = Pocket::new(ssid).await.map_err(BinaryErrorJs::from)?;
            Ok(Self { client })
        })
    }

    /// Creates a new PocketOption client instance with a custom WebSocket URL.
    ///
    /// # Arguments
    /// * `ssid` - A valid session ID string from Pocket Option
    /// * `url` - Custom WebSocket server URL
    ///
    /// # Examples
    /// ```javascript
    /// const client = await PocketOption.newWithUrl(
    ///     "your-ssid-here",
    ///     "wss://custom-server.com/ws"
    /// );
    /// ```
    #[napi(factory)]
    pub async fn new_with_url(ssid: String, url: String) -> Result<Self> {
        let client = Pocket::new_with_url(
            ssid,
            Url::parse(&url).map_err(|e| Error::from_reason(e.to_string()))?,
        )
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self { client })
    }

    /// Checks if the current account is a demo account.
    ///
    /// # Returns
    /// * `true` if the account is a demo account
    /// * `false` if the account is a real account
    ///
    /// # Examples
    /// ```javascript
    /// // Check account type
    /// const isDemo = await client.isDemo();
    /// if (isDemo) {
    ///     console.log("Using demo account");
    /// } else {
    ///     console.log("Using real account");
    /// }
    ///
    /// // Example with balance check
    /// const isDemo = await client.isDemo();
    /// const balance = await client.balance();
    /// console.log(`${isDemo ? 'Demo' : 'Real'} account balance: ${balance}`);
    ///
    /// // Example with trade validation
    /// const isDemo = await client.isDemo();
    /// if (!isDemo && amount > 100) {
    ///     throw new Error("Large trades should be tested in demo first");
    /// }
    /// ```
    #[napi]
    pub async fn is_demo(&self) -> bool {
        self.client.is_demo().await
    }

    /// Executes a buy (CALL) order for a specified asset.
    ///
    /// # Arguments
    /// * `asset` - The trading asset/symbol (e.g., "EUR/USD")
    /// * `amount` - The trade amount in account currency
    /// * `time` - The option duration in seconds
    ///
    /// # Returns
    /// A vector containing the order ID and order details as JSON
    ///
    /// # Examples
    /// ```javascript
    /// const [orderId, details] = await client.buy("EUR/USD", 100, 60);
    /// console.log(`Order placed: ${orderId}`);
    /// console.log(`Details: ${details}`);
    /// ```
    #[napi]
    pub async fn buy(&self, asset: String, amount: f64, time: u32) -> Result<Vec<Value>> {
        let res = self
            .client
            .buy(asset, amount, time)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let deal = serde_json::to_value(&res.1).map_err(|e| Error::from_reason(e.to_string()))?;
        let id = serde_json::to_value(res.0)?;
        Ok(vec![id, deal])
    }

    /// Executes a sell (PUT) order for a specified asset.
    ///
    /// # Arguments
    /// * `asset` - The trading asset/symbol (e.g., "EUR/USD")
    /// * `amount` - The trade amount in account currency
    /// * `time` - The option duration in seconds
    ///
    /// # Returns
    /// A vector containing the order ID and order details as JSON
    ///
    /// # Examples
    /// ```javascript
    /// const [orderId, details] = await client.sell("EUR/USD", 100, 60);
    /// console.log(`Order placed: ${orderId}`);
    /// console.log(`Details: ${details}`);
    /// ```
    #[napi]
    pub async fn sell(&self, asset: String, amount: f64, time: u32) -> Result<Vec<Value>> {
        let res = self
            .client
            .sell(asset, amount, time)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let deal = serde_json::to_value(&res.1).map_err(|e| Error::from_reason(e.to_string()))?;
        let id = serde_json::to_value(res.0)?;
        Ok(vec![id, deal])
    }

    /// Checks the result of a trade by its ID.
    ///
    /// # Arguments
    /// * `trade_id` - The UUID of the trade to check
    ///
    /// # Returns
    /// A JSON string containing the trade result details
    ///
    /// # Examples
    /// ```javascript
    /// const result = await client.checkWin(tradeId);
    /// const details = JSON.parse(result);
    /// console.log(`Profit: ${details.profit}`);
    /// ```
    #[napi]
    pub async fn check_win(&self, trade_id: String) -> Result<Value> {
        let res = self
            .client
            .check_results(
                Uuid::parse_str(&trade_id).map_err(|e| Error::from_reason(e.to_string()))?,
            )
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_value(&res).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Gets the expiration timestamp of a trade.
    ///
    /// # Arguments
    /// * `trade_id` - The UUID of the trade
    ///
    /// # Returns
    /// The Unix timestamp when the trade will expire, or null if not found
    ///
    /// # Examples
    /// ```javascript
    /// const endTime = await client.getDealEndTime(tradeId);
    /// if (endTime) {
    ///     console.log(`Trade expires at: ${new Date(endTime * 1000)}`);
    /// }
    /// ```
    #[napi]
    pub async fn get_deal_end_time(&self, trade_id: String) -> Result<Option<i64>> {
        Ok(self
            .client
            .get_deal_end_time(
                Uuid::parse_str(&trade_id).map_err(|e| Error::from_reason(e.to_string()))?,
            )
            .await
            .map(|t| t.timestamp()))
    }

    /// Retrieves historical candle data for an asset.
    ///
    /// # Arguments
    /// * `asset` - The trading asset/symbol (e.g., "EUR/USD")
    /// * `period` - The candle period in seconds
    /// * `offset` - Time offset for historical data
    ///
    /// # Returns
    /// A JSON string containing the candle data
    ///
    /// # Examples
    /// ```javascript
    /// const candles = await client.getCandles("EUR/USD", 60, 6000);
    /// const data = JSON.parse(candles);
    /// console.log(`Retrieved ${data.length} candles`);
    /// ```
    #[napi]
    pub async fn get_candles(&self, asset: String, period: i64, offset: i64) -> Result<Value> {
        let res = self
            .client
            .get_candles(asset, period, offset)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_value(&res).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Retrieves historical candle data for an asset with time
    ///
    /// # Arguments
    /// * `asset` - The trading asset/symbol (e.g., "EUR/USD")
    /// * `time` - The UNIX timestamp for the start of the candle data
    /// * `period` - The candle period in seconds
    /// * `offset` - Time offset for historical data
    ///
    /// # Returns
    /// A JSON string containing the candle data
    ///
    /// # Examples
    /// ```javascript
    /// const candles = await client.getCandlesAdvanced("EUR/USD", 1751925016, 60, 6000);
    /// const data = JSON.parse(candles);
    /// console.log(`Retrieved ${data.length} candles`);
    /// ```
    #[napi]
    pub async fn get_candles_advanced(&self, asset: String, time: i64, period: i64, offset: i64) -> Result<Value> {
        let res = self
            .client
            .get_candles_advanced(asset, time, period, offset)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_value(&res).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Retrieves the current account balance.
    ///
    /// # Returns
    /// A f64 representing the account balance
    ///
    /// # Examples
    /// ```javascript
    /// const balance = await client.balance();
    /// console.log(`Current balance: ${balance}`);
    /// ```
    #[napi]
    pub async fn balance(&self) -> f64 {
        let res = self.client.get_balance().await;
        res.balance
    }

    /// Retrieves all closed deals/trades.
    ///
    /// # Returns
    /// A JSON string containing the closed deals information
    ///
    /// # Examples
    /// ```javascript
    /// const deals = await client.closedDeals();
    /// const data = JSON.parse(deals);
    /// console.log(`Total closed deals: ${data.length}`);
    /// ```
    #[napi]
    pub async fn closed_deals(&self) -> Result<Value> {
        let res = self.client.get_closed_deals().await;
        serde_json::to_value(&res).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Clears the list of closed deals from memory.
    ///
    /// # Examples
    /// ```javascript
    /// await client.clearClosedDeals();
    /// ```
    #[napi]
    pub async fn clear_closed_deals(&self) {
        self.client.clear_closed_deals().await
    }

    /// Retrieves all currently open deals/trades.
    ///
    /// # Returns
    /// A JSON string containing the open deals information
    ///
    /// # Examples
    /// ```javascript
    /// const deals = await client.openedDeals();
    /// const data = JSON.parse(deals);
    /// console.log(`Total open positions: ${data.length}`);
    /// ```
    #[napi]
    pub async fn opened_deals(&self) -> Result<Value> {
        let res = self.client.get_opened_deals().await;
        serde_json::to_value(&res).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Retrieves the current payout rates for assets.
    ///
    /// # Arguments
    /// * `asset` - Optional parameter that can be:
    ///   * `undefined`: Returns all asset payouts (default)
    ///   * `string`: Returns payout for a specific asset
    ///   * `string[]`: Returns payouts for multiple assets
    ///
    /// # Returns
    /// A JSON value containing the payout information:
    /// * When no asset specified: Object mapping assets to payout percentages
    /// * When single asset specified: Number representing payout percentage
    /// * When multiple assets specified: Array of payout percentages in same order
    ///
    /// # Examples
    /// ```javascript
    /// // Get all payouts
    /// const allPayouts = await client.payout();
    /// console.log("All payouts:", allPayouts);
    /// // Output: { "EUR/USD": 85, "GBP/USD": 82, ... }
    ///
    /// // Get single asset payout
    /// const eurUsdPayout = await client.payout("EUR/USD");
    /// if (eurUsdPayout !== null) {
    ///     console.log(`EUR/USD payout: ${eurUsdPayout}%`);
    /// } else {
    ///     console.log("Asset not found");
    /// }
    ///
    /// // Get multiple asset payouts
    /// const assets = ["EUR/USD", "GBP/USD", "USD/JPY"];
    /// const payouts = await client.payout(assets);
    /// assets.forEach((asset, index) => {
    ///     const rate = payouts[index];
    ///     if (rate > 0) {
    ///         console.log(`${asset} payout: ${rate}%`);
    ///     } else {
    ///         console.log(`${asset} not available`);
    ///     }
    /// });
    ///
    /// // Find best payout
    /// const rates = await client.payout();
    /// const bestAsset = Object.entries(rates)
    ///     .reduce((a, b) => a[1] > b[1] ? a : b);
    /// console.log(`Best payout: ${bestAsset[0]} at ${bestAsset[1]}%`);
    /// ```
    #[napi]
    pub async fn payout(&self, asset: Option<Either<String, Vec<String>>>) -> Result<Value> {
        let res = self.client.get_payout().await;
        match asset {
            Some(Either::A(single)) => Ok(serde_json::to_value(
                res.get(&single)
                    .ok_or(Error::from_reason("Asset not found"))?,
            )?),
            Some(Either::B(multiple)) => Ok(serde_json::to_value(
                multiple
                    .iter()
                    .map(|asset| res.get(asset).unwrap_or(&0))
                    .collect::<Vec<_>>(),
            )?),
            None => Ok(serde_json::to_value(&res)?),
        }
    }

    /// Retrieves historical data for an asset.
    ///
    /// # Arguments
    /// * `asset` - The trading asset/symbol (e.g., "EUR/USD")
    /// * `period` - The historical data period
    ///
    /// # Returns
    /// A JSON string containing the historical data
    ///
    /// # Examples
    /// ```javascript
    /// const history = await client.history("EUR/USD", 60);
    /// const data = JSON.parse(history);
    /// console.log(`Retrieved ${data.length} historical records`);
    /// ```
    #[napi]
    pub async fn history(&self, asset: String, period: i64) -> Result<Value> {
        let res = self
            .client
            .history(asset, period)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        serde_json::to_value(&res).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Subscribes to real-time price updates for a symbol.
    ///
    /// # Arguments
    /// * `symbol` - The trading symbol to subscribe to (e.g., "EUR/USD")
    ///
    /// # Returns
    /// A StreamIterator for receiving price updates
    ///
    /// # Examples
    /// ```javascript
    /// const stream = await client.subscribeSymbol("EUR/USD");
    /// for await (const update of stream) {
    ///     console.log(`New price: ${update.price}`);
    /// }
    /// ```
    #[napi]
    pub async fn subscribe_symbol(&self, symbol: String) -> Result<StreamIterator> {
        let stream_asset = self
            .client
            .subscribe_symbol(symbol)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let boxed_stream = StreamAsset::to_stream_static(Arc::new(stream_asset))
            .boxed()
            .fuse();
        let stream = Arc::new(Mutex::new(boxed_stream));
        Ok(StreamIterator { stream })
    }

    /// Subscribes to symbol updates with chunked delivery.
    ///
    /// # Arguments
    /// * `symbol` - The trading symbol to subscribe to (e.g., "EUR/USD")
    /// * `chunk_size` - Number of updates to collect before delivery
    ///
    /// # Returns
    /// A StreamIterator for receiving chunked price updates
    ///
    /// # Examples
    /// ```javascript
    /// const stream = await client.subscribeSymbolChunked("EUR/USD", 10);
    /// for await (const updates of stream) {
    ///     console.log(`Received batch of ${updates.length} updates`);
    /// }
    /// ```
    #[napi]
    pub async fn subscribe_symbol_chunked(
        &self,
        symbol: String,
        chunk_size: u32,
    ) -> Result<StreamIterator> {
        let stream_asset = self
            .client
            .subscribe_symbol_chuncked(symbol, chunk_size as usize)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let boxed_stream = StreamAsset::to_stream_static(Arc::new(stream_asset))
            .boxed()
            .fuse();
        let stream = Arc::new(Mutex::new(boxed_stream));
        Ok(StreamIterator { stream })
    }

    /// Subscribes to symbol updates with time-based delivery.
    ///
    /// # Arguments
    /// * `symbol` - The trading symbol to subscribe to (e.g., "EUR/USD")
    /// * `time_seconds` - Time interval in seconds between updates
    ///
    /// # Returns
    /// A StreamIterator for receiving time-based price updates
    ///
    /// # Examples
    /// ```javascript
    /// const stream = await client.subscribeSymbolTimed("EUR/USD", 5);
    /// for await (const update of stream) {
    ///     console.log(`Update at ${new Date()}: ${update.price}`);
    /// }
    /// ```
    #[napi]
    pub async fn subscribe_symbol_timed(
        &self,
        symbol: String,
        time_seconds: u32,
    ) -> Result<StreamIterator> {
        let stream_asset = self
            .client
            .subscribe_symbol_timed(symbol, Duration::from_secs(time_seconds as u64))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let boxed_stream = StreamAsset::to_stream_static(Arc::new(stream_asset))
            .boxed()
            .fuse();
        let stream = Arc::new(Mutex::new(boxed_stream));
        Ok(StreamIterator { stream })
    }

    /// Sends a raw WebSocket message to the server.
    ///
    /// # Arguments
    /// * `message` - The raw message string to send
    ///
    /// # Examples
    /// ```javascript
    /// await client.sendRawMessage(JSON.stringify({
    ///     type: "custom_command",
    ///     data: { / ... / }
    /// }));
    /// ```
    #[napi]
    pub async fn send_raw_message(&self, message: String) -> Result<()> {
        self.client
            .send_raw_message(message)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Creates a raw order with custom validation.
    ///
    /// # Arguments
    /// * `message` - The raw order message
    /// * `validator` - A validator instance for response validation
    ///
    /// # Returns
    /// A JSON string containing the order result
    ///
    /// # Examples
    /// ```javascript
    /// const validator = new Validator();
    /// const result = await client.createRawOrder(
    ///     JSON.stringify({ / order details / }),
    ///     validator
    /// );
    /// ```
    #[napi]
    pub async fn create_raw_order(&self, message: String, validator: &Validator) -> Result<String> {
        let res = self
            .client
            .create_raw_order(message, validator.to_val())
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(res.to_string())
    }

    /// Creates a raw order with a timeout for response validation.
    ///
    /// # Arguments
    /// * `message` - The raw order message to send
    /// * `validator` - A validator instance for response validation
    /// * `timeout` - Timeout duration in seconds
    ///
    /// # Returns
    /// A JSON string containing the order result, or error if timeout is reached
    ///
    /// # Examples
    /// ```javascript
    /// const validator = new Validator();
    /// const result = await client.createRawOrderWithTimeout(
    ///     JSON.stringify({ / order details / }),
    ///     validator,
    ///     30 // 30 seconds timeout
    /// );
    /// ```
    #[napi]
    pub async fn create_raw_order_with_timeout(
        &self,
        message: String,
        validator: &Validator,
        timeout: u32,
    ) -> Result<String> {
        let res = self
            .client
            .create_raw_order_with_timeout(
                message,
                validator.to_val(),
                Duration::from_secs(timeout as u64),
            )
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(res.to_string())
    }

    /// Creates a raw order with timeout and automatic retry functionality.
    ///
    /// # Arguments
    /// * `message` - The raw order message to send
    /// * `validator` - A validator instance for response validation
    /// * `timeout` - Timeout duration in seconds for each attempt
    ///
    /// # Returns
    /// A JSON string containing the order result, or error if all retries fail
    ///
    /// # Examples
    /// ```javascript
    /// const validator = new Validator();
    /// const result = await client.createRawOrderWithTimeoutAndRetry(
    ///     JSON.stringify({ / order details / }),
    ///     validator,
    ///     15 // 15 seconds timeout per attempt
    /// );
    /// ```
    #[napi]
    pub async fn create_raw_order_with_timeout_and_retry(
        &self,
        message: String,
        validator: &Validator,
        timeout: u32,
    ) -> Result<String> {
        let res = self
            .client
            .create_raw_order_with_timeout_and_retry(
                message,
                validator.to_val(),
                Duration::from_secs(timeout as u64),
            )
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(res.to_string())
    }

    /// Creates an iterator for handling raw WebSocket messages with validation.
    ///
    /// # Arguments
    /// * `message` - The initial message to send to establish the stream
    /// * `validator` - A validator instance for filtering messages
    /// * `timeout` - Optional timeout duration in seconds for the stream
    ///
    /// # Returns
    /// A RawStreamIterator that yields validated messages
    ///
    /// # Examples
    /// ```javascript
    /// const validator = new Validator();
    /// const stream = await client.createRawIterator(
    ///     JSON.stringify({ / subscription details / }),
    ///     validator,
    ///     60 // Optional: 60 seconds timeout
    /// );
    /// for await (const message of stream) {
    ///     console.log(`Received: ${message}`);
    /// }
    /// ```
    #[napi]
    pub async fn create_raw_iterator(
        &self,
        message: String,
        validator: &Validator,
        timeout: Option<u32>,
    ) -> Result<RawStreamIterator> {
        let timeout = timeout.map(|t| Duration::from_secs(t as u64));
        let stream = self
            .client
            .create_raw_iterator(message, validator.to_val(), timeout)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let boxed_stream = FilteredRecieverStream::to_stream_static(Arc::new(stream))
            .boxed()
            .fuse();
        let stream = Arc::new(Mutex::new(boxed_stream));
        Ok(RawStreamIterator { stream })
    }

    /// Returns the current server time as a UNIX timestamp
    #[napi]
    pub async fn get_server_time(&self) -> i64  {

        let time = self.client.get_server_time().await;
        time.timestamp()
    }
}

#[napi]
impl StreamIterator {
    /// Gets the next price update from the stream.
    ///
    /// # Returns
    /// * A Promise that resolves to:
    ///   * A JSON string containing the next price update data
    ///   * null if the stream has ended
    /// * Rejects with an error if the stream encounters an error
    ///
    /// # Examples
    /// ```javascript
    /// const stream = await client.subscribeSymbol('EUR/USD');
    ///
    /// // Using async/await
    /// try {
    ///     while (true) {
    ///         const update = await stream.next();
    ///         if (update === null) break;
    ///         const data = JSON.parse(update);
    ///         console.log('Price:', data.price);
    ///     }
    /// } catch (err) {
    ///     console.error('Stream error:', err);
    /// }
    /// ```
    #[napi]
    pub async fn next(&self) -> Result<Option<Value>> {
        let mut stream = self.stream.lock().await;
        match stream.next().await {
            Some(Ok(candle)) => serde_json::to_value(&candle)
                .map(Some)
                .map_err(|e| Error::from_reason(e.to_string())),
            Some(Err(e)) => Err(Error::from_reason(e.to_string())),
            None => Ok(None),
        }
    }
}

#[napi]
impl RawStreamIterator {
    /// Gets the next raw WebSocket message from the stream.
    ///
    /// # Returns
    /// * A Promise that resolves to:
    ///   * A string containing the raw WebSocket message
    ///   * null if the stream has ended
    /// * Rejects with an error if the stream encounters an error
    ///
    /// # Examples
    /// ```javascript
    /// const stream = await client.createRawIterator(
    ///     JSON.stringify({ type: 'subscribe' }),
    ///     validator
    /// );
    ///
    /// // Using async/await
    /// try {
    ///     while (true) {
    ///         const message = await stream.next();
    ///         if (message === null) break;
    ///         console.log('Raw message:', message);
    ///     }
    /// } catch (err) {
    ///     console.error('Stream error:', err);
    /// }
    ///
    /// // Using for-await-of
    /// try {
    ///     for await (const message of stream) {
    ///         console.log('Raw message:', message);
    ///     }
    /// } catch (err) {
    ///     console.error('Stream error:', err);
    /// }
    /// ```
    #[napi]
    pub async fn next(&self) -> Result<Option<String>> {
        let mut stream = self.stream.lock().await;
        match stream.next().await {
            Some(Ok(msg)) => Ok(Some(msg.to_string())),
            Some(Err(e)) => Err(Error::from_reason(e.to_string())),
            None => Ok(None),
        }
    }
}
