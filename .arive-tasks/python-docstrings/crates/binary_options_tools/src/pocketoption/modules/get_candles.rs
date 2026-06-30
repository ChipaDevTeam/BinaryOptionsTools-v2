use std::sync::Arc;
use std::collections::HashMap;

use async_trait::async_trait;
use binary_options_tools_core::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule, RunnerCommand},
};
use serde::{Deserialize, Serialize};
use tokio::select;
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption::{
    candle::{Candle, compile_candles_from_ticks, HistoryItem},
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
    utils::{get_index, normalize_timestamp, SocketIoFrame},
};

const LOAD_HISTORY_PERIOD_PATTERNS: [&str; 2] = ["loadHistoryPeriodFast", "loadHistoryPeriod"];

/// Default number of ticks/candles to fetch per pagination page.
const DEFAULT_PAGE_OFFSET: i64 = 1000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadHistoryPeriod {
    pub asset: String,
    pub period: i64,
    pub time: i64,
    pub index: u64,
    pub offset: i64,
    #[serde(skip)]
    pub is_fast: bool,
}

impl LoadHistoryPeriod {
    pub fn new(asset: impl ToString, time: i64, period: i64, offset: i64) -> PocketResult<Self> {
        Ok(LoadHistoryPeriod {
            asset: asset.to_string(),
            period,
            time,
            index: get_index()?,
            offset,
            is_fast: false,
        })
    }

    pub fn new_fast(asset: impl ToString, time: i64, period: i64, offset: i64) -> PocketResult<Self> {
        Ok(LoadHistoryPeriod {
            asset: asset.to_string(),
            period,
            time,
            index: get_index()?,
            offset,
            is_fast: true,
        })
    }
}

impl std::fmt::Display for LoadHistoryPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = serde_json::to_string(&self).map_err(|_| std::fmt::Error)?;
        let event = if self.is_fast { "loadHistoryPeriodFast" } else { "loadHistoryPeriod" };
        write!(f, "42[\"{event}\",{data}]")
    }
}

/// Represents a single tick/trade data point from loadHistoryPeriod.
/// Supports two formats:
/// 1. Tick format: { "asset": "...", "time": timestamp, "price": value }
/// 2. Candle format: { "symbol_id": 123, "time": timestamp, "open": value, "close": value, "high": value, "low": value, "volume": value }
#[derive(Debug, Deserialize, Clone)]
pub struct TickData {
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub symbol_id: Option<u64>,
    pub time: f64,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub open: Option<f64>,
    #[serde(default)]
    pub close: Option<f64>,
    #[serde(default)]
    pub high: Option<f64>,
    #[serde(default)]
    pub low: Option<f64>,
    #[serde(default)]
    pub volume: Option<f64>,
}

impl TickData {
    /// Get the price for tick data (uses close price if available, otherwise price field)
    pub fn get_price(&self) -> f64 {
        self.close.or(self.price).unwrap_or(0.0)
    }

    /// Get the asset name
    pub fn get_asset(&self) -> String {
        self.asset.clone().unwrap_or_default()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadHistoryPeriodResult {
    #[serde(default)]
    pub asset: String,
    pub index: u64,
    #[serde(default)]
    pub data: Vec<TickData>,
    #[serde(default)]
    pub period: i64,
}

/// The type of request being made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestKind {
    Candles,
    Ticks,
}

#[derive(Debug)]
pub enum Command {
    GetCandles {
        asset: String,
        period: i64,
        time: i64,
        offset: i64,
        req_id: Uuid,
    },
    GetTicks {
        asset: String,
        period: i64,
        time: i64,
        offset: i64,
        req_id: Uuid,
    },
}

#[derive(Debug)]
pub enum CommandResponse {
    CandlesResult {
        req_id: Uuid,
        candles: Vec<Candle>,
    },
    TicksResult {
        req_id: Uuid,
        ticks: Vec<(i64, f64)>,
    },
    Error {
        req_id: Uuid,
        error: String,
    },
    /// The module has stopped and cannot fulfill the request.
    Shutdown {
        req_id: Uuid,
    },
}

#[derive(Clone)]
pub struct GetCandlesHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
}

