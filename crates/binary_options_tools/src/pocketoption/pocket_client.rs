use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};
use url::Url;
use uuid::Uuid;

use crate::pocketoption::{
    error::PocketResult,
    parser::basic::LoadHistoryPeriod,
    types::order::SuccessCloseOrder,
    validators::{candle_validator, order_result_validator},
    ws::ssid::Ssid,
};
use binary_options_tools_core::{
    error::BinaryOptionsToolsError,
    general::{
        client::WebSocketClient,
        config::{_Config, Config},
        stream::FilteredRecieverStream,
        traits::{MessageTransfer, ValidatorTrait},
        types::{Callback, Data},
    },
};

use super::{
    error::PocketOptionError,
    parser::message::WebSocketMessage,
    types::{
        base::{ChangeSymbol, RawWebsocketMessage},
        callback::PocketCallback,
        data::PocketData,
        info::MessageInfo,
        order::{Action, Deal, OpenOrder},
        update::{DataCandle, UpdateBalance},
    },
    validators::{history_validator, order_validator},
    ws::{connect::PocketConnect, listener::Handler, stream::StreamAsset},
};

/// A client for interacting with the Pocket Option trading platform.
/// This struct provides methods for executing trades, managing positions,
/// streaming market data, and accessing account information.
///
/// # Features
/// - Real-time market data streaming
/// - Binary options trading (CALL/PUT)
/// - Account management
/// - Historical data access
/// - Raw WebSocket message handling
///
/// # Examples
/// Basic usage:
/// ```rust
/// use binary_options_tools::pocketoption::pocket_client::PocketOption;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize client with session ID
///     let client = PocketOption::new("your-ssid-here").await?;
///
///     // Execute a trade
///     let trade = client.buy("EURUSD", 100.0, 60).await?;
///
///     // Check trade result
///     let result = client.check_win(&trade.id).await?;
///
///     // Stream market data
///     let stream = client.subscribe_symbol("EURUSD").await?;
///     while let Some(candle) = stream.to_stream().next().await {
///         println!("New candle: {:?}", candle);
///     }
///     Ok(())
/// }
/// ```
///
/// # Authentication
/// The client requires a valid session ID (SSID) for authentication. The SSID can be obtained
/// from the Pocket Option platform after logging in.
///
/// # WebSocket Connection
/// The client maintains a WebSocket connection to the Pocket Option servers and automatically
/// handles reconnection in case of disconnects.
///
/// # Error Handling
/// Most methods return a `PocketResult<T>` which can contain various error types:
/// - Connection errors
/// - Authentication failures
/// - Invalid parameters
/// - Trading restrictions
/// - Server errors
///
/// # Thread Safety
/// The client is inherently thread-safe as it internally uses an Arc-wrapped WebSocket client.
/// It can be safely cloned and shared between multiple tasks.
#[derive(Clone)]
pub struct PocketOption {
    client: WebSocketClient<WebSocketMessage, Handler, PocketConnect, Ssid, PocketData, ()>,
}

impl Deref for PocketOption {
    type Target = Config<PocketData, WebSocketMessage, ()>;

    fn deref(&self) -> &Self::Target {
        &self.client.config
    }
}

impl PocketOption {
    /// Creates a new PocketOption client with default connection settings.
    ///
    /// # Arguments
    /// * `ssid` - Session ID for authentication, can be any type that implements ToString
    ///
    /// # Returns
    /// A Result containing the initialized PocketOption client or an error
    ///
    /// # Examples
    /// ```rust
    /// let client = PocketOption::new("your-session-id").await?;
    /// ```
    pub async fn new(ssid: impl ToString) -> PocketResult<Self> {
        let ssid = Ssid::parse(ssid)?;
        let data = Data::new(PocketData::default());
        let handler = Handler::new(ssid.clone());
        let timeout = Duration::from_millis(500);
        let callback = PocketCallback;
        let config = _Config::new(timeout, vec![], ())
            .builder()
            .reconnect_time(5)
            .build()?;
        let client = WebSocketClient::init(
            ssid,
            PocketConnect {},
            data,
            handler,
            Some(Callback::new(std::sync::Arc::new(callback))),
            config,
        )
        .await?;
        Ok(Self { client })
    }

    /// Creates a new PocketOption client with a custom WebSocket URL.
    ///
    /// # Arguments
    /// * `ssid` - Session ID for authentication, can be any type that implements ToString
    /// * `url` - Custom WebSocket URL to connect to
    ///
    /// # Returns
    /// A Result containing the initialized PocketOption client or an error
    ///
    /// # Examples
    /// ```rust
    /// use url::Url;
    /// let url = Url::parse("wss://custom.pocketoption.com/websocket")?;
    /// let client = PocketOption::new_with_url("your-session-id", url).await?;
    /// ```
    pub async fn new_with_url(ssid: impl ToString, url: Url) -> PocketResult<Self> {
        let ssid = Ssid::parse(ssid)?;
        let data = Data::new(PocketData::default());
        let handler = Handler::new(ssid.clone());
        let timeout = Duration::from_millis(500);
        let callback = PocketCallback;
        let config = _Config::new(timeout, vec![], ())
            .builder()
            .reconnect_time(5)
            .default_connection_url(HashSet::from([url]))
            .build()?;
        let client = WebSocketClient::init(
            ssid,
            PocketConnect {},
            data,
            handler,
            Some(Callback::new(std::sync::Arc::new(callback))),
            config,
        )
        .await?;
        // println!("Initialized!");
        Ok(Self { client })
    }

