use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use rust_decimal::prelude::ToPrimitive;
use serde::Deserialize;
use tokio::{select, sync::Mutex, time::timeout};
use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::{
    candle::{compile_candles_from_ticks, BaseCandle, Candle, CandleItem, HistoryItem},
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
};

const HISTORICAL_DATA_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MISMATCH_RETRIES: usize = 5;

#[derive(Debug, Clone)]
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
    Error(String),
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
    ///
    /// # Expected Data Format
    /// The response is expected to contain a list of ticks, where each tick is a tuple of `(timestamp, price)`.
    ///
    /// # Example
    /// ```rust,ignore
    /// let ticks = handle.ticks("EURUSD_otc".to_string(), 60).await?;
    /// for (timestamp, price) in ticks {
    ///     println!("Time: {}, Price: {}", timestamp, price);
    /// }
    /// ```
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
                Ok(Ok(CommandResponse::Candles { .. })) => {
                    // If we got candles but wanted ticks, we might be in trouble if we don't handle it.
                    // But usually the actor handles the response type.
                    continue;
                }
                Ok(Ok(CommandResponse::Error(e))) => return Err(PocketError::General(e)),
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
    ///
    /// # Expected Data Format
    /// The response is expected to contain a list of `Candle` objects.
    /// The server response typically includes OHLC data which is parsed into `Candle` structs.
    ///
    /// # Example
    /// ```rust,ignore
    /// let candles = handle.candles("EURUSD_otc".to_string(), 60).await?;
    /// for candle in candles {
    ///     println!("Time: {}, Open: {}, Close: {}", candle.timestamp, candle.open, candle.close);
    /// }
    /// ```
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
                Ok(Ok(CommandResponse::Ticks { .. })) => {
                    continue;
                }
                Ok(Ok(CommandResponse::Error(e))) => return Err(PocketError::General(e)),
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

    /// Deprecated: use `ticks()` or `candles()` instead.
    pub async fn get_history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        self.candles(asset, period).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RequestType {
    Ticks,
    Candles,
}