impl GetCandlesHandle {
    /// Gets historical candle data for a specific asset.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD_otc")
    /// * `period` - Time period for each candle in seconds
    /// * `offset` - Number of periods to offset from current time
    ///
    /// # Returns
    /// A vector of Candle objects containing historical price data
    pub async fn get_candles(
        &self,
        asset: impl ToString,
        period: i64,
        offset: i64,
    ) -> PocketResult<Vec<Candle>> {
        let current_time = chrono::Utc::now().timestamp();
        self.get_candles_advanced(asset, period, current_time, offset)
            .await
    }

    /// Gets historical candle data with advanced parameters.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD_otc")
    /// * `period` - Time period for each candle in seconds
    /// * `time` - Current time timestamp
    /// * `offset` - Number of periods to offset from current time
    ///
    /// # Returns
    /// A vector of Candle objects containing historical price data
    pub async fn get_candles_advanced(
        &self,
        asset: impl ToString,
        period: i64,
        time: i64,
        offset: i64,
    ) -> PocketResult<Vec<Candle>> {
        info!(target: "GetCandlesHandle", "Requesting candles for asset: {}, period: {}, time: {}, offset: {}", asset.to_string(), period, time, offset);
        let req_id = Uuid::new_v4();

        self.sender
            .send(Command::GetCandles {
                asset: asset.to_string(),
                period,
                time,
                offset,
                req_id,
            })
            .await
            .map_err(CoreError::from)?;

        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::CandlesResult {
                    req_id: response_id,
                    candles,
                }) => {
                    if req_id == response_id {
                        return Ok(candles);
                    }
                    // Continue waiting for the correct response
                }
                Ok(CommandResponse::Error {
                    req_id: response_id,
                    error,
                }) => {
                    if req_id == response_id {
                        return Err(PocketError::General(error));
                    }
                    // Continue waiting for the correct response
                }
                Ok(CommandResponse::Shutdown { req_id: response_id }) => {
                    if req_id == response_id {
                        return Err(PocketError::ModuleStopped {
                            module_name: "GetCandlesApiModule".to_string(),
                            context: "GetCandlesApiModule stopped during request".to_string(),
                        });
                    }
                }
                Ok(_) => continue, // Ignore other response types
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }

    /// Gets historical tick data (timestamp, price) for a specific asset with pagination.
    ///
    /// This method uses `loadHistoryPeriod` with pagination to fetch tick data going back
    /// as far as needed, overcoming the limited window returned by `changeSymbol`.
    ///
    /// # Arguments
    /// * `asset` - Trading symbol (e.g., "EURUSD_otc")
    /// * `period` - Time period in seconds (used as context for the server)
    /// * `lookback_seconds` - How many seconds of tick history to fetch
    ///
    /// # Returns
    /// A vector of (timestamp, price) tuples sorted by timestamp
    pub async fn get_ticks(
        &self,
        asset: impl ToString,
        period: i64,
        lookback_seconds: i64,
    ) -> PocketResult<Vec<(i64, f64)>> {
        let asset_str = asset.to_string();
        let now = chrono::Utc::now().timestamp();
        let target_time = now - lookback_seconds;
        let page_offset: i64 = DEFAULT_PAGE_OFFSET; // Fetch ticks per page

        let mut all_ticks: Vec<(i64, f64)> = Vec::new();
        let mut current_time = now;
        let mut max_pages = 20; // Safety limit to prevent infinite loops

        loop {
            let req_id = Uuid::new_v4();
            // Use loadHistoryPeriodFast for small offsets if needed, but here we use the module's logic
            info!(target: "GetCandlesHandle", "Requesting ticks for asset: {}, period: {}, time: {}, offset: {}", asset_str, period, current_time, page_offset);

            self.sender
                .send(Command::GetTicks {
                    asset: asset_str.clone(),
                    period,
                    time: current_time,
                    offset: page_offset,
                    req_id,
                })
                .await
                .map_err(CoreError::from)?;

            // Wait for the response
            let ticks = loop {
                match self.receiver.recv().await {
                    Ok(CommandResponse::TicksResult {
                        req_id: response_id,
                        ticks,
                    }) => {
                        if req_id == response_id {
                            break ticks;
                        }
                    }
                    Ok(CommandResponse::Error {
                        req_id: response_id,
                        error,
                    }) => {
                        if req_id == response_id {
                            return Err(PocketError::General(error));
                        }
                    }
                    Ok(CommandResponse::Shutdown { req_id: response_id }) => {
                        if req_id == response_id {
                            return Err(PocketError::ModuleStopped {
                                module_name: "GetCandlesApiModule".to_string(),
                                context: "GetCandlesApiModule stopped during request".to_string(),
                            });
                        }
                    }
                    Ok(_) => continue,
                    Err(e) => return Err(CoreError::from(e).into()),
                }
            };

            if ticks.is_empty() {
                break; // No more data
            }

            let earliest_tick_time = ticks.first().map(|(t, _)| *t).unwrap_or(current_time);

            // Add ticks that are within our lookback window
            for (ts, price) in &ticks {
                if *ts >= target_time {
                    all_ticks.push((*ts, *price));
                }
            }

            // Check if we've covered the lookback period
            if earliest_tick_time <= target_time {
                break;
            }

            // Move to the next page
            current_time = earliest_tick_time;
            max_pages -= 1;
            if max_pages <= 0 {
                warn!(target: "GetCandlesHandle", "Reached max pagination pages for {}", asset_str);
                break;
            }
        }

        // Sort by timestamp and deduplicate
        all_ticks.sort_by(|a, b| a.0.cmp(&b.0));
        all_ticks.dedup_by(|a, b| a.0 == b.0);

        info!(target: "GetCandlesHandle", "Collected {} ticks for {} covering {} seconds", all_ticks.len(), asset_str, lookback_seconds);
        Ok(all_ticks)
    }
}