    /// Creates a new PocketOption client with a provided configuration.
    ///
    /// # Arguments
    /// * `ssid` - Session ID for authentication
    /// * `config` - Custom configuration for the client
    ///
    /// # Returns
    /// A Result containing the initialized PocketOption client or an error
    ///
    /// # Examples
    /// ```rust
    /// let config = Config::new(timeout, vec![], Box::new(()))
    ///     .builder()
    ///     .reconnect_time(5)
    ///     .build()?;
    /// let client = PocketOption::new_with_config("your-session-id", config).await?;
    /// ```
    pub async fn new_with_config(
        ssid: impl ToString,
        config: Config<PocketData, WebSocketMessage, ()>,
    ) -> PocketResult<Self> {
        let ssid = Ssid::parse(ssid)?;
        let data = Data::new(PocketData::default());
        let handler = Handler::new(ssid.clone());
        let callback = PocketCallback;

        let client = WebSocketClient::init(
            ssid,
            PocketConnect {},
            data,
            handler,
            Some(Callback::new(std::sync::Arc::new(callback))),
            config,
        )
        .await?;

        Ok(Self { client })
    }

    /// Executes a trade with the specified parameters.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD")
    /// * `action` - Trade direction (Call/Put)
    /// * `amount` - Trade amount in account currency
    /// * `time` - Trade duration in seconds
    ///
    /// # Returns
    /// A tuple containing the trade ID (UUID) and trade details (Deal)
    ///
    /// # Examples
    /// ```rust
    /// let (trade_id, deal) = client.trade("EURUSD", Action::Call, 100.0, 60).await?;
    /// ```
    pub async fn trade(
        &self,
        asset: impl ToString,
        action: Action,
        amount: f64,
        time: u32,
    ) -> PocketResult<(Uuid, Deal)> {
        let order = OpenOrder::new(
            amount,
            asset.to_string(),
            action,
            time,
            self.client.credentials.demo() as u32,
        )?;
        let request_id = order.request_id;
        let res = self
            .client
            .send_message_with_timout(
                self.get_timeout()?,
                "Trade",
                WebSocketMessage::OpenOrder(order),
                MessageInfo::SuccessopenOrder,
                &order_validator(request_id),
            )
            .await?;
        if let WebSocketMessage::SuccessopenOrder(order) = res {
            debug!("Successfully opened buy trade!");
            return Ok((order.id, order));
        }
        Err(PocketOptionError::UnexpectedIncorrectWebSocketMessage(
            res.info(),
        ))
    }

    /// Places a buy (CALL) order.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD")
    /// * `amount` - Trade amount in account currency
    /// * `time` - Trade duration in seconds
    ///
    /// # Returns
    /// A tuple containing the trade ID (UUID) and trade details (Deal)
    ///
    /// # Examples
    /// ```rust
    /// let (trade_id, deal) = client.buy("EURUSD", 100.0, 60).await?;
    /// ```
    pub async fn buy(
        &self,
        asset: impl ToString,
        amount: f64,
        time: u32,
    ) -> PocketResult<(Uuid, Deal)> {
        info!(target: "Buy", "Placing a buy trade for asset '{}', with amount '{}' and time '{}'", asset.to_string(), amount, time);
        self.trade(asset, Action::Call, amount, time).await
    }

    /// Places a sell (PUT) order.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD")
    /// * `amount` - Trade amount in account currency
    /// * `time` - Trade duration in seconds
    ///
    /// # Returns
    /// A tuple containing the trade ID (UUID) and trade details (Deal)
    ///
    /// # Examples
    /// ```rust
    /// let (trade_id, deal) = client.sell("EURUSD", 100.0, 60).await?;
    /// ```
    pub async fn sell(
        &self,
        asset: impl ToString,
        amount: f64,
        time: u32,
    ) -> PocketResult<(Uuid, Deal)> {
        info!(target: "Sell", "Placing a sell trade for asset '{}', with amount '{}' and time '{}'", asset.to_string(), amount, time);
        self.trade(asset, Action::Put, amount, time).await
    }

    /// Gets the end time of a deal by its ID.
    ///
    /// # Arguments
    /// * `id` - UUID of the trade
    ///
    /// # Returns
    /// Optional DateTime indicating when the trade will expire, adjusted for server time
    /// Returns None if the trade is not found
    pub async fn get_deal_end_time(&self, id: Uuid) -> Option<DateTime<Utc>> {
        if let Some(trade) = self
            .client
            .data
            .get_opened_deals()
            .await
            .iter()
            .find(|d| *d == &id)
        {
            return Some(trade.close_timestamp - Duration::from_secs(2 * 3600)); // Pocket Option server seems 2 hours advanced
        }

        if let Some(trade) = self
            .client
            .data
            .get_opened_deals()
            .await
            .iter()
            .find(|d| *d == &id)
        {
            return Some(trade.close_timestamp - Duration::from_secs(2 * 3600)); // Pocket Option server seems 2 hours advanced
        }
        None
    }

