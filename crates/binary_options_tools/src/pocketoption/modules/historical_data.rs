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
    compile_candles_from_ticks, merge_history_ohlc, merge_history_points, BaseCandle, Candle,
    CandleItem, HistoryItem, HistoryPoint,
};
use crate::pocketoption::error::{PocketError, PocketResult};
use crate::pocketoption::state::State;
use crate::pocketoption::types::MultiPatternRule;
use crate::pocketoption::utils::normalize_timestamp;

const HISTORICAL_DATA_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MISMATCH_RETRIES: usize = 5;

/// The kind of historical data a pending request is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestType {
    /// Raw `(timestamp, price)` ticks.
    Ticks,
    /// OHLC candles compiled from the raw server history.
    Candles,
    /// Pocket-style merged history points (ticks + synthetic candle points).
    HistoryPoints,
    /// Closed OHLC candles built from the merged history point stream.
    HistoryOhlc,
}

/// Commands accepted by [`HistoricalDataApiModule`].
#[derive(Debug)]
pub enum Command {
    /// Request raw `(timestamp, price)` ticks for an asset.
    GetTicks {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
    /// Request compiled OHLC candles for an asset.
    GetCandles {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
    /// Request Pocket-style merged history points for an asset.
    GetHistoryPoints {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
    /// Request closed OHLC candles from the merged history flow.
    GetHistoryOhlc {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
}

/// Responses emitted by [`HistoricalDataApiModule`].
///
/// Every variant carries the `req_id` of the request it answers so waiters
/// can correlate responses even if a stale or superseded response is still
/// in the channel.
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
    HistoryPoints {
        req_id: Uuid,
        points: Vec<HistoryPoint>,
    },
    HistoryOhlc {
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

/// Raw payload of an `updateHistory*` server event.
///
/// The server answers a `changeSymbol` request with one of several formats:
/// tick history (`history`), nested-array candles (`candles`), or legacy
/// separate OHLC arrays (`o`/`h`/`l`/`c`/`t`/`v`). All are optional so a
/// single struct covers every variant.
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

/// Public handle for interacting with [`HistoricalDataApiModule`].
///
/// **Concurrency:** the module actor supports only one in-flight request, so
/// every public method serializes callers through an internal lock. A request
/// that is somehow superseded fails fast with an error instead of timing out.
#[derive(Debug, Clone)]
pub struct HistoricalDataHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
    call_lock: Arc<Mutex<()>>,
}

impl HistoricalDataHandle {
    /// Retrieves historical tick data (timestamp, price) for a specific asset and period.
    ///
    /// # Example
    /// ```rust,ignore
    /// let ticks = handle.ticks("EURUSD_otc".to_string(), 60).await?;
    /// for (timestamp, price) in ticks {
    ///     println!("Time: {}, Price: {}", timestamp, price);
    /// }
    /// ```
    pub async fn ticks(&self, asset: String, period: u32) -> PocketResult<Vec<(i64, f64)>> {
        let id = Uuid::new_v4();
        self.request(
            Command::GetTicks {
                asset: asset.clone(),
                period,
                req_id: id,
            },
            id,
            "ticks",
            &asset,
            period,
            |response| match response {
                CommandResponse::Ticks { req_id, ticks } => Some((req_id, ticks)),
                _ => None,
            },
        )
        .await
    }

    /// Retrieves historical candle data for a specific asset and period.
    ///
    /// # Example
    /// ```rust,ignore
    /// let candles = handle.candles("EURUSD_otc".to_string(), 60).await?;
    /// for candle in candles {
    ///     println!("Time: {}, Open: {}, Close: {}", candle.timestamp, candle.open, candle.close);
    /// }
    /// ```
    pub async fn candles(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let id = Uuid::new_v4();
        self.request(
            Command::GetCandles {
                asset: asset.clone(),
                period,
                req_id: id,
            },
            id,
            "candles",
            &asset,
            period,
            |response| match response {
                CommandResponse::Candles { req_id, candles } => Some((req_id, candles)),
                _ => None,
            },
        )
        .await
    }

    /// Retrieves the same merged point stream Pocket Option builds for its chart edge.
    ///
    /// The result combines raw tick history with synthetic points expanded
    /// from the server's OHLC candles (see [`merge_history_points`]).
    pub async fn history_points(
        &self,
        asset: String,
        period: u32,
    ) -> PocketResult<Vec<HistoryPoint>> {
        let id = Uuid::new_v4();
        self.request(
            Command::GetHistoryPoints {
                asset: asset.clone(),
                period,
                req_id: id,
            },
            id,
            "history_points",
            &asset,
            period,
            |response| match response {
                CommandResponse::HistoryPoints { req_id, points } => Some((req_id, points)),
                _ => None,
            },
        )
        .await
    }

    /// Retrieves closed OHLC candles from Pocket Option's merged chart history flow.
    ///
    /// The newest (still developing) candle is intentionally excluded (see
    /// [`merge_history_ohlc`]).
    pub async fn history_ohlc(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        let id = Uuid::new_v4();
        self.request(
            Command::GetHistoryOhlc {
                asset: asset.clone(),
                period,
                req_id: id,
            },
            id,
            "history_ohlc",
            &asset,
            period,
            |response| match response {
                CommandResponse::HistoryOhlc { req_id, candles } => Some((req_id, candles)),
                _ => None,
            },
        )
        .await
    }

    /// Sends `command` and waits for the matching response.
    ///
    /// `extract` pulls the `(req_id, value)` pair out of the response variant
    /// this request expects; every other data variant is skipped. `Error` and
    /// `Shutdown` responses addressed to this request abort the wait, and a
    /// bounded number of mismatched responses is tolerated before giving up.
    async fn request<T>(
        &self,
        command: Command,
        id: Uuid,
        task: &'static str,
        asset: &str,
        period: u32,
        extract: impl Fn(CommandResponse) -> Option<(Uuid, T)>,
    ) -> PocketResult<T> {
        // Serialize public calls: the module actor only supports one
        // in-flight request at a time.
        let _guard = self.call_lock.lock().await;

        self.sender.send(command).await.map_err(CoreError::from)?;

        let mut mismatch_count = 0;
        loop {
            match timeout(HISTORICAL_DATA_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Error { req_id, error })) if req_id == id => {
                    return Err(PocketError::General(error));
                }
                Ok(Ok(CommandResponse::Shutdown { req_id })) if req_id == id => {
                    return Err(PocketError::ModuleStopped {
                        module_name: "HistoricalDataApiModule".to_string(),
                        context: format!("HistoricalDataApiModule stopped during {task} request"),
                    });
                }
                Ok(Ok(response)) => match extract(response) {
                    Some((req_id, value)) if req_id == id => return Ok(value),
                    Some((req_id, _)) => {
                        warn!("Received {task} response for unknown req_id: {req_id}");
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: task.to_string(),
                                context: format!(
                                    "asset: {asset}, period: {period}, exceeded mismatch retries"
                                ),
                                duration: HISTORICAL_DATA_TIMEOUT,
                            });
                        }
                    }
                    // A response for a different request kind (or a stale
                    // Error/Shutdown for another id); keep waiting.
                    None => continue,
                },
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: task.to_string(),
                        context: format!("asset: {asset}, period: {period}"),
                        duration: HISTORICAL_DATA_TIMEOUT,
                    });
                }
            }
        }
    }
}