/// API module for handling candle data requests.
pub struct GetCandlesApiModule {
    #[allow(dead_code)]
    state: Arc<State>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    ws_sender: AsyncSender<Message>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    pending_requests: HashMap<u64, (Uuid, String, RequestKind, u32)>, // index -> (req_id, asset, kind, period)
    latest_ticks: HashMap<String, Vec<(i64, f64)>>,
}

#[async_trait]
impl ApiModule<State> for GetCandlesApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = GetCandlesHandle;

    fn new(
        state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        ws_receiver: AsyncReceiver<Arc<Message>>,
        ws_sender: AsyncSender<Message>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self {
            state,
            ws_receiver,
            ws_sender,
            command_receiver,
            command_responder,
            pending_requests: HashMap::new(),
            latest_ticks: HashMap::new(),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        GetCandlesHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                msg_res = self.ws_receiver.recv() => {
                    match msg_res {
                        Ok(msg) => {
                            match msg.as_ref() {
                                Message::Binary(data) => {
                                    if let Ok(result) = serde_json::from_slice::<LoadHistoryPeriodResult>(data) {
                                        if let Err(e) = self.process_result(result).await {
                                            warn!(target: "GetCandlesApiModule", "Error processing binary result: {}", e);
                                        }
                                    } else {
                                        // Try parsing as updateStream tick data
                                        if let Ok(text) = std::str::from_utf8(data) {
                                            if let Some((symbol, timestamp, price)) = self.parse_update_stream(text) {
                                                self.latest_ticks.entry(symbol).or_default().push((timestamp, price));
                                            }
                                        }
                                    }
                                }
                                Message::Text(text) => {
                                    if let Ok(result) = serde_json::from_str::<LoadHistoryPeriodResult>(text) {
                                        if let Err(e) = self.process_result(result).await {
                                            warn!(target: "GetCandlesApiModule", "Error processing text result: {}", e);
                                        }
                                    } else if let Some(frame) = SocketIoFrame::parse(text) {
                                        let event_payload: Option<(String, serde_json::Value)> = frame.extract_event();
                                        if let Some((event_name, payload)) = event_payload {
                                            if event_name == "loadHistoryPeriod" || event_name == "loadHistoryPeriodFast" {
                                                match serde_json::from_value::<LoadHistoryPeriodResult>(payload) {
                                                    Ok(result) => {
                                                        if let Err(e) = self.process_result(result).await {
                                                            warn!(target: "GetCandlesApiModule", "Error processing event result: {}", e);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!("Failed to deserialize LoadHistoryPeriodResult from Socket.IO frame (event: {}): {}", event_name, e);
                                                    }
                                                }
                                            }
                                        }
                                    } else if let Some((symbol, timestamp, price)) = self.parse_update_stream(text) {
                                        self.latest_ticks.entry(symbol).or_default().push((timestamp, price));
                                    }
                                }
                                _ => {
                                    warn!(target: "GetCandlesApiModule", "Received unexpected message type: {:?}", msg);
                                }
                            }
                        }
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            break;
                        }
                    }
                }
                cmd_res = self.command_receiver.recv() => {
                    match cmd_res {
                        Ok(cmd) => {
                            match cmd {
                                Command::GetCandles { asset, period, time, offset, req_id } => {
                                    let load_history_res = if offset <= 100 {
                                        LoadHistoryPeriod::new_fast(&asset, time, period, offset)
                                    } else {
                                        LoadHistoryPeriod::new(&asset, time, period, offset)
                                    };

                                    match load_history_res {
                                        Ok(load_history) => {
                                            // Clear buffered ticks for this asset to ensure we get fresh ones after the historical request
                                            self.latest_ticks.remove(&asset);
                                            
                                            // Store the request mapping
                                            self.pending_requests.insert(load_history.index, (req_id, asset, RequestKind::Candles, period as u32));

                                            // Send the WebSocket message
                                            let message = Message::text(load_history.to_string());
                                            if let Err(e) = self.ws_sender.send(message).await {
                                                self.pending_requests.remove(&load_history.index);

                                                if let Err(resp_err) = self.command_responder.send(CommandResponse::Error {
                                                    req_id,
                                                    error: format!("Failed to send WebSocket message: {e}"),
                                                }).await {
                                                    warn!("Failed to send error response: {}", resp_err);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(resp_err) = self.command_responder.send(CommandResponse::Error {
                                                req_id,
                                                error: format!("Failed to create LoadHistoryPeriod: {e}"),
                                            }).await {
                                                warn!("Failed to send error response: {}", resp_err);
                                            }
                                        }
                                    }
                                }
                                Command::GetTicks { asset, period, time, offset, req_id } => {
                                    let load_history_res = if offset <= 100 {
                                        LoadHistoryPeriod::new_fast(&asset, time, period, offset)
                                    } else {
                                        LoadHistoryPeriod::new(&asset, time, period, offset)
                                    };

                                    match load_history_res {
                                        Ok(load_history) => {
                                            self.latest_ticks.remove(&asset);
                                            
                                            // Store the request mapping
                                            self.pending_requests.insert(load_history.index, (req_id, asset, RequestKind::Ticks, period as u32));

                                            // Send the WebSocket message
                                            let message = Message::text(load_history.to_string());
                                            if let Err(e) = self.ws_sender.send(message).await {
                                                self.pending_requests.remove(&load_history.index);

                                                if let Err(resp_err) = self.command_responder.send(CommandResponse::Error {
                                                    req_id,
                                                    error: format!("Failed to send WebSocket message: {e}"),
                                                }).await {
                                                    warn!("Failed to send error response: {}", resp_err);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            if let Err(resp_err) = self.command_responder.send(CommandResponse::Error {
                                                req_id,
                                                error: format!("Failed to create LoadHistoryPeriod: {e}"),
                                            }).await {
                                                warn!("Failed to send error response: {}", resp_err);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(Vec::from(
            LOAD_HISTORY_PERIOD_PATTERNS,
        )))
    }
}

impl GetCandlesApiModule {
    /// Notifies all pending waiters that the module has stopped.
    async fn notify_waiters_module_stopped(&mut self) {
        let waiters = std::mem::take(&mut self.pending_requests);
        for (_, (req_id, _, _, _)) in waiters {
            let _ = self
                .command_responder
                .send(CommandResponse::Shutdown { req_id })
                .await;
        }
    }
    /// Parses an updateStream message into (symbol, timestamp, price).
    fn parse_update_stream(&self, text: &str) -> Option<(String, i64, f64)> {
        // Handle Socket.IO array format: [["symbol", timestamp, price]]
        if let Ok(serde_json::Value::Array(outer_arr)) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(inner_arr) = outer_arr.first().and_then(|v| v.as_array()) {
                if inner_arr.len() >= 3 {
                    let symbol = inner_arr[0].as_str()?.to_string();
                    let timestamp = normalize_timestamp(inner_arr[1].as_f64()?);
                    let price = inner_arr[2].as_f64()?;
                    return Some((symbol, timestamp, price));
                }
            }
        }
        None
    }

    async fn process_result(&mut self, result: LoadHistoryPeriodResult) -> CoreResult<()> {
        // Find the pending request by index
        if let Some((req_id, asset, request_kind, requested_period)) = self.pending_requests.remove(&result.index) {
            match request_kind {
                RequestKind::Candles => {
                    // Check if the data is already OHLC candles
                    let has_ohlc = result.data.iter().any(|d| {
                        d.open.is_some() && d.high.is_some() && d.low.is_some() && d.close.is_some()
                    });

                    let mut history_items: Vec<HistoryItem> = if has_ohlc {
                        result
                            .data
                            .into_iter()
                            .map(|tick_data| {
                                let timestamp = normalize_timestamp(tick_data.time);
                                if let (Some(open), Some(high), Some(low), Some(close)) = (
                                    tick_data.open,
                                    tick_data.high,
                                    tick_data.low,
                                    tick_data.close,
                                ) {
                                    HistoryItem::Candle(crate::pocketoption::candle::CandleItem {
                                        timestamp,
                                        open,
                                        high,
                                        low,
                                        close,
                                        volume: tick_data.volume.unwrap_or(0.0),
                                    })
                                } else {
                                    let price = tick_data.get_price();
                                    HistoryItem::Tick([
                                        serde_json::Value::from(tick_data.time),
                                        serde_json::Value::from(price),
                                    ])
                                }
                            })
                            .collect()
                    } else {
                        result
                            .data
                            .into_iter()
                            .map(|td| {
                                HistoryItem::Tick([
                                    serde_json::Value::from(td.time),
                                    serde_json::Value::from(td.get_price()),
                                ])
                            })
                            .collect()
                    };

                    // Append buffered ticks from updateStream if they are newer
                    if let Some(stream_ticks) = self.latest_ticks.remove(&asset) {
                        let last_ts = history_items.last().map(|i| i.to_tick().0).unwrap_or(0);
                        
                        for (ts, price) in stream_ticks {
                            if ts > last_ts {
                                history_items.push(HistoryItem::Tick([
                                    serde_json::Value::from(ts as f64),
                                    serde_json::Value::from(price),
                                ]));
                            }
                        }
                    }

                    let candles = compile_candles_from_ticks(&history_items, requested_period, &asset);

                    if let Err(e) = self
                        .command_responder
                        .send(CommandResponse::CandlesResult { req_id, candles })
                        .await
                    {
                        warn!("Failed to send candles result: {}", e);
                    }
                }
                RequestKind::Ticks => {
                    let mut ticks: Vec<(i64, f64)> = result
                        .data
                        .into_iter()
                        .map(|tick_data| {
                            let timestamp = normalize_timestamp(tick_data.time);
                            (timestamp, tick_data.get_price())
                        })
                        .collect();

                    // Append buffered ticks from updateStream
                    if let Some(stream_ticks) = self.latest_ticks.remove(&asset) {
                        let last_ts = ticks.last().map(|(t, _)| *t).unwrap_or(0);
                        for (ts, price) in stream_ticks {
                            if ts > last_ts {
                                ticks.push((ts, price));
                            }
                        }
                    }

                    if let Err(e) = self
                        .command_responder
                        .send(CommandResponse::TicksResult { req_id, ticks })
                        .await
                    {
                        warn!("Failed to send ticks result: {}", e);
                    }
                }
            }
        } else {
            warn!("Received data for unknown request index: {}", result.index);
        }
        Ok(())
    }
}