    /// Checks the results of a trade by its ID.
    ///
    /// # Arguments
    /// * `trade_id` - UUID of the trade to check
    ///
    /// # Returns
    /// Deal information containing profit/loss and other trade details
    ///
    /// # Errors
    /// Returns an OutOfRangeError if the deal is not found in closed deals after expiration
    ///
    /// # Examples
    /// ```rust
    /// let result = client.check_results(trade_id).await?;
    /// println!("Trade profit: {}", result.profit);
    /// ```
    pub async fn check_results(&self, trade_id: Uuid) -> PocketResult<Deal> {
        info!(target: "CheckResults", "Checking results for trade of id {}", trade_id);
        if let Some(trade) = self
            .client
            .data
            .get_closed_deals()
            .await
            .iter()
            .find(|d| d.id == trade_id)
        {
            return Ok(trade.clone());
        }
        debug!("Trade result not found in closed deals list, waiting for closing order to check.");
        if let Some(timestamp) = self.get_deal_end_time(trade_id).await {
            let exp = timestamp
                .signed_duration_since(Utc::now() - self.get_timeout()?) // TODO: Change this since the current time depends on the timezone.
                .to_std()
                .map_err(BinaryOptionsToolsError::from)?;
            debug!(target: "CheckResult", "Expiration time in {exp:?} seconds.");
            let start = Instant::now();
            // println!("Expiration time in {exp:?} seconds.");
            let res: WebSocketMessage = match self
                .client
                .send_message_with_timeout_and_retry(
                    exp + self.get_timeout()?,
                    "CheckResult",
                    WebSocketMessage::None,
                    MessageInfo::SuccesscloseOrder,
                    &order_result_validator(trade_id),
                )
                .await
            {
                Ok(msg) => msg,
                Err(e) => {
                    info!(target: "CheckResults", "Time elapsed, {:?}, checking closed deals one last time.", start.elapsed());
                    if let Some(deal) = self
                        .get_closed_deals()
                        .await
                        .iter()
                        .find(|d| d.id == trade_id)
                    {
                        WebSocketMessage::SuccesscloseOrder(SuccessCloseOrder {
                            profit: 0.0,
                            deals: vec![deal.to_owned()],
                        })
                    } else {
                        return Err(e.into());
                    }
                }
            };

            if let WebSocketMessage::SuccesscloseOrder(order) = res {
                return order
                    .deals
                    .iter()
                    .find(|d| d.id == trade_id)
                    .cloned()
                    .ok_or(PocketOptionError::UnreachableError(
                        "Error finding correct trade".into(),
                    ));
            }
            return Err(PocketOptionError::UnexpectedIncorrectWebSocketMessage(
                res.info(),
            ));
        }
        warn!("No opened trade with the given uuid please check if you are passing the correct id");
        Err(BinaryOptionsToolsError::Unallowed("Couldn't check result for a deal that is not in the list of opened trades nor closed trades.".into()).into())
    }

    pub async fn get_candles_advanced(
        &self,
        asset: impl ToString,
        time: i64,
        period: i64,
        offset: i64,
    ) -> PocketResult<Vec<DataCandle>> {
        info!(target: "GetCandlesAdvanced", "Retrieving candles for asset '{}' with period of '{}' and offset of '{}'", asset.to_string(), period, offset);
        if time == 0 {
            return Err(PocketOptionError::GeneralParsingError(
                "Server time is invalid.".to_string(),
            ));
        }
        let request = LoadHistoryPeriod::new(asset.to_string(), time, period, offset)?;
        let index = request.index;
        debug!(
            "Sent get candles message, message: {:?}",
            WebSocketMessage::GetCandles(request).to_string()
        );
        let request = LoadHistoryPeriod::new(asset.to_string(), time, period, offset)?;
        let res = self
            .client
            .send_message_with_timeout_and_retry(
                self.get_timeout()?,
                "GetCandles",
                WebSocketMessage::GetCandles(request),
                MessageInfo::LoadHistoryPeriod,
                &candle_validator(index),
            )
            .await?;
        if let WebSocketMessage::LoadHistoryPeriod(history) = res {
            return Ok(history.candle_data());
        }
        Err(PocketOptionError::UnexpectedIncorrectWebSocketMessage(
            res.info(),
        ))
    }

    /// Retrieves historical candle data for a specific asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD")
    /// * `period` - Time period for each candle in seconds
    /// * `offset` - Number of periods to offset from current time
    ///
    /// # Returns
    /// A vector of DataCandle objects containing historical price data
    ///
    /// # Errors
    /// * Returns GeneralParsingError if server time is invalid
    /// * Returns UnexpectedIncorrectWebSocketMessage if response format is incorrect
    ///
    /// # Examples
    /// ```rust
    /// let candles = client.get_candles("EURUSD", 60, 0).await?; // Get current minute candles
    /// ```
    pub async fn get_candles(
        &self,
        asset: impl ToString,
        period: i64,
        offset: i64,
    ) -> PocketResult<Vec<DataCandle>> {
        let time = self.client.data.get_server_time().await.div_euclid(period) * period;
        self.get_candles_advanced(asset, time, period, offset).await
    }

    /// Retrieves the most recent historical data for an asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD")
    /// * `period` - Time period for each candle in seconds
    ///
    /// # Returns
    /// A vector of DataCandle objects containing recent price data
    ///
    /// # Examples
    /// ```rust
    /// let recent_data = client.history("EURUSD", 60).await?; // Get recent minute data
    /// ```
    pub async fn history(
        &self,
        asset: impl ToString,
        period: i64,
    ) -> PocketResult<Vec<DataCandle>> {
        info!(target: "History", "Retrieving candles for asset '{}' with period of '{}'", asset.to_string(), period);

        let request = ChangeSymbol::new(asset.to_string(), period);
        let res = self
            .client
            .send_message_with_timeout_and_retry(
                self.get_timeout()?,
                "History",
                WebSocketMessage::ChangeSymbol(request),
                MessageInfo::UpdateHistoryNewFast,
                &history_validator(asset.to_string(), period),
            )
            .await?;
        if let WebSocketMessage::UpdateHistoryNewFast(history) = res {
            return Ok(history.candle_data());
        }
        Err(PocketOptionError::UnexpectedIncorrectWebSocketMessage(
            res.info(),
        ))
    }