/// Actor that fetches historical market data over the `changeSymbol` flow.
///
/// **Concurrency notes:**
/// - Only one request is supported at a time; a new command supersedes any
///   pending one (the superseded waiter receives an `Error` response).
/// - The PocketOption server does not echo request ids, so correlation is
///   purely client-side bookkeeping via `pending_request`.
/// - Live `updateStream` ticks that arrive while a request is pending are
///   buffered in `latest_ticks` and merged into tick/candle responses so the
///   returned data reaches as close to "now" as possible.
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
                            let (asset, period, req_id, req_type) = match cmd {
                                Command::GetTicks { asset, period, req_id } => (asset, period, req_id, RequestType::Ticks),
                                Command::GetCandles { asset, period, req_id } => (asset, period, req_id, RequestType::Candles),
                                Command::GetHistoryPoints { asset, period, req_id } => (asset, period, req_id, RequestType::HistoryPoints),
                                Command::GetHistoryOhlc { asset, period, req_id } => (asset, period, req_id, RequestType::HistoryOhlc),
                            };
                            self.begin_request(asset, period, req_id, req_type).await?;
                        }
                        Err(_) => {
                            self.notify_waiters_module_stopped().await;
                            break;
                        }
                    }
                },
                msg_res = self.message_receiver.recv() => {
                    match msg_res {
                        Ok(msg) => self.handle_message(msg).await,
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
        Box::new(MultiPatternRule::new(vec![
            "updateHistory",
            "updateHistoryNewFast",
            "updateHistoryNew",
            "updateStream",
        ]))
    }
}

