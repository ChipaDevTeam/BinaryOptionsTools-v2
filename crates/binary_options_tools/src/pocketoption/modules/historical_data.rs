use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use binary_options_tools_core::error::{CoreError, CoreResult};
use binary_options_tools_core::reimports::{AsyncReceiver, AsyncSender, Message};
use binary_options_tools_core::traits::{ApiModule, Rule, RunnerCommand};
use rust_decimal::prelude::ToPrimitive;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::{select, time::timeout};
use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::candle::{
    compile_candles_from_ticks, BaseCandle, Candle, CandleItem, HistoryItem,
};
use crate::pocketoption::error::{PocketError, PocketResult};
use crate::pocketoption::state::State;
use crate::pocketoption::types::{MultiPatternRule};
use crate::pocketoption::utils::normalize_timestamp;

const HISTORICAL_DATA_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MISMATCH_RETRIES: usize = 5;

#[derive(Debug, PartialEq, Eq)]
enum RequestType {
    Ticks,
    Candles,
}

#[derive(Debug)]
pub enum Command {
    GetTicks {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
    GetCandles {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum CommandResponse {
    Ticks {
        req_id: Uuid,
        ticks: Vec<(i64, f64)>,
    },
    Candles {
        req_id: Uuid,
        candles: Vec<Candle>,
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

#[derive(Deserialize)]
pub struct HistoryResponse {
    pub asset: String,
    pub period: u32,
    #[serde(default)]
    pub history: Option<Vec<HistoryItem>>,
    #[serde(default)]
    pub candles: Option<Vec<CandleItem>>,
    // Separate arrays for OHLC data (legacy format)
    #[serde(default)]
    pub o: Option<Vec<f64>>,
    #[serde(default)]
    pub h: Option<Vec<f64>>,
    #[serde(default)]
    pub l: Option<Vec<f64>>,
    #[serde(default)]
    pub c: Option<Vec<f64>>,
    #[serde(alias = "t", default)]
    pub timestamps: Option<Vec<f64>>,
    #[serde(default)]
    pub v: Option<Vec<f64>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ServerResponse {
    Success(Vec<Candle>),
    History(HistoryResponse),
    Fail(String),
}

#[derive(Debug, Clone)]
pub struct HistoricalDataHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
    call_lock: Arc<Mutex<()>>,
}

impl HistoricalDataHandle {
    /// Retrieves historical tick data (timestamp, price) for a specific asset and period.
    pub async fn ticks(&self, asset: String, period: u32) -> PocketResult<Vec<(i64, f64)>> {
        let _guard = self.call_lock.lock().await;

        let id = Uuid::new_v4();
        self.sender
            .send(Command::GetTicks {
                asset: asset.clone(),
                period,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        let mut mismatch_count = 0;
        loop {
            match timeout(HISTORICAL_DATA_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Ticks { req_id, ticks })) => {
                    if req_id == id {
                        return Ok(ticks);
                    } else {
                        warn!("Received response for unknown req_id: {}", req_id);
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: "ticks".to_string(),
                                context: format!(
                                    "asset: {}, period: {}, exceeded mismatch retries",
                                    asset, period
                                ),
                                duration: HISTORICAL_DATA_TIMEOUT,
                            });
                        }
                        continue;
                    }
                }
                Ok(Ok(CommandResponse::Candles { .. })) => continue,
                Ok(Ok(CommandResponse::Error { req_id, error })) => {
                    if req_id == id {
                        return Err(PocketError::General(error));
                    }
                    continue;
                }
                Ok(Ok(CommandResponse::Shutdown { req_id })) => {
                    if req_id == id {
                        return Err(PocketError::ModuleStopped {
                            module_name: "HistoricalDataApiModule".to_string(),
                            context: "HistoricalDataApiModule stopped during request".to_string(),
                        });
                    }
                    continue;
                }
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: "ticks".to_string(),
                        context: format!("asset: {}, period: {}", asset, period),
                        duration: HISTORICAL_DATA_TIMEOUT,
                    });
                }
            }
        }
    }

    /// Retrieves historical candle data for a specific asset and period.
    pub async fn candles(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let _guard = self.call_lock.lock().await;

        let id = Uuid::new_v4();
        self.sender
            .send(Command::GetCandles {
                asset: asset.clone(),
                period,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        let mut mismatch_count = 0;
        loop {
            match timeout(HISTORICAL_DATA_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Candles { req_id, candles })) => {
                    if req_id == id {
                        return Ok(candles);
                    } else {
                        warn!("Received response for unknown req_id: {}", req_id);
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: "candles".to_string(),
                                context: format!(
                                    "asset: {}, period: {}, exceeded mismatch retries",
                                    asset, period
                                ),
                                duration: HISTORICAL_DATA_TIMEOUT,
                            });
                        }
                        continue;
                    }
                }
                Ok(Ok(CommandResponse::Ticks { .. })) => continue,
                Ok(Ok(CommandResponse::Error { req_id, error })) => {
                    if req_id == id {
                        return Err(PocketError::General(error));
                    }
                    continue;
                }
                Ok(Ok(CommandResponse::Shutdown { req_id })) => {
                    if req_id == id {
                        return Err(PocketError::ModuleStopped {
                            module_name: "HistoricalDataApiModule".to_string(),
                            context: "HistoricalDataApiModule stopped during request".to_string(),
                        });
                    }
                    continue;
                }
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: "candles".to_string(),
                        context: format!("asset: {}, period: {}", asset, period),
                        duration: HISTORICAL_DATA_TIMEOUT,
                    });
                }
            }
        }
    }
}