    pub async fn get_closed_deals(&self) -> Vec<Deal> {
        info!(target: "GetClosedDeals", "Retrieving list of closed deals");
        self.client.data.get_closed_deals().await
    }

    pub async fn clear_closed_deals(&self) {
        info!(target: "ClearClosedDeals", "Clearing list of closed deals");
        self.client.data.clean_closed_deals().await
    }

    pub async fn get_opened_deals(&self) -> Vec<Deal> {
        info!(target: "GetOpenDeals", "Retrieving list of open deals");
        self.client.data.get_opened_deals().await
    }

    pub async fn get_balance(&self) -> UpdateBalance {
        info!(target: "GetBalance", "Retrieving account balance");
        self.client.data.get_balance().await
    }

    pub async fn is_demo(&self) -> bool {
        info!(target: "IsDemo", "Retrieving demo status");
        self.client.credentials.demo()
    }

    pub async fn get_payout(&self) -> HashMap<String, i32> {
        info!(target: "GetPayout", "Retrieving payout for all the assets");
        self.client.data.get_full_payout().await
    }

    /// Subscribes to real-time price updates for an asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol to subscribe to (e.g., "EURUSD")
    ///
    /// # Returns
    /// A StreamAsset object that can be used to receive real-time updates
    ///
    /// # Examples
    /// ```rust
    /// let stream = client.subscribe_symbol("EURUSD").await?;
    /// while let Some(update) = stream.next().await {
    ///     println!("New price: {:?}", update);
    /// }
    /// ```
    pub async fn subscribe_symbol(&self, asset: impl ToString) -> PocketResult<StreamAsset> {
        info!(target: "SubscribeSymbol", "Subscribing to asset '{}'", asset.to_string());
        // Send 3 messages, 1 changesymbol, 2 unsubfor, 3 subfor 
        self.client.send(WebSocketMessage::ChangeSymbol(ChangeSymbol::new(asset.to_string(), 1))).await?;
        self.client.send(WebSocketMessage::Unsubfor(asset.to_string())).await?;
        self.client.send(WebSocketMessage::Subfor(asset.to_string())).await?;
        debug!("Created StreamAsset instance.");
        Ok(self.client.data.add_stream(asset.to_string()).await)
    }

    /// Subscribes to chunked real-time price updates for an asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol to subscribe to (e.g., "EURUSD")
    /// * `chunck_size` - Number of updates to group together into a single candle
    ///
    /// # Returns
    /// A StreamAsset object that emits chunks of price updates
    ///
    /// # Examples
    /// ```rust
    /// let stream = client.subscribe_symbol_chuncked("EURUSD", 5).await?;
    /// // Will receive updates in groups of 5
    /// ```
    pub async fn subscribe_symbol_chuncked(
        &self,
        asset: impl ToString,
        chunck_size: impl Into<usize>,
    ) -> PocketResult<StreamAsset> {
        info!(target: "SubscribeSymbolChuncked", "Subscribing to asset '{}'", asset.to_string());
        // Send 3 messages, 1 changesymbol, 2 unsubfor, 3 subfor, honestly no clue why pocket option does that
        self.client.send(WebSocketMessage::ChangeSymbol(ChangeSymbol::new(asset.to_string(), 1))).await?;
        self.client.send(WebSocketMessage::Unsubfor(asset.to_string())).await?;
        self.client.send(WebSocketMessage::Subfor(asset.to_string())).await?;
        debug!("Created StreamAsset instance.");
        Ok(self
            .client
            .data
            .add_stream_chuncked(asset.to_string(), chunck_size.into())
            .await)
    }

    /// Subscribes to time-based real-time price updates for an asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol to subscribe to (e.g., "EURUSD")
    /// * `time` - Time duration between updates
    ///
    /// # Returns
    /// A StreamAsset object that emits updates at specified time intervals
    ///
    /// # Examples
    /// ```rust
    /// use std::time::Duration;
    /// let stream = client.subscribe_symbol_timed("EURUSD", Duration::from_secs(5)).await?;
    /// // Will receive updates every 5 seconds
    /// ```
    pub async fn subscribe_symbol_timed(
        &self,
        asset: impl ToString,
        time: impl Into<Duration>,
    ) -> PocketResult<StreamAsset> {
        info!(target: "SubscribeSymbolTimed", "Subscribing to asset '{}'", asset.to_string());
        // Send 3 messages, 1 changesymbol, 2 unsubfor, 3 subfor 
        self.client.send(WebSocketMessage::ChangeSymbol(ChangeSymbol::new(asset.to_string(), 1))).await?;
        self.client.send(WebSocketMessage::Unsubfor(asset.to_string())).await?;
        self.client.send(WebSocketMessage::Subfor(asset.to_string())).await?;
        debug!("Created StreamAsset instance.");
        Ok(self
            .client
            .data
            .add_stream_timed(asset.to_string(), time.into())
            .await)
    }