impl HistoricalDataApiModule {
    /// Registers a new pending request and sends the `changeSymbol` frame.
    ///
    /// Any previously pending request is superseded: its waiter is unblocked
    /// with an `Error` response so it fails fast instead of timing out.
    async fn begin_request(
        &mut self,
        asset: String,
        period: u32,
        req_id: Uuid,
        req_type: RequestType,
    ) -> CoreResult<()> {
        if let Some((prev_id, _, _, _)) = self.pending_request.take() {
            warn!(target: "HistoricalDataApiModule", "Overwriting pending request {} due to concurrent call", prev_id);
            let _ = self
                .command_responder
                .send(CommandResponse::Error {
                    req_id: prev_id,
                    error: "Request superseded by another concurrent call".to_string(),
                })
                .await;
        }

        // Start collecting live ticks fresh for this asset so stale data from
        // a previous request never leaks into the new response.
        self.latest_ticks.remove(&asset);
        self.pending_request = Some((req_id, asset.clone(), period, req_type));

        if let Err(e) = self.send_change_symbol(&asset, period).await {
            warn!(target: "HistoricalDataApiModule", "Failed to send history request: {}", e);
            self.pending_request = None;
            let _ = self
                .command_responder
                .send(CommandResponse::Error {
                    req_id,
                    error: e.to_string(),
                })
                .await;
        }
        Ok(())
    }

    /// Sends the `changeSymbol` Socket.IO frame that triggers a history push.
    async fn send_change_symbol(&self, asset: &str, period: u32) -> CoreResult<()> {
        let payload = serde_json::json!(["changeSymbol", { "asset": asset, "period": period }]);
        let msg = format!("42{}", serde_json::to_string(&payload)?);
        self.to_ws_sender
            .send(Message::text(msg))
            .await
            .map_err(CoreError::from)
    }