pub struct HistoricalDataApiModule {
    _state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    pending_request: Option<(Uuid, String, u32, RequestType)>,
    latest_ticks: HashMap<String, Vec<(i64, f64)>>,
}

#[async_trait]
impl ApiModule<State> for HistoricalDataApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = HistoricalDataHandle;

    fn new(
        shared_state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self {
            _state: shared_state,
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
            pending_request: None,
            latest_ticks: HashMap::new(),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        HistoricalDataHandle {
            sender,
            receiver,
            call_lock: Arc::new(Mutex::new(())),
        }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                cmd_res = self.command_receiver.recv() => {
                    match cmd_res {
                        Ok(cmd) => {
                            match cmd {
                                Command::GetTicks { asset, period, req_id } => {
                                    if self.pending_request.is_some() {
                                        warn!(target: "HistoricalDataApiModule", "Overwriting a pending request. Concurrent calls are not supported.");
                                    }
                                    self.latest_ticks.remove(&asset);
                                    self.pending_request = Some((req_id, asset.clone(), period, RequestType::Ticks));
                                    let payload = serde_json::json!(["changeSymbol", { "asset": asset, "period": period }]);
                                    let msg = format!("42{}", serde_json::to_string(&payload)?);
                                    if let Err(e) = self.to_ws_sender.send(Message::text(msg)).await {
                                        warn!(target: "HistoricalDataApiModule", "Failed to send history request: {}", e);
                                        self.pending_request = None;
                                        let _ = self.command_responder.send(CommandResponse::Error { req_id, error: e.to_string() }).await;
                                    }
                                }
                                Command::GetCandles { asset, period, req_id } => {
                                    if self.pending_request.is_some() {
                                        warn!(target: "HistoricalDataApiModule", "Overwriting a pending request. Concurrent calls are not supported.");
                                    }
                                    self.latest_ticks.remove(&asset);
                                    self.pending_request = Some((req_id, asset.clone(), period, RequestType::Candles));
                                    let payload = serde_json::json!(["changeSymbol", { "asset": asset, "period": period }]);
                                    let msg = format!("42{}", serde_json::to_string(&payload)?);
                                    if let Err(e) = self.to_ws_sender.send(Message::text(msg)).await {
                                        warn!(target: "HistoricalDataApiModule", "Failed to send history request: {}", e);
                                        self.pending_request = None;
                                        let _ = self.command_responder.send(CommandResponse::Error { req_id, error: e.to_string() }).await;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            break;
                        }
                    }
                },
                msg_res = self.message_receiver.recv() => {
                    match msg_res {
                        Ok(msg) => {
                            let mut is_binary_placeholder = false;
                            let response = match &*msg {
                                Message::Binary(data) => match serde_json::from_slice::<ServerResponse>(data) {
                                    Ok(res) => Some(res),
                                    Err(e) => {
                                        warn!(target: "HistoricalDataApiModule", "Failed to parse binary ServerResponse: {}", e);
                                        None
                                    }
                                },
                                Message::Text(text) => {
                                    if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                                        Some(res)
                                    } else if let Some(start) = text.find('[') {
                                        match serde_json::from_str::<serde_json::Value>(&text[start..]) {
                                            Ok(serde_json::Value::Array(arr)) => {
                                                if arr.len() >= 2 && arr[0].as_str().map(|s| s.starts_with("updateHistory")).unwrap_or(false) {
                                                    if arr[1].as_object().is_some_and(|obj| obj.contains_key("_placeholder")) {
                                                        is_binary_placeholder = true;
                                                        None
                                                    } else {
                                                        match serde_json::from_value::<ServerResponse>(arr[1].clone()) {
                                                            Ok(res) => Some(res),
                                                            Err(e) => {
                                                                warn!(target: "HistoricalDataApiModule", "Failed to parse updateHistory payload: {}", e);
                                                                None
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    None
                                                }
                                            }
                                            Ok(_) => None,
                                            Err(e) => {
                                                warn!(target: "HistoricalDataApiModule", "Failed to parse JSON array from text: {}", e);
                                                None
                                            }
                                        }
                                    } else {
                                        None
                                    }
                                },
                                _ => {
                                    warn!(target: "HistoricalDataApiModule", "Received unexpected message type: {:?}", msg);
                                    None
                                },
                            };

                            if is_binary_placeholder { continue; }

                            if response.is_none() {
                                if let Message::Text(text) = &*msg {
                                    if let Some((symbol, timestamp, price)) = Self::parse_update_stream(text) {
                                        self.latest_ticks.entry(symbol).or_default().push((timestamp, price));
                                        continue;
                                    }
                                } else if let Message::Binary(data) = &*msg {
                                    if let Ok(text) = std::str::from_utf8(data) {
                                        if let Some((symbol, timestamp, price)) = Self::parse_update_stream(text) {
                                            self.latest_ticks.entry(symbol).or_default().push((timestamp, price));
                                            continue;
                                        }
                                    }
                                }
                            }

                            if let Some(response) = response {
                                match response {
                                    ServerResponse::Success(candles) => {
                                        if let Some((req_id, _, _, req_type)) = self.pending_request.take() {
                                            let resp = if req_type == RequestType::Ticks {
                                                CommandResponse::Ticks { req_id, ticks: candles.iter().map(|c| (c.timestamp, c.close.to_f64().unwrap_or_default())).collect() }
                                            } else {
                                                CommandResponse::Candles { req_id, candles }
                                            };
                                            let _ = self.command_responder.send(resp).await;
                                        }
                                    }
                                    ServerResponse::History(history_response) => {
                                        if let Some((req_id, asset, period, req_type)) = self.pending_request.take() {
                                            if history_response.asset != asset || history_response.period != period {
                                                self.pending_request = Some((req_id, asset, period, req_type));
                                                continue;
                                            }
                                            let symbol = history_response.asset;
                                            let mut ticks = history_response.history.as_ref().map(|h| h.iter().map(|item| item.to_tick()).collect()).unwrap_or_else(Vec::new);

                                            if req_type == RequestType::Ticks {
                                                if ticks.is_empty() {
                                                    if let Some(c_items) = history_response.candles {
                                                        ticks = c_items.iter().map(|i| (i.timestamp, i.close)).collect();
                                                    }
                                                }
                                                if let Some(stream_ticks) = self.latest_ticks.get(&symbol) {
                                                    let last_ts = ticks.last().map(|(t, _)| *t).unwrap_or(0);
                                                    for &(ts, price) in stream_ticks {
                                                        if ts > last_ts { ticks.push((ts, price)); }
                                                    }
                                                }
                                                let _ = self.command_responder.send(CommandResponse::Ticks { req_id, ticks }).await;
                                            } else {
                                                let mut candles = Vec::new();
                                                if let Some(c_items) = history_response.candles {
                                                    for item in c_items {
                                                        let bc = BaseCandle { timestamp: item.timestamp, open: item.open, close: item.close, high: item.high, low: item.low, volume: Some(item.volume) };
                                                        if let Ok(c) = Candle::try_from((bc, symbol.clone())) { candles.push(c); }
                                                    }
                                                }
                                                let mut h_items = history_response.history.unwrap_or_default();
                                                if let Some(s_ticks) = self.latest_ticks.get(&symbol) {
                                                    for &(ts, price) in s_ticks {
                                                        h_items.push(HistoryItem::Tick([serde_json::Value::from(ts as f64), serde_json::Value::from(price)]));
                                                    }
                                                }
                                                if !h_items.is_empty() {
                                                    let compiled = compile_candles_from_ticks(&h_items, history_response.period, &symbol);
                                                    let last_ts = candles.iter().map(|c| c.timestamp).max().unwrap_or(0);
                                                    for cc in compiled { if cc.timestamp > last_ts { candles.push(cc); } }
                                                }
                                                candles.sort_by_key(|c| c.timestamp);
                                                let _ = self.command_responder.send(CommandResponse::Candles { req_id, candles }).await;
                                            }
                                        }
                                    }
                                    ServerResponse::Fail(e) => {
                                        if let Some((req_id, _, _, _)) = self.pending_request.take() {
                                            let _ = self.command_responder.send(CommandResponse::Error { req_id, error: e }).await;
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
        Box::new(MultiPatternRule::new(vec!["updateHistory", "updateHistoryNewFast", "updateHistoryNew", "updateStream"]))
    }
}

impl HistoricalDataApiModule {
    fn parse_update_stream(text: &str) -> Option<(String, i64, f64)> {
        let start = text.find('[')?;
        let arr: serde_json::Value = serde_json::from_str(&text[start..]).ok()?;
        let outer = arr.as_array()?;
        let data = if let Some(first) = outer.first() {
            if first.is_string() && outer.len() >= 2 { outer.get(1)?.as_array()? } else { outer }
        } else { return None; };
        if let Some(inner) = data.first().and_then(|v| v.as_array()) {
            if inner.len() >= 3 {
                return Some((inner[0].as_str()?.to_string(), normalize_timestamp(inner[1].as_f64()?), inner[2].as_f64()?));
            }
        }
        None
    }

    async fn notify_waiters_module_stopped(&mut self) {
        if let Some((req_id, _, _, _)) = self.pending_request.take() {
            let _ = self.command_responder.send(CommandResponse::Shutdown { req_id }).await;
        }
    }
}

impl Drop for HistoricalDataApiModule {
    fn drop(&mut self) {
        tracing::debug!(target: "HistoricalDataApiModule", "HistoricalDataApiModule dropped");
    }
}