    /// Sends a raw WebSocket message without waiting for a response.
    ///
    /// # Arguments
    /// * `message` - Raw message to send to the WebSocket server
    ///
    /// # Returns
    /// Returns Ok(()) if the message was sent successfully
    ///
    /// # Examples
    /// ```rust
    /// client.send_raw_message(r#"42["signals/subscribe"]"#).await?;
    /// ```
    pub async fn send_raw_message(&self, message: impl ToString) -> PocketResult<()> {
        self.client
            .raw_send(RawWebsocketMessage::from(message.to_string()))
            .await?;
        Ok(())
    }

    /// Sends a raw WebSocket message and waits for a validated response.
    ///
    /// # Arguments
    /// * `message` - Raw message or RawWebsocketMessage to send
    /// * `validator` - Validator instance to filter and validate the response
    ///
    /// # Returns
    /// The first validated response message
    ///
    /// # Examples
    /// ```rust
    /// let validator = Box::new(RawValidator::starts_with(r#"42["signals/load""#));
    /// let response = client.create_raw_order(
    ///     r#"42["signals/subscribe"]"#,
    ///     validator
    /// ).await?;
    /// ```
    pub async fn create_raw_order(
        &self,
        message: impl Into<RawWebsocketMessage>,
        validator: Box<dyn ValidatorTrait<RawWebsocketMessage> + Send + Sync>,
    ) -> PocketResult<RawWebsocketMessage> {
        // TODO: Complete this function + add the following functionality
        //  * create_raw_order_with_timeout
        //  * create_raw_order_iterator: return a stream like the StreamAsset
        //  * send_raw_message: send message without validator
        //  * OTHER: Create a callback related function to add new options for the callback + add support for struct or functions in it (like the Validator) so future me will have it easy
        Ok(self
            .client
            .send_raw_message(message.into(), validator)
            .await?)
    }

    /// Sends a raw WebSocket message and waits for a validated response with a timeout.
    ///
    /// # Arguments
    /// * `message` - Raw message or RawWebsocketMessage to send
    /// * `validator` - Validator instance to filter and validate the response
    /// * `timeout` - Maximum time to wait for a response
    ///
    /// # Returns
    /// The first validated response message or times out
    ///
    /// # Errors
    /// Returns TimeoutError if no valid response is received within the timeout period
    ///
    /// # Examples
    /// ```rust
    /// use std::time::Duration;
    ///
    /// let validator = Box::new(RawValidator::starts_with(r#"42["signals/load""#));
    /// let response = client.create_raw_order_with_timeout(
    ///     r#"42["signals/subscribe"]"#,
    ///     validator,
    ///     Duration::from_secs(5)
    /// ).await?;
    /// ```
    pub async fn create_raw_order_with_timeout(
        &self,
        message: impl Into<RawWebsocketMessage>,
        validator: Box<dyn ValidatorTrait<RawWebsocketMessage> + Send + Sync>,
        timeout: Duration,
    ) -> PocketResult<RawWebsocketMessage> {
        Ok(self
            .client
            .send_raw_message_with_timout(
                timeout,
                "CreateRawOrder".to_string(),
                message.into(),
                validator,
            )
            .await?)
    }

    /// Sends a raw WebSocket message with timeout and automatic retry on failure.
    ///
    /// # Arguments
    /// * `message` - Raw message or RawWebsocketMessage to send
    /// * `validator` - Validator instance to filter and validate the response
    /// * `timeout` - Maximum time to wait for each attempt
    ///
    /// # Returns
    /// The first validated response message
    ///
    /// # Notes
    /// Will retry the request if a timeout occurs, using exponential backoff
    ///
    /// # Examples
    /// ```rust
    /// use std::time::Duration;
    ///
    /// let validator = Box::new(RawValidator::starts_with(r#"42["signals/load""#));
    /// let response = client.create_raw_order_with_timeout_and_retry(
    ///     r#"42["signals/subscribe"]"#,
    ///     validator,
    ///     Duration::from_secs(5)
    /// ).await?;
    /// ```
    pub async fn create_raw_order_with_timeout_and_retry(
        &self,
        message: impl Into<RawWebsocketMessage>,
        validator: Box<dyn ValidatorTrait<RawWebsocketMessage> + Send + Sync>,
        timeout: Duration,
    ) -> PocketResult<RawWebsocketMessage> {
        Ok(self
            .client
            .send_raw_message_with_timeout_and_retry(
                timeout,
                "CreateRawOrderWithRetry".to_string(),
                message.into(),
                validator,
            )
            .await?)
    }

    /// Creates a stream of validated WebSocket messages.
    ///
    /// # Arguments
    /// * `message` - Initial message to establish the stream
    /// * `validator` - Validator instance to filter incoming messages
    /// * `timeout` - Optional timeout for the entire stream
    ///
    /// # Returns
    /// A stream that yields validated WebSocket messages
    ///
    /// # Examples
    /// ```rust
    /// use std::time::Duration;
    ///
    /// let validator = Box::new(RawValidator::starts_with(r#"42["signals/load""#));
    /// let stream = client.create_raw_iterator(
    ///     r#"42["signals/subscribe"]"#,
    ///     validator,
    ///     Some(Duration::from_secs(60))
    /// ).await?;
    ///
    /// while let Some(message) = stream.next().await {
    ///     println!("Received: {}", message?);
    /// }
    /// ```
    pub async fn create_raw_iterator(
        &self,
        message: impl Into<RawWebsocketMessage>,
        validator: Box<dyn ValidatorTrait<RawWebsocketMessage> + Send + Sync>,
        timeout: Option<Duration>,
    ) -> PocketResult<FilteredRecieverStream<RawWebsocketMessage>> {
        Ok(self
            .client
            .send_raw_message_iterator(message.into(), validator, timeout)
            .await?)
    }

