use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::CoreError,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
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

enum ExpectedMessage {
    UpdateClosedDeals,
    UpdateOpenedDeals,
    SuccessCloseOrder,
    None,
}

#[derive(Deserialize)]
struct CloseOrder {
    #[serde(rename = "profit")]
    _profit: f64,
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
        DealsHandle { sender, _receiver: receiver }
    }

    async fn run(&mut self) -> binary_options_tools_core_pre::error::CoreResult<()> {
        let mut expected = ExpectedMessage::None;
        loop {
            tokio::select! {
                Ok(msg) = self.ws_receiver.recv() => {
                    tracing::debug!("Received message: {:?}", msg);
                    match msg.as_ref() {
                        Message::Text(text) => {
                            if text.starts_with(UPDATE_OPENED_DEALS) {
                                expected = ExpectedMessage::UpdateOpenedDeals;
                            } else if text.starts_with(UPDATE_CLOSED_DEALS) {
                                expected = ExpectedMessage::UpdateClosedDeals;
                            } else if text.starts_with(SUCCESS_CLOSE_ORDER) {
                                expected = ExpectedMessage::SuccessCloseOrder;
                            }
                        },
                        Message::Binary(data) => {
                            // Handle binary messages if needed
                            match expected {
                                ExpectedMessage::UpdateOpenedDeals => {
                                    // Handle UpdateOpenedDeals
                                    match serde_json::from_slice::<Vec<Deal>>(data) {
                                        Ok(deals) => {
                                            self.state.trade_state.update_opened_deals(deals).await;
                                        },
                                        Err(e) => return Err(CoreError::from(e)),
                                    }
                                }
                                ExpectedMessage::UpdateClosedDeals => {
                                    // Handle UpdateClosedDeals
                                    match serde_json::from_slice::<Vec<Deal>>(data) {
                                        Ok(deals) => {
                                            self.state.trade_state.update_closed_deals(deals.clone()).await;

                                            // Check if some trades of the waiting_requests are now closed
                                            for deal in deals {
                                                if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                                    info!("Trade closed: {:?}", deal);
                                                    for tx in waiters {
                                                        let _ = tx.send(Ok(deal.clone()));
                                                    }
                                                }
                                            }
                                        },
                                        Err(e) => return Err(CoreError::from(e)),
                                    }
                                }
                                ExpectedMessage::SuccessCloseOrder => {
                                    // Handle SuccessCloseOrder
                                    match serde_json::from_slice::<CloseOrder>(data) {
                                        Ok(close_order) => {
                                            self.state.trade_state.update_closed_deals(close_order.deals.clone()).await;

                                            // Check if some trades of the waiting_requests are now closed
                                            for deal in close_order.deals {
                                                if let Some(waiters) = self.waiting_requests.remove(&deal.id) {
                                                    info!("Trade closed: {:?}", deal);
                                                    for tx in waiters {
                                                        let _ = tx.send(Ok(deal.clone()));
                                                    }
                                                }
                                            }
                                        },
                                        Err(e) => return Err(CoreError::from(e)),
                                    }
                                },
                                ExpectedMessage::None => {
                                    let payload_preview = if data.len() > 64 {
                                        format!(
                                            "Payload ({} bytes, truncated): {:?}",
                                            data.len(),
                                            &data[..64]
                                        )
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
                        self.valid.store(true, Ordering::SeqCst);
                        return true;
                    }
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
