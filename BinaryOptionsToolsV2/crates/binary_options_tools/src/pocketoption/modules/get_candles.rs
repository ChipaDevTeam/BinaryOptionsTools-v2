use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::{Deserialize, Serialize};
use tokio::select;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    error::BinaryOptionsError,
    pocketoption::{
        candle::Candle,
        error::{PocketError, PocketResult},
        state::State,
        types::MultiPatternRule,
        utils::get_index,
    },
};

const LOAD_HISTORY_PERIOD_PATTERNS: [&str; 2] = [
    r#"451-["loadHistoryPeriodFast","#,
    r#"451-["loadHistoryPeriod","#,
];

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

#[derive(Debug, Deserialize, Clone)]
pub struct CandleData {
    pub symbol_id: u32,
    pub time: i64,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoadHistoryPeriodResult {
    pub asset: String,
    pub index: u64,
    pub data: Vec<CandleData>,
    pub period: i64,
}

impl TryFrom<CandleData> for Candle {
    type Error = BinaryOptionsError;

    fn try_from(candle_data: CandleData) -> Result<Self, Self::Error> {
        Ok(Candle {
            symbol: String::new(), // Will be filled by the caller
            timestamp: candle_data.time as f64,
            open: Decimal::from_f64(candle_data.open).ok_or(BinaryOptionsError::General(
                "Couldn't parse f64 to Decimal".to_string(),
            ))?,
            high: Decimal::from_f64(candle_data.high).ok_or(BinaryOptionsError::General(
                "Couldn't parse f64 to Decimal".to_string(),
            ))?,
            low: Decimal::from_f64(candle_data.low).ok_or(BinaryOptionsError::General(
                "Couldn't parse f64 to Decimal".to_string(),
            ))?,
            close: Decimal::from_f64(candle_data.close).ok_or(BinaryOptionsError::General(
                "Couldn't parse f64 to Decimal".to_string(),
            ))?,
            volume: None,
        })
    }
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
}

#[derive(Debug)]
pub enum CommandResponse {
    CandlesResult { req_id: Uuid, candles: Vec<Candle> },
    Error { req_id: Uuid, error: String },
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
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
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
    pending_requests: std::collections::HashMap<u64, (Uuid, String)>, // index -> (req_id, asset)
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
                    if let Message::Binary(data) = msg.as_ref() {
                        match serde_json::from_slice::<LoadHistoryPeriodResult>(data) {
                            Ok(result) => {
                                // Find the pending request by index
                                if let Some((req_id, asset)) = self.pending_requests.remove(&result.index) {
                                    let candles: Vec<Candle> = result.data
                                        .into_iter()
                                        .map(|candle_data| {
                                            Candle::try_from(candle_data).map_err(|e| CoreError::Other(e.to_string())).map(|mut c| {c.symbol = asset.clone();
                                            c})
                                        })
                                        .collect::<Result<Vec<Candle>, _>>()?;

                                    // Send the response
                                    if let Err(e) = self.command_responder.send(CommandResponse::CandlesResult {
                                        req_id,
                                        candles,
                                    }).await {
                                        warn!("Failed to send candles result: {}", e);
                                    }
                                } else {
                                    warn!("Received candles for unknown request index: {}", result.index);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse LoadHistoryPeriodResult: {}", e);
                            }
                        }
                    }
                }
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::GetCandles { asset, period, time, offset, req_id } => {
                            match LoadHistoryPeriod::new(&asset, time, period, offset) {
                                Ok(load_history) => {
                                    // Store the request mapping
                                    self.pending_requests.insert(load_history.index, (req_id, asset));

                                    // Send the WebSocket message
                                    let message = Message::text(load_history.to_string());
                                    if let Err(e) = self.ws_sender.send(message).await {
                                        // Remove the pending request on error
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
