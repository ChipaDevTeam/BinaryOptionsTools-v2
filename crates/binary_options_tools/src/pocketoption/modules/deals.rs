use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::CoreError,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::oneshot;
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
    types::Deal,
};

const UPDATE_OPENED_DEALS: &str = r#"451-["updateOpenedDeals","#;
const UPDATE_CLOSED_DEALS: &str = r#"451-["updateClosedDeals","#;
const SUCCESS_CLOSE_ORDER: &str = r#"451-["successcloseOrder","#;

#[derive(Debug)]
pub enum Command {
    CheckResult(Uuid, oneshot::Sender<PocketResult<Deal>>),
}

#[derive(Debug)]
pub enum CommandResponse {
    CheckResult(Box<Deal>),
    DealNotFound(Uuid),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ExpectedMessage {
    UpdateClosedDeals,
    UpdateOpenedDeals,
    SuccessCloseOrder,
    None,
}

#[derive(Deserialize)]
struct CloseOrder {
    #[serde(rename = "profit")]
    _profit: Decimal,
    deals: Vec<Deal>,
}

#[derive(Clone)]
pub struct DealsHandle {
    sender: AsyncSender<Command>,
    _receiver: AsyncReceiver<CommandResponse>,
}

impl DealsHandle {
    pub async fn check_result(&self, trade_id: Uuid) -> PocketResult<Deal> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Command::CheckResult(trade_id, tx))
            .await
            .map_err(CoreError::from)?;

        match rx.await {
            Ok(result) => result,
            Err(_) => Err(CoreError::Other("DealsApiModule responder dropped".into()).into()),
        }
    }

    pub async fn check_result_with_timeout(
        &self,
        trade_id: Uuid,
        timeout: Duration,
    ) -> PocketResult<Deal> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(Command::CheckResult(trade_id, tx))
            .await
            .map_err(CoreError::from)?;

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CoreError::Other("DealsApiModule responder dropped".into()).into()),
            Err(_) => Err(PocketError::Timeout {
                task: "check_result".to_string(),
                context: format!("Waiting for trade '{trade_id}' result"),
                duration: timeout,
            }),
        }
    }
}

/// An API module responsible for listening to deal updates,
/// maintaining the shared `TradeState`, and checking trade results.
pub struct DealsApiModule {
    state: Arc<State>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    command_receiver: AsyncReceiver<Command>,
    _command_responder: AsyncSender<CommandResponse>,
    // Map of Trade ID -> List of waiters expecting the result
    waiting_requests: HashMap<Uuid, Vec<oneshot::Sender<PocketResult<Deal>>>>,
}

impl DealsApiModule {
    async fn process_text_data(&mut self, text: &str, expected: ExpectedMessage) {
        match expected {
            ExpectedMessage::UpdateOpenedDeals => match serde_json::from_str::<Vec<Deal>>(text) {
                Ok(deals) => {
                    self.state.trade_state.update_opened_deals(deals).await;
                }
                Err(e) => warn!("Failed to parse UpdateOpenedDeals (text): {:?}", e),
            },
            ExpectedMessage::UpdateClosedDeals => match serde_json::from_str::<Vec<Deal>>(text) {
                Ok(deals) => {
                    self.state
                        .trade_state
                        .update_closed_deals(deals.clone())
                        .await;
                    for deal in deals {
                        if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                            info!("Trade closed: {:?}", deal);
                            for tx in waiters {
                                let _ = tx.send(Ok(deal.clone()));
                            }
                        }
                    }
                }
                Err(e) => warn!("Failed to parse UpdateClosedDeals (text): {:?}", e),
            },
            ExpectedMessage::SuccessCloseOrder => {
                // Try parsing as CloseOrder struct first
                match serde_json::from_str::<CloseOrder>(text) {
                    Ok(close_order) => {
                        self.state
                            .trade_state
                            .update_closed_deals(close_order.deals.clone())
                            .await;
                        for deal in close_order.deals {
                            if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                info!("Trade closed: {:?}", deal);
                                for tx in waiters {
                                    let _ = tx.send(Ok(deal.clone()));
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Fallback: Try parsing as Vec<Deal> (sometimes API sends just the list)
                        match serde_json::from_str::<Vec<Deal>>(text) {
                            Ok(deals) => {
                                self.state
                                    .trade_state
                                    .update_closed_deals(deals.clone())
                                    .await;
                                for deal in deals {
                                    if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                        info!("Trade closed (fallback): {:?}", deal);
                                        for tx in waiters {
                                            let _ = tx.send(Ok(deal.clone()));
                                        }
                                    }
                                }
                            }
                            Err(e) => warn!("Failed to parse SuccessCloseOrder (text): {:?}", e),
                        }
                    }
                }
            }
            ExpectedMessage::None => {}
        }
    }
}