    /// Parses and dispatches one incoming WebSocket message.
    async fn handle_message(&mut self, msg: Arc<Message>) {
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
                    // Try parsing as a Socket.IO event frame: 42["updateHistory", {...}]
                    match serde_json::from_str::<serde_json::Value>(&text[start..]) {
                        Ok(serde_json::Value::Array(arr)) => {
                            if arr.len() >= 2
                                && arr[0]
                                    .as_str()
                                    .map(|s| s.starts_with("updateHistory"))
                                    .unwrap_or(false)
                            {
                                if arr[1]
                                    .as_object()
                                    .is_some_and(|obj| obj.contains_key("_placeholder"))
                                {
                                    // The payload arrives in a follow-up binary frame.
                                    is_binary_placeholder = true;
                                    None
                                } else {
                                    match serde_json::from_value::<ServerResponse>(arr[1].clone())
                                    {
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
            }
            _ => {
                warn!(target: "HistoricalDataApiModule", "Received unexpected message type: {:?}", msg);
                None
            }
        };

        if is_binary_placeholder {
            return;
        }

        // Messages that are not history responses may be live `updateStream`
        // rows; buffer them so they can be merged into the final response.
        if response.is_none() {
            let text = match &*msg {
                Message::Text(text) => Some(text.as_str()),
                Message::Binary(data) => std::str::from_utf8(data).ok(),
                _ => None,
            };
            if let Some(text) = text {
                if let Some((symbol, timestamp, price)) = Self::parse_update_stream(text) {
                    self.latest_ticks
                        .entry(symbol)
                        .or_default()
                        .push((timestamp, price));
                }
            }
            return;
        }

        match response {
            Some(ServerResponse::Success(candles)) => {
                if let Some((req_id, asset, _, req_type)) = self.pending_request.take() {
                    let response = Self::build_success_response(req_id, &asset, req_type, candles);
                    let _ = self.command_responder.send(response).await;
                }
            }
            Some(ServerResponse::History(history)) => {
                if let Some((req_id, asset, period, req_type)) = self.pending_request.take() {
                    if history.asset != asset || history.period != period {
                        // Not the response we are waiting for; keep the
                        // request pending until the matching one arrives.
                        self.pending_request = Some((req_id, asset, period, req_type));
                        return;
                    }
                    let response = self.build_history_response(req_id, req_type, history);
                    let _ = self.command_responder.send(response).await;
                }
            }
            Some(ServerResponse::Fail(e)) => {
                if let Some((req_id, _, _, _)) = self.pending_request.take() {
                    let _ = self
                        .command_responder
                        .send(CommandResponse::Error { req_id, error: e })
                        .await;
                }
            }
            None => {}
        }
    }

    /// Builds the response for a direct candle-list payload, adapting it to
    /// whatever data shape the pending request asked for.
    fn build_success_response(
        req_id: Uuid,
        asset: &str,
        req_type: RequestType,
        candles: Vec<Candle>,
    ) -> CommandResponse {
        match req_type {
            RequestType::Ticks => CommandResponse::Ticks {
                req_id,
                ticks: candles
                    .iter()
                    .map(|c| (c.timestamp, c.close.to_f64().unwrap_or_default()))
                    .collect(),
            },
            RequestType::Candles => CommandResponse::Candles { req_id, candles },
            RequestType::HistoryPoints => CommandResponse::HistoryPoints {
                req_id,
                points: candles
                    .iter()
                    .map(|c| HistoryPoint {
                        asset: asset.to_string(),
                        time: c.timestamp as f64,
                        price: c.close.to_f64().unwrap_or_default(),
                    })
                    .collect(),
            },
            RequestType::HistoryOhlc => CommandResponse::HistoryOhlc { req_id, candles },
        }
    }

    /// Builds the response for an `updateHistory*` payload, adapting it to
    /// whatever data shape the pending request asked for.
    fn build_history_response(
        &self,
        req_id: Uuid,
        req_type: RequestType,
        history: HistoryResponse,
    ) -> CommandResponse {
        let symbol = history.asset.clone();
        match req_type {
            RequestType::HistoryPoints => CommandResponse::HistoryPoints {
                req_id,
                points: merge_history_points(
                    &symbol,
                    history.period,
                    history.history.as_deref(),
                    history.candles.as_deref(),
                ),
            },
            RequestType::HistoryOhlc => CommandResponse::HistoryOhlc {
                req_id,
                candles: merge_history_ohlc(
                    &symbol,
                    history.period,
                    history.history.as_deref(),
                    history.candles.as_deref(),
                ),
            },
            RequestType::Ticks => {
                let mut ticks: Vec<(i64, f64)> = history
                    .history
                    .as_ref()
                    .map(|h| h.iter().map(|item| item.to_tick()).collect())
                    .unwrap_or_default();

                // Fall back to candle closes when no raw ticks were sent.
                if ticks.is_empty() {
                    if let Some(candle_items) = history.candles {
                        ticks = candle_items
                            .iter()
                            .map(|item| (item.timestamp, item.close))
                            .collect();
                    } else if let (Some(timestamps), Some(c)) = (history.timestamps, history.c) {
                        let len = timestamps.len().min(c.len());
                        for i in 0..len {
                            ticks.push((timestamps[i] as i64, c[i]));
                        }
                    }
                }

                // Append live ticks that streamed in while we were waiting.
                if let Some(stream_ticks) = self.latest_ticks.get(&symbol) {
                    let last_ts = ticks.last().map(|(t, _)| *t).unwrap_or(0);
                    for &(ts, price) in stream_ticks {
                        if ts > last_ts {
                            ticks.push((ts, price));
                        }
                    }
                }

                CommandResponse::Ticks { req_id, ticks }
            }
            RequestType::Candles => {
                let mut candles = Vec::new();

                // Preferred format: nested-array candles.
                if let Some(candle_items) = history.candles {
                    for item in candle_items {
                        let base_candle = BaseCandle {
                            timestamp: item.timestamp,
                            open: item.open,
                            close: item.close,
                            high: item.high,
                            low: item.low,
                            volume: Some(item.volume),
                        };
                        if let Ok(candle) = Candle::try_from((base_candle, symbol.clone())) {
                            candles.push(candle);
                        }
                    }
                } else if let (Some(timestamps), Some(o), Some(h), Some(l), Some(c)) = (
                    history.timestamps,
                    history.o,
                    history.h,
                    history.l,
                    history.c,
                ) {
                    // Legacy format: separate OHLC arrays.
                    let min_len = timestamps
                        .len()
                        .min(o.len())
                        .min(h.len())
                        .min(l.len())
                        .min(c.len());
                    for i in 0..min_len {
                        let base_candle = BaseCandle {
                            timestamp: timestamps[i] as i64,
                            open: o[i],
                            close: c[i],
                            high: h[i],
                            low: l[i],
                            volume: history.v.as_ref().and_then(|v| v.get(i).cloned()),
                        };
                        if let Ok(candle) = Candle::try_from((base_candle, symbol.clone())) {
                            candles.push(candle);
                        }
                    }
                }

                // Compile tick history plus any buffered live ticks into
                // candles that extend past the last server-provided candle.
                let mut history_items = history.history.unwrap_or_default();
                if let Some(stream_ticks) = self.latest_ticks.get(&symbol) {
                    for &(ts, price) in stream_ticks {
                        history_items.push(HistoryItem::Tick([
                            serde_json::Value::from(ts as f64),
                            serde_json::Value::from(price),
                        ]));
                    }
                }
                if !history_items.is_empty() {
                    let compiled =
                        compile_candles_from_ticks(&history_items, history.period, &symbol);
                    let last_ts = candles.iter().map(|c| c.timestamp).max().unwrap_or(0);
                    for compiled_candle in compiled {
                        if compiled_candle.timestamp > last_ts {
                            candles.push(compiled_candle);
                        }
                    }
                }
                candles.sort_by_key(|c| c.timestamp);

                CommandResponse::Candles { req_id, candles }
            }
        }
    }

    /// Extracts one `(symbol, timestamp, price)` row from an `updateStream`
    /// frame, returning `None` for any other message shape.
    fn parse_update_stream(text: &str) -> Option<(String, i64, f64)> {
        let start = text.find('[')?;
        let arr: serde_json::Value = serde_json::from_str(&text[start..]).ok()?;
        let outer = arr.as_array()?;
        let data = if let Some(first) = outer.first() {
            if first.is_string() && outer.len() >= 2 {
                outer.get(1)?.as_array()?
            } else {
                outer
            }
        } else {
            return None;
        };
        if let Some(inner) = data.first().and_then(|v| v.as_array()) {
            if inner.len() >= 3 {
                return Some((
                    inner[0].as_str()?.to_string(),
                    normalize_timestamp(inner[1].as_f64()?),
                    inner[2].as_f64()?,
                ));
            }
        }
        None
    }

    /// Unblocks the pending waiter (if any) with a `Shutdown` response.
    async fn notify_waiters_module_stopped(&mut self) {
        if let Some((req_id, _, _, _)) = self.pending_request.take() {
            let _ = self
                .command_responder
                .send(CommandResponse::Shutdown { req_id })
                .await;
        }
    }
}

impl Drop for HistoricalDataApiModule {
    fn drop(&mut self) {
        if let Some((req_id, _, _, _)) = self.pending_request.take() {
            let _ = self
                .command_responder
                .as_sync()
                .try_send(CommandResponse::Shutdown { req_id });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pocketoption::ssid::Ssid;
    use crate::pocketoption::state::StateBuilder;
    use binary_options_tools_core::reimports::{bounded_async, Message};
    use binary_options_tools_core::traits::ApiModule;
    use std::sync::Arc;
    use uuid::Uuid;

    type ModuleChannels = (
        AsyncSender<Command>,
        AsyncReceiver<CommandResponse>,
        AsyncSender<Arc<Message>>,
        AsyncReceiver<Message>,
    );

    /// Spawns the module actor with fresh channels and a dummy state.
    fn spawn_module() -> ModuleChannels {
        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);
        let (runner_tx, _runner_rx) = bounded_async(1);

        let dummy_ssid_str =
            r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");
        let state = Arc::new(
            StateBuilder::default()
                .ssid(ssid)
                .build()
                .expect("Failed to build state"),
        );

        let mut module =
            HistoricalDataApiModule::new(state, cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);
        tokio::spawn(async move {
            if let Err(e) = module.run().await {
                eprintln!("Module run error: {:?}", e);
            }
        });

        (cmd_tx, resp_rx, msg_tx, ws_rx)
    }

    #[tokio::test]
    async fn test_historical_data_flow_binary_response() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        // 1. Send GetCandles command
        let req_id = Uuid::new_v4();
        let asset = "CADJPY_otc".to_string();
        let period = 60;

        cmd_tx
            .send(Command::GetCandles {
                asset: asset.clone(),
                period,
                req_id,
            })
            .await
            .expect("Failed to send command");

        // 2. Verify the WS message sent (changeSymbol)
        let ws_msg = ws_rx.recv().await.expect("Failed to receive WS message");
        if let Message::Text(text) = ws_msg {
            let expected = format!(
                "42[\"changeSymbol\",{{\"asset\":\"{}\",\"period\":{}}}]",
                asset, period
            );
            assert_eq!(text, expected);
        } else {
            panic!("Expected Text message for WS");
        }

        // 3. Simulate incoming response (legacy OHLC arrays) as Binary
        let response_payload = r#"{
            "asset": "CADJPY_otc",
            "period": 60,
            "o": [122.24, 122.204],
            "h": [122.259, 122.272],
            "l": [122.184, 122.204],
            "c": [122.23, 122.243],
            "t": [1766378160, 1766378100]
        }"#;

        let msg = Message::Binary(response_payload.as_bytes().to_vec().into());
        msg_tx
            .send(Arc::new(msg))
            .await
            .expect("Failed to send mock incoming message");

        // 4. Verify the response from the module
        let response = resp_rx
            .recv()
            .await
            .expect("Failed to receive module response");

        match response {
            CommandResponse::Candles {
                req_id: r_id,
                candles,
            } => {
                assert_eq!(r_id, req_id);
                assert_eq!(candles.len(), 2);
                // Candles are sorted ascending by timestamp, so the older
                // entry (sent second by the server) comes first.
                assert_eq!(candles[0].timestamp, 1766378100);
                assert_eq!(
                    candles[0].open,
                    rust_decimal::Decimal::from_str_exact("122.204").unwrap()
                );
                assert_eq!(candles[1].timestamp, 1766378160);
                assert_eq!(
                    candles[1].open,
                    rust_decimal::Decimal::from_str_exact("122.24").unwrap()
                );
            }
            _ => panic!("Expected Candles response"),
        }
    }

    #[tokio::test]
    async fn test_historical_data_flow_text_response() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        let req_id = Uuid::new_v4();
        let asset = "AUDUSD_otc".to_string();
        let period = 60;

        cmd_tx
            .send(Command::GetCandles {
                asset: asset.clone(),
                period,
                req_id,
            })
            .await
            .expect("Failed to send command");

        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        let response_payload = r#"{
            "asset": "AUDUSD_otc",
            "period": 60,
            "o": [0.59563],
            "h": [0.59563],
            "l": [0.59511],
            "c": [0.59514],
            "t": [1766378160]
        }"#;

        msg_tx
            .send(Arc::new(Message::Text(response_payload.to_string().into())))
            .await
            .expect("Failed to send mock incoming message");

        let response = resp_rx
            .recv()
            .await
            .expect("Failed to receive module response");

        match response {
            CommandResponse::Candles {
                req_id: r_id,
                candles,
            } => {
                assert_eq!(r_id, req_id);
                assert_eq!(candles.len(), 1);
                assert_eq!(candles[0].timestamp, 1766378160);
                assert_eq!(
                    candles[0].close,
                    rust_decimal::Decimal::from_str_exact("0.59514").unwrap()
                );
            }
            _ => panic!("Expected Candles response"),
        }
    }

