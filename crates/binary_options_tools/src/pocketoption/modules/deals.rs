use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::CoreError,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule, RunnerCommand},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::oneshot;
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
    types::{Deal, MultiPatternRule},
};

const EV_UPDATE_OPENED_DEALS: &str = "updateOpenedDeals";
const EV_UPDATE_CLOSED_DEALS: &str = "updateClosedDeals";
const EV_SUCCESS_CLOSE_ORDER: &str = "successcloseOrder";

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
        _: AsyncSender<RunnerCommand>,
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
                biased;
                msg_res = self.ws_receiver.recv() => {
                    match msg_res {
                        Ok(msg) => {
                            tracing::debug!("Received message: {:?}", msg);
                            match msg.as_ref() {
                                Message::Text(text) => {
                                    let mut current_expected = ExpectedMessage::None;

                                    if let Some(start) = text.find('[') {
                                        if let Ok(serde_json::Value::Array(arr)) =
                                            serde_json::from_str::<serde_json::Value>(&text[start..])
                                        {
                                            if arr.len() >= 1 {
                                                if let Some(event_name) = arr[0].as_str() {
                                                    if event_name == EV_UPDATE_OPENED_DEALS {
                                                        current_expected = ExpectedMessage::UpdateOpenedDeals;
                                                    } else if event_name == EV_UPDATE_CLOSED_DEALS {
                                                        current_expected = ExpectedMessage::UpdateClosedDeals;
                                                    } else if event_name == EV_SUCCESS_CLOSE_ORDER {
                                                        current_expected = ExpectedMessage::SuccessCloseOrder;
                                                    }

                                                    if current_expected != ExpectedMessage::None {
                                                        if arr.len() >= 2 {
                                                            // 1-step message
                                                            if let Ok(data) = serde_json::to_string(&arr[1]) {
                                                                self.process_text_data(&data, current_expected).await;
                                                                expected = ExpectedMessage::None;
                                                                continue;
                                                            }
                                                        } else {
                                                            // Check for binary placeholder in the whole array if it's not 1-step
                                                            let has_placeholder = arr.iter().skip(1).any(|v| {
                                                                v.as_object().is_some_and(|obj| obj.contains_key("_placeholder"))
                                                            });
                                                            
                                                            if has_placeholder || arr.len() == 1 {
                                                                tracing::debug!(target: "DealsApiModule", "Detected binary placeholder, waiting for binary payload for {:?}", current_expected);
                                                                expected = current_expected;
                                                                continue;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
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
                        Err(_) => break,
                    }
                }
                cmd_res = self.command_receiver.recv() => {
                    match cmd_res {
                        Ok(cmd) => {
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
                        Err(_) => break,
                    }
                }
            }
        }
        Ok(())
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        // This rule will match messages like:
        // 451-["updateOpenedDeals",...]
        // 451-["updateClosedDeals",...]
        // 451-["successcloseOrder",...]

        Box::new(MultiPatternRule::new(vec![
            EV_UPDATE_CLOSED_DEALS,
            EV_UPDATE_OPENED_DEALS,
            EV_SUCCESS_CLOSE_ORDER,
        ]))
    }
}