    pub async fn get_server_time(&self) -> DateTime<Utc> {
        Utc::now() + Duration::from_secs(2 * 3600 + 123)
    }

    pub fn kill(self) {
        drop(self)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use futures_util::{
        StreamExt,
        future::{try_join, try_join_all, try_join3},
    };
    use rand::{random, rng, seq::IndexedRandom};
    use tokio::{task::JoinHandle, time::sleep};

    use binary_options_tools_core::utils::tracing::{start_tracing, start_tracing_leveled};
    use tracing::level_filters::LevelFilter;
    use url::Url;

    use super::*;

    fn to_future(stream: StreamAsset, id: i32) -> JoinHandle<anyhow::Result<()>> {
        tokio::spawn(async move {
            while let Some(item) = stream.to_stream().next().await {
                info!("StreamAsset n°{}, candle: {}", id, item?);
            }
            Ok(())
        })
    }

    #[tokio::test]
    #[should_panic(expected = "MaxDemoTrades")]
    async fn test_pocket_option() {
        // start_tracing()?;
        let ssid = r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#;
        let api = PocketOption::new(ssid).await.unwrap();
        // let mut loops = 0;
        // while loops < 100 {
        //     loops += 1;
        //     sleep(Duration::from_millis(100)).await;
        // }
        for i in 0..100 {
            let now = Instant::now();
            let _ = api.buy("EURUSD_otc", 1.0, 60).await.expect("MaxDemoTrades");
            println!("Loop n°{i}, Elapsed time: {:.8?} ms", now.elapsed());
        }
    }

    #[tokio::test]
    async fn test_subscribe_symbol_v2() -> anyhow::Result<()> {
        start_tracing(true)?;
        // start_tracing()?;
        let ssid = r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await?;
        let stream_asset1 = client.subscribe_symbol("EURUSD_otc").await?;
        let stream_asset2 = client.subscribe_symbol("#FB_otc").await?;
        let stream_asset3 = client.subscribe_symbol("YERUSD_otc").await?;

        let f1 = to_future(stream_asset1, 1);
        let f2 = to_future(stream_asset2, 2);
        let f3 = to_future(stream_asset3, 3);
        let _ = try_join3(f1, f2, f3).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_subscribe_symbol_real_v2() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::DEBUG)?;
        // start_tracing()?;
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"02ac5a5875a4b583042aae064351e0bb\";s:10:\"ip_address\";s:13:\"191.113.133.5\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 OPR/116.\";s:13:\"last_activity\";i:1740838529;}0850af6716a7a1d1eea59e80987f4a59","isDemo":0,"uid":87742848,"platform":2}]	"#;
        info!("SSID: {}", ssid);
        let client = PocketOption::new(ssid).await?;
        let stream_asset1 = client.subscribe_symbol("EURUSD_otc").await?;
        let stream_asset2 = client.subscribe_symbol("#FB_otc").await?;
        let stream_asset3 = client.subscribe_symbol("YERUSD_otc").await?;

        let f1 = to_future(stream_asset1, 1);
        let f2 = to_future(stream_asset2, 2);
        let f3 = to_future(stream_asset3, 3);
        let _ = try_join3(f1, f2, f3).await?;
        Ok(())
    }
    #[tokio::test]
    async fn test_subscribe_symbol_real_timed() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::INFO)?;
        // start_tracing()?;
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"7f57151f639ae5c46afe607bc18b8c45\";s:10:\"ip_address\";s:14:\"201.189.135.40\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 OPR/114.\";s:13:\"last_activity\";i:1739194195;}99082f426830d4692e6b6bf194ed94b2","isDemo":0,"uid":87742848,"platform":2}]	"#;
        info!("SSID: {}", ssid);
        let client = PocketOption::new(ssid).await?;
        let stream_asset1 = client
            .subscribe_symbol_timed("EURUSD_otc", Duration::from_secs(5))
            .await?;
        let stream_asset2 = client
            .subscribe_symbol_timed("#FB_otc", Duration::from_secs(5))
            .await?;
        let stream_asset3 = client
            .subscribe_symbol_timed("YERUSD_otc", Duration::from_secs(5))
            .await?;

        let f1 = to_future(stream_asset1, 1);
        let f2 = to_future(stream_asset2, 2);
        let f3 = to_future(stream_asset3, 3);
        let _ = try_join3(f1, f2, f3).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_subscribe_symbol_timed() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::INFO)?;
        // start_tracing()?;
        let ssid = r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await?;
        let stream_asset1 = client
            .subscribe_symbol_timed("EURUSD_otc", Duration::from_secs(30))
            .await?;
        let stream_asset2 = client
            .subscribe_symbol_timed("#FB_otc", Duration::from_secs(15))
            .await?;
        let stream_asset3 = client
            .subscribe_symbol_timed("YERUSD_otc", Duration::from_secs(60))
            .await?;

        let f1 = to_future(stream_asset1, 1);
        let f2 = to_future(stream_asset2, 2);
        let f3 = to_future(stream_asset3, 3);
        let _ = try_join3(f1, f2, f3).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_payout() -> anyhow::Result<()> {
        start_tracing(true)?;
        let ssid = r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#;
        let api = PocketOption::new(ssid).await?;

        tokio::time::sleep(Duration::from_secs(6)).await;
        dbg!(api.get_payout().await);
        Ok(())
    }

    #[tokio::test]
    async fn test_use_default_url() -> anyhow::Result<()> {
        start_tracing(true)?;
        let ssid = r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#;
        let api = PocketOption::new_with_url(
            ssid,
            Url::parse("wss://demo-api-eu.po.market/socket.io/?EIO=4&transport=websocket")?,
        )
        .await?;

        tokio::time::sleep(Duration::from_secs(6)).await;
        dbg!(api.get_payout().await);
        Ok(())
    }

    #[tokio::test]
    async fn test_check_win_v1() -> anyhow::Result<()> {
        start_tracing(true)?;
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await.unwrap();
        let mut test = 0;
        let mut checks = Vec::new();
        while test < 1000 {
            test += 1;
            if test % 100 == 0 {
                let res = client.sell("EURUSD_otc", 1.0, 15).await?;
                dbg!("Trade id: {}", res.0);
                let m_client = client.clone();
                let res: tokio::task::JoinHandle<Result<(), BinaryOptionsToolsError>> =
                    tokio::spawn(async move {
                        let result = m_client.check_results(res.0).await?;
                        dbg!("Trade result: {}", result.profit);
                        Ok(())
                    });
                checks.push(res);
            } else if test % 100 == 50 {
                let res = &client.buy("#AAPL_otc", 1.0, 5).await?;
                dbg!(res);
            }
            sleep(Duration::from_millis(100)).await;
        }
        try_join_all(checks).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_check_win_v2() -> anyhow::Result<()> {
        start_tracing(true)?;
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await.unwrap();
        let times = [5, 15, 30, 60, 300];
        for time in times {
            info!("Checkind for an expiration of '{time}' seconds!");
            let res: Result<(), BinaryOptionsToolsError> =
                tokio::time::timeout(Duration::from_secs(time as u64 + 30), async {
                    let (id1, _) = client.buy("EURUSD_otc", 1.5, time).await?;
                    let (id2, _) = client.sell("EURUSD_otc", 4.2, time).await?;
                    let r1 = client.check_results(id1).await?;
                    let r2 = client.check_results(id2).await?;
                    assert_eq!(r1.id, id1);
                    assert_eq!(r2.id, id2);
                    Ok(())
                })
                .await?;
            res?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_check_win_v3() -> anyhow::Result<()> {
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await.unwrap();
        let times = [5, 15, 30, 60, 300];
        let assets = ["#AAPL_otc", "#MSFT_otc", "EURUSD_otc", "YERUSD_otc"];
        for asset in assets {
            for time in times {
                println!("Checkind for an expiration of '{time}' seconds!");
                let at = tokio::time::Instant::now() + Duration::from_secs(time as u64 + 5);
                let res: Result<Duration, BinaryOptionsToolsError> =
                    tokio::time::timeout_at(at, async {
                        let start = tokio::time::Instant::now();
                        let (id1, _) = client.buy(asset, 1.5, time).await?;
                        let (id2, _) = client.sell(asset, 4.2, time).await?;
                        let r1 = client.check_results(id1).await?;
                        let r2 = client.check_results(id2).await?;
                        assert_eq!(r1.id, id1);
                        assert_eq!(r2.id, id2);
                        let elapsed = start.elapsed();
                        Ok(elapsed)
                    })
                    .await?;
                let duration = res?;
                println!(
                    "Test passed for expiration of '{time}' seconds in '{duration:#?}'!"
                );
            }
        }

        Ok(())
    }

    // #[tokio::test]
    // #[should_panic(expected = "CheckResults")]
    // async fn test_timeout() {
    //     let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
    //     let client = PocketOption::new(ssid).await.unwrap();
    //     let (id, _) = client.buy("EURUSD_otc", 1.5, 60).await.unwrap();
    //     dbg!(&id);
    //     let check = client.check_results(id);
    //     let res = binary_options_tools_core::utils::time::timeout(Duration::from_secs(30), check, "CheckResults".into())
    //         .await
    //         .expect("CheckResults");
    //     dbg!(res);
    // }

    #[tokio::test]
    async fn test_buy_check() -> anyhow::Result<()> {
        start_tracing(false)?;
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await.unwrap();
        let time_frames = [5, 15, 30, 60, 300];
        let assets = ["EURUSD_otc"];
        let mut rng = rng();
        loop {
            let amount = (random::<f64>() * 10.0).max(1.0);
            let asset = assets.choose(&mut rng).ok_or(anyhow::anyhow!("Error"))?;
            let timeframe = time_frames
                .choose(&mut rng)
                .ok_or(anyhow::anyhow!("Error"))?;
            let direction = if random() { Action::Call } else { Action::Put };
            println!(
                "Placing '{direction:?}' trade on asset '{asset}', amount '{amount}' usd and expiration of '{timeframe}'s."
            );
            let (id, _) = client
                .trade(asset, direction, amount, timeframe.to_owned())
                .await?;
            match client.check_results(id).await {
                Ok(res) => println!("Result for trade: {}", res.profit),
                Err(e) => eprintln!("Error, {e}\nTime: {}", Utc::now()),
            }
        }
    }

    #[tokio::test]
    async fn test_server_time() -> anyhow::Result<()> {
        // start_tracing(true)?;
        // start_tracing()?;
        let ssid = r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await?;
        let stream = client.subscribe_symbol("EURUSD_otc").await?;
        while let Some(item) = stream.to_stream().next().await {
            let time = item?.time;
            let now_test = Utc::now() + Duration::from_secs(2 * 3600);
            let dif = time - now_test;
            println!("Difference: {dif:?}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_candles() -> anyhow::Result<()> {
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        // time: 1733040000, offset: 540000, period: 3600
        let client = PocketOption::new(ssid).await.unwrap();
        let mut last_candles = Vec::new();
        for i in 0..10 {
            let candles = client.get_candles("EURUSD_otc", 60, 6000).await?;
            last_candles = candles.clone();
            println!("Candles n°{} len: {}, ", i + 1, candles.len());
        }
        println!("Candles: {last_candles:#?}");
        Ok(())
    }

    #[tokio::test]
    async fn test_history() -> anyhow::Result<()> {
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        // time: 1733040000, offset: 540000, period: 3600
        let client = PocketOption::new(ssid).await.unwrap();
        for i in 0..1000 {
            let candles = client.history("EURUSD_otc", 6000).await?;
            println!("Candles n°{} len: {}, ", i + 1, candles.len());
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_instances_same_ssid() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::INFO)?;
        let ssid = r#"42["auth",{"session":"t0mc6nefcv7ncr21g4fmtioidb","isDemo":1,"uid":90000798,"platform":2}]	"#;
        let client1 = PocketOption::new(ssid).await?;
        let client2 = PocketOption::new(ssid).await?;

        let stream1 = client1.subscribe_symbol("EURUSD_otc").await?;
        let stream2 = client2.subscribe_symbol("EURUSD_otc").await?;
        let fut1 = to_future(stream1, 1);
        let fut2 = to_future(stream2, 2);
        let _ = try_join(fut1, fut2).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_instances_same_real_ssid() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::INFO)?;
        let ssid = r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"02ac5a5875a4b583042aae064351e0bb\";s:10:\"ip_address\";s:13:\"191.113.133.5\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 OPR/116.\";s:13:\"last_activity\";i:1740838529;}0850af6716a7a1d1eea59e80987f4a59","isDemo":0,"uid":87742848,"platform":2}]	"#;
        let client1 = PocketOption::new(ssid).await?;
        let client2 = PocketOption::new(ssid).await?;

        let stream1 = client1.subscribe_symbol("EURUSD_otc").await?;
        let stream2 = client2.subscribe_symbol("EURUSD_otc").await?;
        let fut1 = to_future(stream1, 1);
        let fut2 = to_future(stream2, 2);
        let _ = try_join(fut1, fut2).await?;
        Ok(())
    }

    fn raw_validator() -> impl Fn(&RawWebsocketMessage) -> bool + Send + Sync {
        move |msg| {
            let msg = msg.to_string();
            msg.starts_with(r#"451-["signals/load""#)
        }
    }

    fn raw_iterator() -> impl Fn(&RawWebsocketMessage) -> bool + Send + Sync {
        move |msg| {
            let msg = msg.to_string();
            msg.starts_with(r#"{"signals":"#)
        }
    }
    #[tokio::test]
    async fn test_send_raw_message() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::INFO)?;
        let ssid = r#"42["auth",{"session":"mj194bjgehatidr1ml82453ajg","isDemo":1,"uid":87888871,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await?;
        sleep(Duration::from_secs(5)).await;
        fn _my_validator(msg: &RawWebsocketMessage) -> bool {
            msg.to_string().contains("success")
        }
        let res = client
            .create_raw_order(r#"42["signals/subscribe"]"#, Box::new(raw_validator()))
            .await?;
        info!("{res}");
        Ok(())
    }

    #[tokio::test]
    async fn test_send_raw_iterator() -> anyhow::Result<()> {
        start_tracing_leveled(true, LevelFilter::INFO)?;
        let ssid = r#"42["auth",{"session":"mj194bjgehatidr1ml82453ajg","isDemo":1,"uid":87888871,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await?;
        sleep(Duration::from_secs(5)).await;
        let res = client
            .create_raw_iterator(r#"42["signals/subscribe"]"#, Box::new(raw_iterator()), None)
            .await?;
        let mut stream = res.to_stream();
        while let Some(Ok(e)) = stream.next().await {
            info!(target: "RecievedStreamItem", "{}", e);
        }
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn test_send_raw_timeout_iterator() {
        start_tracing_leveled(true, LevelFilter::INFO).unwrap();
        let ssid = r#"42["auth",{"session":"mj194bjgehatidr1ml82453ajg","isDemo":1,"uid":87888871,"platform":2}]	"#;
        let client = PocketOption::new(ssid).await.unwrap();
        sleep(Duration::from_secs(5)).await;
        let res = client
            .create_raw_iterator(
                r#"42["signals/subscribe"]"#,
                Box::new(raw_iterator()),
                Some(Duration::from_secs(1)),
            )
            .await
            .unwrap();
        let mut stream = res.to_stream();
        while let Some(e) = stream.next().await {
            match e {
                Ok(e) => info!(target: "RecievedStreamItem", "{}", e),
                Err(e) => panic!("Error, {e}"),
            }
        }
    }
}