    #[tokio::test]
    async fn test_historical_data_mismatch_is_skipped() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        let req_id = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "EURUSD_otc".to_string(),
                period: 60,
                req_id,
            })
            .await
            .expect("Failed to send command");

        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        // A response for the wrong asset must be ignored...
        let mismatch = r#"{ "asset": "WRONG_ASSET", "period": 60, "history": [] }"#;
        msg_tx
            .send(Arc::new(Message::Text(mismatch.to_string().into())))
            .await
            .expect("Failed to send mismatch message");

        // ...and the correct one answered.
        let correct = r#"{ "asset": "EURUSD_otc", "period": 60, "history": [] }"#;
        msg_tx
            .send(Arc::new(Message::Text(correct.to_string().into())))
            .await
            .expect("Failed to send correct message");

        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out waiting for response")
            .expect("Failed to receive module response");

        match response {
            CommandResponse::Candles { req_id: r_id, .. } => assert_eq!(r_id, req_id),
            _ => panic!("Expected Candles response"),
        }
    }

    #[tokio::test]
    async fn test_historical_data_no_pending_request() {
        let (_cmd_tx, resp_rx, msg_tx, _ws_rx) = spawn_module();

        // An unsolicited history response must be discarded silently.
        let payload = r#"{ "asset": "EURUSD_otc", "period": 60, "history": [] }"#;
        msg_tx
            .send(Arc::new(Message::Text(payload.to_string().into())))
            .await
            .expect("Failed to send message");

        let result = timeout(Duration::from_millis(200), resp_rx.recv()).await;
        assert!(
            result.is_err(),
            "Should not receive a response when no request was pending"
        );
    }

    #[tokio::test]
    async fn test_superseded_request_gets_error() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        // 1. First request
        let req_id1 = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "ASSET1".to_string(),
                period: 60,
                req_id: req_id1,
            })
            .await
            .expect("Failed to send command 1");
        let _ = ws_rx.recv().await.expect("Failed to receive WS message 1");

        // 2. Second request supersedes the first
        let req_id2 = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "ASSET2".to_string(),
                period: 60,
                req_id: req_id2,
            })
            .await
            .expect("Failed to send command 2");
        let _ = ws_rx.recv().await.expect("Failed to receive WS message 2");

        // 3. The superseded request receives an Error response first
        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");
        match response {
            CommandResponse::Error { req_id, .. } => assert_eq!(req_id, req_id1),
            other => panic!("Expected Error for superseded request, got {other:?}"),
        }

        // 4. The second request is answered normally
        let payload2 = r#"{ "asset": "ASSET2", "period": 60, "history": [] }"#;
        msg_tx
            .send(Arc::new(Message::Text(payload2.to_string().into())))
            .await
            .expect("Failed to send message");

        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");
        match response {
            CommandResponse::Candles { req_id, .. } => assert_eq!(req_id, req_id2),
            other => panic!("Expected Candles response, got {other:?}"),
        }

        // 5. A late response for the first request is ignored
        let payload1 = r#"{ "asset": "ASSET1", "period": 60, "history": [] }"#;
        msg_tx
            .send(Arc::new(Message::Text(payload1.to_string().into())))
            .await
            .expect("Failed to send message");

        let result = timeout(Duration::from_millis(200), resp_rx.recv()).await;
        assert!(
            result.is_err(),
            "Should not receive response for superseded request"
        );
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        let req_id = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "EURUSD_otc".to_string(),
                period: 60,
                req_id,
            })
            .await
            .expect("Failed to send command");
        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        // Invalid JSON must be ignored without crashing the module...
        msg_tx
            .send(Arc::new(Message::Text(
                "INVALID_JSON_DATA".to_string().into(),
            )))
            .await
            .expect("Failed to send message");

        let result = timeout(Duration::from_millis(200), resp_rx.recv()).await;
        assert!(
            result.is_err(),
            "Should not receive response for invalid JSON"
        );

        // ...and a subsequent valid response must still be delivered.
        let valid = r#"{ "asset": "EURUSD_otc", "period": 60, "history": [] }"#;
        msg_tx
            .send(Arc::new(Message::Text(valid.to_string().into())))
            .await
            .expect("Failed to send message");

        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");
        match response {
            CommandResponse::Candles { req_id: r_id, .. } => assert_eq!(r_id, req_id),
            _ => panic!("Expected Candles response"),
        }
    }

    #[tokio::test]
    async fn test_history_points_from_merged_history() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        let req_id = Uuid::new_v4();
        cmd_tx
            .send(Command::GetHistoryPoints {
                asset: "EURUSD_otc".to_string(),
                period: 60,
                req_id,
            })
            .await
            .expect("Failed to send command");
        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        let payload = r#"{
            "asset": "EURUSD_otc",
            "period": 60,
            "history": [[100, 1.0], [160, 1.1]]
        }"#;
        msg_tx
            .send(Arc::new(Message::Text(payload.to_string().into())))
            .await
            .expect("Failed to send message");

        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");

        match response {
            CommandResponse::HistoryPoints {
                req_id: r_id,
                points,
            } => {
                assert_eq!(r_id, req_id);
                assert_eq!(points.len(), 2);
                assert_eq!(points[0].asset, "EURUSD_otc");
                assert_eq!(points[0].time, 100.0);
                assert_eq!(points[0].price, 1.0);
                assert_eq!(points[1].time, 160.0);
            }
            other => panic!("Expected HistoryPoints response, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_history_ohlc_drops_developing_candle() {
        let (cmd_tx, resp_rx, msg_tx, ws_rx) = spawn_module();

        let req_id = Uuid::new_v4();
        cmd_tx
            .send(Command::GetHistoryOhlc {
                asset: "EURUSD_otc".to_string(),
                period: 60,
                req_id,
            })
            .await
            .expect("Failed to send command");
        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        // Two complete buckets (60s each) plus one point in a third bucket;
        // the newest bucket must be dropped as "still developing".
        let payload = r#"{
            "asset": "EURUSD_otc",
            "period": 60,
            "history": [[100, 1.0], [101, 1.2], [160, 1.3], [161, 1.1], [220, 1.4]]
        }"#;
        msg_tx
            .send(Arc::new(Message::Text(payload.to_string().into())))
            .await
            .expect("Failed to send message");

        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");

        match response {
            CommandResponse::HistoryOhlc {
                req_id: r_id,
                candles,
            } => {
                assert_eq!(r_id, req_id);
                assert_eq!(candles.len(), 2, "developing candle must be excluded");
                assert_eq!(candles[0].timestamp, 60);
                assert_eq!(
                    candles[0].open,
                    rust_decimal::Decimal::from_str_exact("1").unwrap()
                );
                assert_eq!(
                    candles[0].close,
                    rust_decimal::Decimal::from_str_exact("1.2").unwrap()
                );
                assert_eq!(candles[1].timestamp, 120);
            }
            other => panic!("Expected HistoryOhlc response, got {other:?}"),
        }
    }
}