/// This API module handles historical data requests.
///
/// **Concurrency Notes:**
/// - Only one `get_history` request is supported at a time by this module's actor.
/// - The `last_req_id` field is purely for client-side bookkeeping to correlate responses;
///   the PocketOption server protocol does not echo this `req_id` in its responses.
/// - `MAX_MISMATCH_RETRIES` exists to guard against potential misrouted `CommandResponse` messages
///   if the `AsyncReceiver` is shared with other consumers, or if messages arrive out of order
///   due to network conditions or client-side issues.
#[allow(dead_code)] // The state field is not directly read in the module's run logic, but used indirectly by the rule.
pub struct HistoricalDataApiModule {
    _state: Arc<State>, // Prefix with _ to mark as intentionally unused
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    pending_request: Option<(Uuid, String, u32, RequestType)>,
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
    ) -> Self {
        Self {
            _state: shared_state, // Prefix with _ to mark as intentionally unused
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
            pending_request: None,
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
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::GetTicks { asset, period, req_id } => {
                            if self.pending_request.is_some() {
                                warn!(target: "HistoricalDataApiModule", "Overwriting a pending request. Concurrent calls are not supported.");
                            }
                            self.pending_request = Some((req_id, asset.clone(), period, RequestType::Ticks));
                            let payload = serde_json::json!([
                                "changeSymbol",
                                {
                                    "asset": asset,
                                    "period": period
                                }
                            ]);
                            let serialized_payload = serde_json::to_string(&payload)?;
                            let msg = format!("42{}", serialized_payload);
                            self.to_ws_sender.send(Message::text(msg)).await?;
                        }
                        Command::GetCandles { asset, period, req_id } => {
                            if self.pending_request.is_some() {
                                warn!(target: "HistoricalDataApiModule", "Overwriting a pending request. Concurrent calls are not supported.");
                            }
                            self.pending_request = Some((req_id, asset.clone(), period, RequestType::Candles));
                            let payload = serde_json::json!([
                                "changeSymbol",
                                {
                                    "asset": asset,
                                    "period": period
                                }
                            ]);
                            let serialized_payload = serde_json::to_string(&payload)?;
                            let msg = format!("42{}", serialized_payload);
                            self.to_ws_sender.send(Message::text(msg)).await?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    let mut is_binary_placeholder = false;
                    let response = match &*msg {
                        Message::Binary(data) => serde_json::from_slice::<ServerResponse>(data).ok(),
                        Message::Text(text) => {
                            if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                                Some(res)
                            } else if let Some(start) = text.find('[') {
                                // Try parsing as a 1-step Socket.IO message: 42["updateHistory", {...}]
                                if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&text[start..]) {
                                    if arr.len() >= 2 && arr[0].as_str().map(|s| s.starts_with("updateHistory")).unwrap_or(false) {
                                        // Check for binary placeholder
                                        if arr[1].as_object().is_some_and(|obj| obj.contains_key("_placeholder")) {
                                            is_binary_placeholder = true;
                                            None
                                        } else {
                                            serde_json::from_value::<ServerResponse>(arr[1].clone()).ok()
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        },
                        _ => None,
                    };

                    if is_binary_placeholder {
                        // Wait for the next message (the binary payload)
                        continue;
                    }

                    if let Some(response) = response {
                        match response {
                            ServerResponse::Success(candles) => {
                                if let Some((req_id, _, _, req_type)) = self.pending_request.take() {
                                    match req_type {
                                        RequestType::Candles => {
                                            self.command_responder.send(CommandResponse::Candles {
                                                req_id,
                                                candles,
                                            }).await?;
                                        }
                                        RequestType::Ticks => {
                                            // Convert candles back to ticks (not ideal but better than nothing)
                                            let ticks = candles.iter().map(|c| (c.timestamp, c.close.to_f64().unwrap_or_default())).collect();
                                            self.command_responder.send(CommandResponse::Ticks {
                                                req_id,
                                                ticks,
                                            }).await?;
                                        }
                                    }
                                } else {
                                    warn!(target: "HistoricalDataApiModule", "Received history data but no req_id was pending. Discarding.");
                                }
                            }
                            ServerResponse::History(history_response) => {
                                if let Some((_req_id, requested_asset, requested_period, _req_type)) = self.pending_request.as_ref() {
                                    // Validate that the response matches the pending request
                                    if history_response.asset != *requested_asset || history_response.period != *requested_period {
                                        warn!(
                                            target: "HistoricalDataApiModule",
                                            "Received history for {} (p:{}) but expected {} (p:{}). Skipping.",
                                            history_response.asset, history_response.period, requested_asset, requested_period
                                        );
                                        continue;
                                    }

                                    let (req_id, _, _, req_type) = if let Some(req) = self.pending_request.take() {
                                        req
                                    } else {
                                        warn!(target: "HistoricalDataApiModule", "Pending request missing when expected.");
                                        continue;
                                    };
                                    let symbol = history_response.asset;

                                    // Extract ticks first if available
                                    let mut ticks = Vec::new();
                                    if let Some(history_items) = history_response.history.as_ref() {
                                        ticks = history_items.iter().map(|item| item.to_tick()).collect();
                                    }

                                    if req_type == RequestType::Ticks {
                                        // If we only have candles, try to get ticks from them
                                        if ticks.is_empty() {
                                             if let Some(candle_items) = history_response.candles {
                                                ticks = candle_items.iter().map(|item| (item.0 as i64, item.2)).collect(); // timestamp, close
                                             } else if let (Some(timestamps), Some(c)) = (history_response.timestamps, history_response.c) {
                                                let len = timestamps.len().min(c.len());
                                                for i in 0..len {
                                                    ticks.push((timestamps[i] as i64, c[i]));
                                                }
                                             }
                                        }

                                        self.command_responder.send(CommandResponse::Ticks {
                                            req_id,
                                            ticks,
                                        }).await?;
                                    } else {
                                        // RequestType::Candles
                                        let mut candles = Vec::new();
                                        let mut has_candles = false;
                                        if let Some(candle_items) = history_response.candles {
                                            if !candle_items.is_empty() {
                                                has_candles = true;
                                                // Handle nested array candles format
                                                // Format: [timestamp, open, close, high, low, volume]
                                                for item in candle_items {
                                                    let base_candle = BaseCandle {
                                                        timestamp: item.0 as i64,
                                                        open: item.1,
                                                        close: item.2,
                                                        high: item.3,
                                                        low: item.4,
                                                        volume: Some(item.5),
                                                    };
                                                    if let Ok(candle) = Candle::try_from((base_candle, symbol.clone())) {
                                                        candles.push(candle);
                                                    }
                                                }
                                            }
                                        }

                                        if !has_candles {
                                            if let Some(history_items) = history_response.history {
                                                // Handle nested array ticks format - compile to candles
                                                candles = compile_candles_from_ticks(&history_items, history_response.period, &symbol);
                                            } else if let (Some(timestamps), Some(o), Some(h), Some(l), Some(c)) = (
                                                history_response.timestamps,
                                                history_response.o,
                                                history_response.h,
                                                history_response.l,
                                                history_response.c,
                                            ) {
                                                // Handle legacy separate arrays format
                                                let len = timestamps.len();
                                                let min_len = len.min(o.len()).min(h.len()).min(l.len()).min(c.len());

                                                for i in 0..min_len {
                                                    let base_candle = BaseCandle {
                                                        timestamp: timestamps[i] as i64,
                                                        open: o[i],
                                                        close: c[i],
                                                        high: h[i],
                                                        low: l[i],
                                                        volume: history_response.v.as_ref().and_then(|v| v.get(i).cloned()),
                                                    };
                                                    if let Ok(candle) = Candle::try_from((base_candle, symbol.clone())) {
                                                        candles.push(candle);
                                                    }
                                                }
                                            }
                                        }

                                        self.command_responder.send(CommandResponse::Candles {
                                            req_id,
                                            candles,
                                        }).await?;
                                    }
                                } else {
                                    warn!(target: "HistoricalDataApiModule", "Received history data but no req_id was pending. Discarding.");
                                }
                            }
                            ServerResponse::Fail(e) => {
                                self.pending_request = None;
                                self.command_responder.send(CommandResponse::Error(e)).await?;
                            }
                        }
                    } else {
                        warn!(
                            target: "HistoricalDataApiModule",
                            "Failed to deserialize message. Message: {:?}", msg
                        );
                    }
                }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec![
            "updateHistory",
            "updateHistoryNewFast",
            "updateHistoryNew",
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pocketoption::ssid::Ssid;
    use crate::pocketoption::state::StateBuilder;
    use binary_options_tools_core_pre::reimports::{bounded_async, Message};
    use binary_options_tools_core_pre::traits::ApiModule;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_historical_data_flow_binary_response() {
        // Setup channels
        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);

        // Create shared state using StateBuilder
        // We need a dummy SSID string that passes parsing
        let dummy_ssid_str =
            r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");

        let state = Arc::new(
            StateBuilder::default()
                .ssid(ssid)
                .build()
                .expect("Failed to build state"),
        );

        // Initialize the module
        let mut module =
            HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

        // Spawn the module loop in a separate task
        tokio::spawn(async move {
            if let Err(e) = module.run().await {
                eprintln!("Module run error: {:?}", e);
            }
        });

        // 1. Send GetHistory command
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

        // 3. Simulate incoming response (updateHistoryNewFast) as Binary
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
                assert_eq!(candles[0].timestamp, 1766378160);
                // Use from_str to ensure precise decimal representation matching the input string
                assert_eq!(
                    candles[0].open,
                    rust_decimal::Decimal::from_str_exact("122.24").unwrap()
                );
            }
            _ => panic!("Expected Candles response"),
        }
    }

    #[tokio::test]
    async fn test_historical_data_flow_text_response() {
        // Setup channels
        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);

        // Create shared state
        let dummy_ssid_str =
            r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");

        let state = Arc::new(
            StateBuilder::default()
                .ssid(ssid)
                .build()
                .expect("Failed to build state"),
        );

        // Initialize the module
        let mut module =
            HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

        // Spawn the module loop in a separate task
        tokio::spawn(async move {
            if let Err(e) = module.run().await {
                eprintln!("Module run error: {:?}", e);
            }
        });

        // 1. Send GetHistory command
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

        // 2. Consume WS message
        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        // 3. Simulate incoming response as Text
        let response_payload = r#"{
            "asset": "AUDUSD_otc",
            "period": 60,
            "o": [0.59563],
            "h": [0.59563],
            "l": [0.59511],
            "c": [0.59514],
            "t": [1766378160]
        }"#;

        let msg = Message::Text(response_payload.to_string().into());
        msg_tx
            .send(Arc::new(msg))
            .await
            .expect("Failed to send mock incoming message");

        // 4. Verify response
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
    async fn test_historical_data_mismatch_retry() {
        // Setup channels
        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);

        // Create shared state
        let dummy_ssid_str =
            r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");

        let state = Arc::new(
            StateBuilder::default()
                .ssid(ssid)
                .build()
                .expect("Failed to build state"),
        );

        // Initialize the module
        let mut module =
            HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

        // Spawn the module loop
        tokio::spawn(async move {
            if let Err(e) = module.run().await {
                eprintln!("Module run error: {:?}", e);
            }
        });

        // 1. Send GetCandles command
        let req_id = Uuid::new_v4();
        let asset = "EURUSD_otc".to_string();
        let period = 60;

        cmd_tx
            .send(Command::GetCandles {
                asset: asset.clone(),
                period,
                req_id,
            })
            .await
            .expect("Failed to send command");

        // 2. Consume WS message
        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        // 3. Send MISMATCHING response (wrong asset)
        let response_payload_mismatch = r#"{
            "asset": "WRONG_ASSET",
            "period": 60,
            "history": []
        }"#;
        let msg_mismatch = Message::Text(response_payload_mismatch.to_string().into());
        msg_tx
            .send(Arc::new(msg_mismatch))
            .await
            .expect("Failed to send mismatch message");

        // 4. Send CORRECT response
        let response_payload_correct = r#"{
            "asset": "EURUSD_otc",
            "period": 60,
            "history": []
        }"#;
        let msg_correct = Message::Text(response_payload_correct.to_string().into());
        msg_tx
            .send(Arc::new(msg_correct))
            .await
            .expect("Failed to send correct message");

        // 5. Verify we get the response for the correct one
        // The mismatch one should be ignored.
        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out waiting for response")
            .expect("Failed to receive module response");

        match response {
            CommandResponse::Candles { req_id: r_id, .. } => {
                assert_eq!(r_id, req_id);
            }
            _ => panic!("Expected Candles response"),
        }
    }

    #[tokio::test]
    async fn test_historical_data_no_pending_request() {
        // Setup channels
        let (_cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, _ws_rx) = bounded_async(10);

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
            HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

        tokio::spawn(async move {
            let _ = module.run().await;
        });

        // 1. Send unsolicited response
        let response_payload = r#"{
            "asset": "EURUSD_otc",
            "period": 60,
            "history": []
        }"#;
        let msg = Message::Text(response_payload.to_string().into());
        msg_tx
            .send(Arc::new(msg))
            .await
            .expect("Failed to send message");

        // 2. Verify NO response is sent
        let result = timeout(Duration::from_millis(200), resp_rx.recv()).await;
        assert!(
            result.is_err(),
            "Should not receive a response when no request was pending"
        );
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        // Setup channels
        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);

        // Create shared state
        let dummy_ssid_str =
            r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");
        let state = Arc::new(
            StateBuilder::default()
                .ssid(ssid)
                .build()
                .expect("Failed to build state"),
        );

        // Initialize the module
        let mut module =
            HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

        // Spawn the module loop
        tokio::spawn(async move {
            if let Err(e) = module.run().await {
                eprintln!("Module run error: {:?}", e);
            }
        });

        // 1. Send First Request
        let req_id1 = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "ASSET1".to_string(),
                period: 60,
                req_id: req_id1,
            })
            .await
            .expect("Failed to send command 1");

        // Consume WS message 1
        let _ = ws_rx.recv().await.expect("Failed to receive WS message 1");

        // 2. Send Second Request (Concurrent)
        let req_id2 = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "ASSET2".to_string(),
                period: 60,
                req_id: req_id2,
            })
            .await
            .expect("Failed to send command 2");

        // Consume WS message 2
        let _ = ws_rx.recv().await.expect("Failed to receive WS message 2");

        // 3. Send Response for Request 2 (The one that should be pending now)
        let response_payload2 = r#"{
            "asset": "ASSET2",
            "period": 60,
            "history": []
        }"#;
        msg_tx
            .send(Arc::new(Message::Text(
                response_payload2.to_string().into(),
            )))
            .await
            .expect("Failed to send message");

        // 4. Verify Response for Request 2
        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");

        match response {
            CommandResponse::Candles { req_id, .. } => {
                assert_eq!(
                    req_id, req_id2,
                    "Should receive response for the second request"
                );
            }
            _ => panic!("Expected Candles response"),
        }

        // 5. Send Response for Request 1 (Should be ignored as it was overwritten)
        let response_payload1 = r#"{
            "asset": "ASSET1",
            "period": 60,
            "history": []
        }"#;
        msg_tx
            .send(Arc::new(Message::Text(
                response_payload1.to_string().into(),
            )))
            .await
            .expect("Failed to send message");

        // 6. Verify NO Response for Request 1
        let result = timeout(Duration::from_millis(200), resp_rx.recv()).await;
        assert!(
            result.is_err(),
            "Should not receive response for overwritten request"
        );
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        // Setup channels
        let (cmd_tx, cmd_rx) = bounded_async(10);
        let (resp_tx, resp_rx) = bounded_async(10);
        let (msg_tx, msg_rx) = bounded_async(10);
        let (ws_tx, ws_rx) = bounded_async(10);

        // Create shared state
        let dummy_ssid_str =
            r#"42["auth",{"session":"dummy_session","isDemo":1,"uid":123,"platform":2}]"#;
        let ssid = Ssid::parse(dummy_ssid_str).expect("Failed to parse dummy SSID");
        let state = Arc::new(
            StateBuilder::default()
                .ssid(ssid)
                .build()
                .expect("Failed to build state"),
        );

        // Initialize the module
        let mut module =
            HistoricalDataApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx);

        // Spawn the module loop
        tokio::spawn(async move {
            if let Err(e) = module.run().await {
                eprintln!("Module run error: {:?}", e);
            }
        });

        // 1. Send Request
        let req_id = Uuid::new_v4();
        cmd_tx
            .send(Command::GetCandles {
                asset: "EURUSD_otc".to_string(),
                period: 60,
                req_id,
            })
            .await
            .expect("Failed to send command");

        // Consume WS message
        let _ = ws_rx.recv().await.expect("Failed to receive WS message");

        // 2. Send Invalid JSON Response
        let invalid_payload = "INVALID_JSON_DATA";
        msg_tx
            .send(Arc::new(Message::Text(invalid_payload.to_string().into())))
            .await
            .expect("Failed to send message");

        // 3. Verify NO Crash and NO Response (it should be ignored)
        let result = timeout(Duration::from_millis(200), resp_rx.recv()).await;
        assert!(
            result.is_err(),
            "Should not receive response for invalid JSON"
        );

        // 4. Send Valid Response afterwards to ensure module is still alive
        let valid_payload = r#"{
            "asset": "EURUSD_otc",
            "period": 60,
            "history": []
        }"#;
        msg_tx
            .send(Arc::new(Message::Text(valid_payload.to_string().into())))
            .await
            .expect("Failed to send message");

        // 5. Verify Response
        let response = timeout(Duration::from_secs(1), resp_rx.recv())
            .await
            .expect("Timed out")
            .expect("Failed to receive response");

        match response {
            CommandResponse::Candles { req_id: r_id, .. } => {
                assert_eq!(r_id, req_id);
            }
            _ => panic!("Expected Candles response"),
        }
    }
}
