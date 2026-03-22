use std::sync::Arc;

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
    candle::{BaseCandle, Candle},
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
    utils::get_index,
};

const LOAD_HISTORY_PERIOD_PATTERNS: [&str; 2] = ["loadHistoryPeriodFast", "loadHistoryPeriod"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadHistoryPeriod {
    pub asset: String,
    pub period: i64,
    pub time: i64,
    pub index: u64,
    pub offset: i64,
}

impl LoadHistoryPeriod {
    pub fn new(asset: impl ToString, time: i64, period: i64, offset: i64) -> PocketResult<Self> {
        Ok(LoadHistoryPeriod {
            asset: asset.to_string(),
            period,
            time,
            index: get_index()?,
            offset,
        })
    }
}

impl std::fmt::Display for LoadHistoryPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = serde_json::to_string(&self).map_err(|_| std::fmt::Error)?;
        write!(f, "42[\"loadHistoryPeriod\",{data}]")
    }
}

/// Represents a single tick/trade data point from loadHistoryPeriod.
/// Format: { "asset": "...", "time": timestamp, "price": value }
#[derive(Debug, Deserialize, Clone)]
pub struct TickData {
    pub asset: String,
    pub time: f64,
    pub price: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadHistoryPeriodResult {
    pub asset: String,
    pub index: u64,
    #[serde(default)]
    pub data: Vec<TickData>,
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
        let page_offset: i64 = 1000; // Fetch 1000 ticks per page

        let mut all_ticks: Vec<(i64, f64)> = Vec::new();
        let mut current_time = now;
        let mut max_pages = 20; // Safety limit to prevent infinite loops

        loop {
            let req_id = Uuid::new_v4();
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
    pending_requests: std::collections::HashMap<u64, (Uuid, String, RequestKind)>, // index -> (req_id, asset, kind)
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
            pending_requests: std::collections::HashMap::new(),
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
                Ok(msg) = self.ws_receiver.recv() => {
                    match msg.as_ref() {
                        Message::Binary(data) => {
                            if let Ok(result) = serde_json::from_slice::<LoadHistoryPeriodResult>(data) {
                                self.process_result(result).await?;
                            } else {
                                warn!("Failed to parse LoadHistoryPeriodResult (binary)");
                            }
                        }
                        Message::Text(text) => {
                            if let Ok(result) = serde_json::from_str::<LoadHistoryPeriodResult>(text) {
                                self.process_result(result).await?;
                            } else if let Some(start) = text.find('[') {
                                // Try parsing as a 1-step Socket.IO message: 42["loadHistoryPeriod", {...}]
                                if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&text[start..]) {
                                    if arr.len() >= 2 && (arr[0] == "loadHistoryPeriod" || arr[0] == "loadHistoryPeriodFast") {
                                        if let Ok(result) = serde_json::from_value::<LoadHistoryPeriodResult>(arr[1].clone()) {
                                            self.process_result(result).await?;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::GetCandles { asset, period, time, offset, req_id } => {
                            match LoadHistoryPeriod::new(&asset, time, period, offset) {
                                Ok(load_history) => {
                                    // Store the request mapping
                                    self.pending_requests.insert(load_history.index, (req_id, asset, RequestKind::Candles));

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
                            match LoadHistoryPeriod::new(&asset, time, period, offset) {
                                Ok(load_history) => {
                                    // Store the request mapping
                                    self.pending_requests.insert(load_history.index, (req_id, asset, RequestKind::Ticks));

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
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(Vec::from(
            LOAD_HISTORY_PERIOD_PATTERNS,
        )))
    }
}

impl GetCandlesApiModule {
    async fn process_result(&mut self, result: LoadHistoryPeriodResult) -> CoreResult<()> {
        // Find the pending request by index
        if let Some((req_id, asset, request_kind)) = self.pending_requests.remove(&result.index) {
            match request_kind {
                RequestKind::Candles => {
                    let candles: Vec<Candle> = result
                        .data
                        .into_iter()
                        .filter_map(|tick_data| {
                            // Convert tick data to a single-price candle
                            // This maintains backwards compatibility when the server returns
                            // tick data instead of OHLC candles
                            let base_candle = BaseCandle {
                                timestamp: tick_data.time as i64,
                                open: tick_data.price,
                                high: tick_data.price,
                                low: tick_data.price,
                                close: tick_data.price,
                                volume: None,
                            };
                            let symbol = asset.clone();
                            Candle::try_from((base_candle, symbol)).ok()
                        })
                        .collect();

                    if let Err(e) = self
                        .command_responder
                        .send(CommandResponse::CandlesResult { req_id, candles })
                        .await
                    {
                        warn!("Failed to send candles result: {}", e);
                    }
                }
                RequestKind::Ticks => {
                    let ticks: Vec<(i64, f64)> = result
                        .data
                        .into_iter()
                        .map(|tick_data| (tick_data.time as i64, tick_data.price))
                        .collect();

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