#[async_trait]
impl ApiModule<State> for DealsApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = DealsHandle;

    fn new(
        state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        ws_receiver: AsyncReceiver<Arc<Message>>,
        _ws_sender: AsyncSender<Message>,
    ) -> Self {
        Self {
            state,
            ws_receiver,
            command_receiver,
            _command_responder: command_responder,
            waiting_requests: HashMap::new(),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        DealsHandle {
            sender,
            _receiver: receiver,
        }
    }

    async fn run(&mut self) -> binary_options_tools_core_pre::error::CoreResult<()> {
        let mut expected = ExpectedMessage::None;
        loop {
            tokio::select! {
                Ok(msg) = self.ws_receiver.recv() => {
                    tracing::debug!("Received message: {:?}", msg);
                    match msg.as_ref() {
                        Message::Text(text) => {
                            let mut data_text = None;
                            let mut current_expected = ExpectedMessage::None;
                            if text.starts_with(UPDATE_OPENED_DEALS) {
                                current_expected = ExpectedMessage::UpdateOpenedDeals;
                                data_text = text.strip_prefix(UPDATE_OPENED_DEALS);
                            } else if text.starts_with(UPDATE_CLOSED_DEALS) {
                                current_expected = ExpectedMessage::UpdateClosedDeals;
                                data_text = text.strip_prefix(UPDATE_CLOSED_DEALS);
                            } else if text.starts_with(SUCCESS_CLOSE_ORDER) {
                                current_expected = ExpectedMessage::SuccessCloseOrder;
                                data_text = text.strip_prefix(SUCCESS_CLOSE_ORDER);
                            }

                            if let Some(data) = data_text {
                                let trimmed = data.trim();

                                // Socket.IO 4.x binary placeholder check
                                if trimmed.contains(r#""_placeholder":true"#) {
                                    tracing::debug!(target: "DealsApiModule", "Detected binary placeholder, waiting for binary payload for {:?}", current_expected);
                                    expected = current_expected;
                                    continue;
                                }

                                if !trimmed.is_empty() && trimmed != "]" && trimmed != ",]" {
                                    // It's a 1-step message, process the data now
                                    let json_data = trimmed.strip_suffix(']').unwrap_or(trimmed);
                                    self.process_text_data(json_data, current_expected).await;
                                    expected = ExpectedMessage::None;
                                    continue;
                                } else {
                                    // Header-only, wait for data
                                    expected = current_expected;
                                    continue;
                                }
                            }

                            if expected != ExpectedMessage::None {
                                // Handle data as text if expected is set and this is not a header
                                self.process_text_data(text, expected).await;
                                expected = ExpectedMessage::None;
                            }
                        },
                        Message::Binary(data) => {
                            // Handle binary messages
                            match expected {
                                ExpectedMessage::UpdateOpenedDeals => {
                                    match serde_json::from_slice::<Vec<Deal>>(data) {
                                        Ok(deals) => {
                                            self.state.trade_state.update_opened_deals(deals).await;
                                        },
                                        Err(e) => warn!("Failed to parse UpdateOpenedDeals (binary): {:?}", e),
                                    }
                                }
                                ExpectedMessage::UpdateClosedDeals => {
                                    match serde_json::from_slice::<Vec<Deal>>(data) {
                                        Ok(deals) => {
                                            self.state.trade_state.update_closed_deals(deals.clone()).await;
                                            for deal in deals {
                                                if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                                    info!("Trade closed: {:?}", deal);
                                                    for tx in waiters {
                                                        let _ = tx.send(Ok(deal.clone()));
                                                    }
                                                }
                                            }
                                        },
                                        Err(e) => warn!("Failed to parse UpdateClosedDeals (binary): {:?}", e),
                                    }
                                }
                                ExpectedMessage::SuccessCloseOrder => {
                                    match serde_json::from_slice::<CloseOrder>(data) {
                                        Ok(close_order) => {
                                            self.state.trade_state.update_closed_deals(close_order.deals.clone()).await;
                                            for deal in close_order.deals {
                                                if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                                    info!("Trade closed: {:?}", deal);
                                                    for tx in waiters {
                                                        let _ = tx.send(Ok(deal.clone()));
                                                    }
                                                }
                                            }
                                        },
                                        Err(_) => {
                                             // Fallback: Try parsing as Vec<Deal>
                                             match serde_json::from_slice::<Vec<Deal>>(data) {
                                                Ok(deals) => {
                                                    self.state.trade_state.update_closed_deals(deals.clone()).await;
                                                    for deal in deals {
                                                        if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                                            info!("Trade closed (fallback): {:?}", deal);
                                                            for tx in waiters {
                                                                let _ = tx.send(Ok(deal.clone()));
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => warn!("Failed to parse SuccessCloseOrder (binary): {:?}", e),
                                            }
                                        }
                                    }
                                },
                                ExpectedMessage::None => {
                                    let payload_preview = if data.len() > 64 {
                                        format!("Payload ({} bytes, truncated): {:?}", data.len(), &data[..64])
                                    } else {
                                        format!("Payload ({} bytes): {:?}", data.len(), data)
                                    };
                                    warn!(target: "DealsApiModule", "Received unexpected binary message when no header was seen. {}", payload_preview);
                                }
                            }
                            expected = ExpectedMessage::None;
                        },
                        _ => {}
                    }
                }
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::CheckResult(trade_id, responder) => {
                            if self.state.trade_state.contains_opened_deal(trade_id).await {
                                // If the deal is still opened, add it to the waitlist
                                self.waiting_requests.entry(trade_id).or_default().push(responder);
                            } else if let Some(deal) = self.state.trade_state.get_closed_deal(trade_id).await {
                                // If the deal is already closed, send the result immediately
                                let _ = responder.send(Ok(deal));
                            } else {
                                // If the deal is not found, send a DealNotFound response
                                let _ = responder.send(Err(PocketError::DealNotFound(trade_id)));
                            }
                        }
                    }
                }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        // This rule will match messages like:
        // 451-["updateOpenedDeals",...]
        // 451-["updateClosedDeals",...]
        // 451-["successcloseOrder",...]

        Box::new(DealsUpdateRule::new(vec![
            UPDATE_CLOSED_DEALS,
            UPDATE_OPENED_DEALS,
            SUCCESS_CLOSE_ORDER,
        ]))
    }
}

/// Create a new custom rule that matches the specific patterns and also returns true for strings
/// that starts with any of the patterns
struct DealsUpdateRule {
    valid: AtomicBool,
    patterns: Vec<String>,
}

impl DealsUpdateRule {
    /// Create a new MultiPatternRule with the specified patterns
    ///
    /// # Arguments
    /// * `patterns` - The string patterns to match against incoming messages
    pub fn new(patterns: Vec<impl ToString>) -> Self {
        Self {
            valid: AtomicBool::new(false),
            patterns: patterns.into_iter().map(|p| p.to_string()).collect(),
        }
    }
}

impl Rule for DealsUpdateRule {
    fn call(&self, msg: &Message) -> bool {
        match msg {
            Message::Text(text) => {
                for pattern in &self.patterns {
                    if text.starts_with(pattern) {
                        let remaining = &text[pattern.len()..];
                        let trimmed_rem = remaining.trim();
                        let has_placeholder = trimmed_rem.contains(r#""_placeholder":true"#);
                        let is_header_only = trimmed_rem.is_empty()
                            || trimmed_rem == "]"
                            || trimmed_rem == ",]"
                            || has_placeholder;

                        if is_header_only {
                            self.valid.store(true, Ordering::SeqCst);
                            return true;
                        } else {
                            self.valid.store(false, Ordering::SeqCst);
                            return true;
                        }
                    }
                }

                if let Some(start) = text.find('[') {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text[start..]) {
                        if let Some(arr) = value.as_array() {
                            if arr.first().and_then(|v| v.as_str()).is_some() {
                                // It's an event, but doesn't match our pattern.
                                // Ignore it and don't consume 'valid'.
                                return false;
                            }
                        }
                    }
                }

                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    return true;
                }
                false
            }
            Message::Binary(_) => {
                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn reset(&self) {
        self.valid.store(false, Ordering::SeqCst)
    }
}
