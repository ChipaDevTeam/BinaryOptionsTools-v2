use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use serde::Deserialize;
use tokio::{select, sync::Mutex, time::timeout};
use tracing::warn;
use uuid::Uuid;

use crate::pocketoption::{
    candle::{BaseCandle, Candle},
    error::{PocketError, PocketResult},
    state::State,
    types::MultiPatternRule,
};

const HISTORICAL_DATA_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MISMATCH_RETRIES: usize = 5;

#[derive(Debug, Clone)]
pub enum Command {
    GetHistory {
        asset: String,
        period: u32,
        req_id: Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum CommandResponse {
    Success { req_id: Uuid, candles: Vec<Candle> },
    Error(String),
}

#[derive(Deserialize)]
pub struct HistoryResponse {
    asset: String,
    #[allow(dead_code)]
    period: u64,
    #[serde(default)]
    #[allow(dead_code)]
    history: Option<Vec<Vec<f64>>>,
    // Separate arrays for OHLC data
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    #[serde(alias = "t")]
    timestamps: Vec<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    v: Option<Vec<f64>>, // Volume might be optional
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
    /// Retrieves historical candle data for a specific asset and period.
    ///
    /// **Concurrency Warning:**
    /// This method uses a shared internal lock (`call_lock`) to serialize requests across
    /// all clones of this handle. This ensures that only one `get_history` request is
    /// in-flight at a time for the underlying client actor, matching the actor's single-request
    /// limitation. Concurrent calls will be queued and executed sequentially.
    pub async fn get_history(&self, asset: String, period: u32) -> PocketResult<Vec<Candle>> {
        // Acquire lock to serialize requests.
        // This ensures that we don't have concurrent get_history calls racing for responses
        // on the shared receiver, matching the single-request limitation of the actor.
        let _guard = self.call_lock.lock().await;

        let id = Uuid::new_v4();
        self.sender
            .send(Command::GetHistory {
                asset: asset.clone(),
                period,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        let mut mismatch_count = 0;
        loop {
            // Mismatched `req_id` cases are typically only expected if other consumers
            // are unexpectedly sharing the same response receiver, or if messages
            // arrive severely out of order (though less likely with serialized requests).
            match timeout(HISTORICAL_DATA_TIMEOUT, self.receiver.recv()).await {
                Ok(Ok(CommandResponse::Success { req_id, candles })) => {
                    if req_id == id {
                        return Ok(candles);
                    } else {
                        warn!("Received response for unknown req_id: {}", req_id);
                        mismatch_count += 1;
                        if mismatch_count >= MAX_MISMATCH_RETRIES {
                            return Err(PocketError::Timeout {
                                task: "get_history".to_string(),
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
                Ok(Ok(CommandResponse::Error(e))) => return Err(PocketError::General(e)),
                Ok(Err(e)) => return Err(CoreError::from(e).into()),
                Err(_) => {
                    return Err(PocketError::Timeout {
                        task: "get_history".to_string(),
                        context: format!("asset: {}, period: {}", asset, period),
                        duration: HISTORICAL_DATA_TIMEOUT,
                    });
                }
            }
        }
    }
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
    last_req_id: Option<Uuid>,
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
            last_req_id: None,
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
                        Command::GetHistory { asset, period, req_id } => {
                            if self.last_req_id.is_some() {
                                warn!(target: "HistoricalDataApiModule", "Overwriting a pending request. Concurrent get_history calls are not supported.");
                            }
                            self.last_req_id = Some(req_id);
                            let msg = format!("42[\"changeSymbol\",{{\"asset\":\"{}\",\"period\":{}}}]", asset, period);
                            self.to_ws_sender.send(Message::text(msg)).await?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    let response = match &*msg {
                        Message::Binary(data) => serde_json::from_slice::<ServerResponse>(data).ok(),
                        Message::Text(text) => serde_json::from_str::<ServerResponse>(text).ok(),
                        _ => None,
                    };

                    if let Some(response) = response {
                        match response {
                            ServerResponse::Success(candles) => {
                                if let Some(req_id) = self.last_req_id.take() {
                                    self.command_responder.send(CommandResponse::Success {
                                        req_id,
                                        candles,
                                    }).await?;
                                } else {
                                    warn!(target: "HistoricalDataApiModule", "Received history data but no req_id was pending. Discarding.");
                                }
                            }
                            ServerResponse::History(history_response) => {
                                if let Some(req_id) = self.last_req_id.take() {
                                    let len = history_response.timestamps.len();
                                    let mut candles = Vec::with_capacity(len);
                                    let symbol = history_response.asset;

                                    // Check if all arrays have the same length (basic validation)
                                    if history_response.o.len() != len
                                        || history_response.h.len() != len
                                        || history_response.l.len() != len
                                        || history_response.c.len() != len
                                    {
                                        warn!(
                                            target: "HistoricalDataApiModule",
                                            "History response array lengths mismatch. Timestamps: {}, Open: {}, High: {}, Low: {}, Close: {}",
                                            len, history_response.o.len(), history_response.h.len(), history_response.l.len(), history_response.c.len()
                                        );
                                        // We might still try to parse as many as possible or fail.
                                        // Let's iterate up to the minimum length to be safe.
                                    }

                                    let min_len = len
                                        .min(history_response.o.len())
                                        .min(history_response.h.len())
                                        .min(history_response.l.len())
                                        .min(history_response.c.len());

                                    for i in 0..min_len {
                                        let base_candle = BaseCandle {
                                            timestamp: history_response.timestamps[i] as f64,
                                            open: history_response.o[i],
                                            close: history_response.c[i],
                                            high: history_response.h[i],
                                            low: history_response.l[i],
                                            volume: history_response.v.as_ref().and_then(|v| v.get(i).cloned()),
                                        };
                                        if let Ok(candle) = Candle::try_from((base_candle, symbol.clone())) {
                                            candles.push(candle);
                                        }
                                    }

                                    self.command_responder.send(CommandResponse::Success {
                                        req_id,
                                        candles,
                                    }).await?;
                                } else {
                                    warn!(target: "HistoricalDataApiModule", "Received history data but no req_id was pending. Discarding.");
                                }
                            }
                            ServerResponse::Fail(e) => {
                                self.last_req_id = None;
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
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pocketoption::ssid::Ssid;
    use crate::pocketoption::state::StateBuilder;
    use binary_options_tools_core_pre::reimports::{Message, bounded_async};
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
            .send(Command::GetHistory {
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
            CommandResponse::Success {
                req_id: r_id,
                candles,
            } => {
                assert_eq!(r_id, req_id);
                assert_eq!(candles.len(), 2);
                assert_eq!(candles[0].timestamp, 1766378160.0);
                // Use from_str to ensure precise decimal representation matching the input string
                assert_eq!(
                    candles[0].open,
                    rust_decimal::Decimal::from_str_exact("122.24").unwrap()
                );
            }
            CommandResponse::Error(e) => panic!("Received error response: {}", e),
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
            .send(Command::GetHistory {
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
            CommandResponse::Success {
                req_id: r_id,
                candles,
            } => {
                assert_eq!(r_id, req_id);
                assert_eq!(candles.len(), 1);
                assert_eq!(candles[0].timestamp, 1766378160.0);
                assert_eq!(
                    candles[0].close,
                    rust_decimal::Decimal::from_str_exact("0.59514").unwrap()
                );
            }
            CommandResponse::Error(e) => panic!("Received error response: {}", e),
        }
    }
}
