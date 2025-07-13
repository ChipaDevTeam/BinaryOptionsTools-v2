use std::{
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
use tracing::info;
use uuid::Uuid;

use crate::pocketoption_pre::{
    error::{PocketError, PocketResult},
    state::State,
    types::Deal,
};

const UPDATE_OPENED_DEALS: &str = r#"451-["updateOpenedDeals","#;
const UPDATE_CLOSED_DEALS: &str = r#"451-["updateClosedDeals","#;
const SUCCESS_CLOSE_ORDER: &str = r#"451-["successcloseOrder","#;

#[derive(Debug)]
pub enum Command {
    CheckResult(Uuid),
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
    receiver: AsyncReceiver<CommandResponse>,
}

impl DealsHandle {
    pub async fn check_result(&self, trade_id: Uuid) -> PocketResult<Deal> {
        self.sender
            .send(Command::CheckResult(trade_id))
            .await
            .map_err(CoreError::from)?;
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::CheckResult(deal)) => {
                    if trade_id == deal.id {
                        return Ok(*deal);
                    } else {
                        // If the request ID does not match, continue waiting for the correct response
                        continue;
                    }
                }
                Ok(CommandResponse::DealNotFound(id)) => return Err(PocketError::DealNotFound(id)),
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }

    pub async fn check_result_with_timeout(
        &self,
        trade_id: Uuid,
        timeout: Duration,
    ) -> PocketResult<Deal> {
        self.sender
            .send(Command::CheckResult(trade_id))
            .await
            .map_err(CoreError::from)?;

        let timeout_future = tokio::time::sleep(timeout);
        tokio::pin!(timeout_future);

        loop {
            tokio::select! {
                result = self.receiver.recv() => {
                    match result {
                        Ok(CommandResponse::CheckResult(deal)) => {
                            if trade_id == deal.id {
                                return Ok(*deal);
                            } else {
                                // If the request ID does not match, continue waiting for the correct response
                                continue;
                            }
                        },
                        Ok(CommandResponse::DealNotFound(id)) => return Err(PocketError::DealNotFound(id)),
                        Err(e) => return Err(CoreError::from(e).into()),
                    }
                }
                _ = &mut timeout_future => {
                    return Err(PocketError::Timeout {
                        task: "check_result".to_string(),
                        context: format!("Waiting for trade '{trade_id}' result"),
                        duration: timeout,
                    });
                }
            }
        }
    }
}

/// An API module responsible for listening to deal updates,
/// maintaining the shared `TradeState`, and checking trade results.
pub struct DealsApiModule {
    state: Arc<State>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    waitlist: Vec<Uuid>,
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
            command_responder,
            waitlist: Vec::new(),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        DealsHandle { sender, receiver }
    }

    async fn run(&mut self) -> binary_options_tools_core_pre::error::CoreResult<()> {
        // TODO: Implement the run loop.
        // 1. Use tokio::select! to listen on both `ws_receiver` and `command_receiver`.
        // 2. For WebSocket messages:
        //    - Deserialize into `UpdateOpenedDeals`, `UpdateClosedDeals`, or `SuccessCloseOrder`.
        //    - Call the appropriate methods on `self.state.trade_state` to update the state.
        // 3. For `CheckResult` commands:
        //    - Implement the logic described in README.md to wait for the deal to close.
        //    - Send the result back via `command_responder`.
        let mut expected = ExpectedMessage::None;
        loop {
            tokio::select! {
                Ok(msg) = self.ws_receiver.recv() => {
                    info!("Received message: {:?}", msg);
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
                                            self.state.trade_state.update_closed_deals(deals).await;
                                            // Check if some trades of the waitlist are now closed
                                            let mut remove = Vec::new();
                                            for id in &self.waitlist {
                                                if let Some(deal) = self.state.trade_state.get_closed_deal(*id).await {
                                                    info!("Trade closed: {:?}", deal);
                                                    self.command_responder.send(CommandResponse::CheckResult(Box::new(deal))).await?;
                                                    remove.push(*id);
                                                }
                                            }
                                            self.waitlist.retain(|id| !remove.contains(id));
                                        },
                                        Err(e) => return Err(CoreError::from(e)),
                                    }
                                }
                                ExpectedMessage::SuccessCloseOrder => {
                                    // Handle SuccessCloseOrder
                                    match serde_json::from_slice::<CloseOrder>(data) {
                                        Ok(close_order) => {
                                            self.state.trade_state.update_closed_deals(close_order.deals).await;
                                            // Check if some trades of the waitlist are now closed
                                            let mut remove = Vec::new();
                                            for id in &self.waitlist {
                                                if let Some(deal) = self.state.trade_state.get_closed_deal(*id).await {
                                                    info!("Trade closed: {:?}", deal);
                                                    self.command_responder.send(CommandResponse::CheckResult(Box::new(deal))).await?;
                                                    remove.push(*id);
                                                }
                                            }
                                            self.waitlist.retain(|id| !remove.contains(id));

                                        },
                                        Err(e) => return Err(CoreError::from(e)),
                                    }
                                },
                                _ => {}
                            }
                            expected = ExpectedMessage::None;
                        },
                        _ => {}
                    }

                }
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::CheckResult(trade_id) => {
                            if self.state.trade_state.contains_opened_deal(trade_id).await {
                                // If the deal is still opened, add it to the waitlist
                                self.waitlist.push(trade_id);
                            } else if let Some(deal) = self.state.trade_state.get_closed_deal(trade_id).await {
                                // If the deal is already closed, send the result immediately
                                self.command_responder.send(CommandResponse::CheckResult(Box::new(deal))).await?;
                            } else {
                                // If the deal is not found, send a DealNotFound response
                                self.command_responder.send(CommandResponse::DealNotFound(trade_id)).await?;
                            }
                            // Implement logic to check the result of a trade
                            // For example, wait for the deal to close and return the result
                        }
                    }
                }
            }
        }
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
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
