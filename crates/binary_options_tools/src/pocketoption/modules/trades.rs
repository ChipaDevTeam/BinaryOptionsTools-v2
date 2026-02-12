use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::Arc,
};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule, RunnerCommand},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::{select, sync::oneshot};
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::State,
    types::{Action, Deal, FailOpenOrder, MultiPatternRule, OpenOrder},
};

/// Command enum for the `TradesApiModule`.
#[derive(Debug)]
pub enum Command {
    /// Command to place a new trade.
    OpenOrder {
        asset: String,
        action: Action,
        amount: Decimal,
        time: u32,
        req_id: Uuid,
        responder: oneshot::Sender<PocketResult<Deal>>,
    },
}

/// CommandResponse enum for the `TradesApiModule`.
/// Kept for trait compatibility but mostly unused in the new oneshot pattern.
#[derive(Debug)]
pub enum CommandResponse {
    /// Response for an `OpenOrder` command.
    Success {
        req_id: Uuid,
        deal: Box<Deal>,
    },
    Error(Box<FailOpenOrder>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ServerResponse {
    Success(Box<Deal>),
    Fail(Box<FailOpenOrder>),
}

/// Handle for interacting with the `TradesApiModule`.
#[derive(Clone)]
pub struct TradesHandle {
    sender: AsyncSender<Command>,
    // Receiver is no longer needed in the handle as we use oneshot channels per request
    _receiver: AsyncReceiver<CommandResponse>,
}

impl TradesHandle {
    /// Places a new trade.
    pub async fn trade(
        &self,
        asset: String,
        action: Action,
        amount: Decimal,
        time: u32,
    ) -> PocketResult<Deal> {
        let id = Uuid::new_v4(); // Generate a unique request ID for this order
        let (tx, rx) = oneshot::channel();

        self.sender
            .send(Command::OpenOrder {
                asset,
                action,
                amount,
                time,
                req_id: id,
                responder: tx,
            })
            .await
            .map_err(CoreError::from)?;

        // Wait for the specific response for this trade
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(CoreError::Other("TradesApiModule responder dropped".into()).into()),
        }
    }

    /// Places a new BUY trade.
    pub async fn buy(&self, asset: String, amount: Decimal, time: u32) -> PocketResult<Deal> {
        self.trade(asset, Action::Call, amount, time).await
    }

    /// Places a new SELL trade.
    pub async fn sell(&self, asset: String, amount: Decimal, time: u32) -> PocketResult<Deal> {
        self.trade(asset, Action::Put, amount, time).await
    }
}

/// Internal struct to track pending orders
struct PendingOrderTracker {
    asset: String,
    amount: Decimal,
    responder: oneshot::Sender<PocketResult<Deal>>,
}

/// The API module for handling all trade-related operations.
pub struct TradesApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    _command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    pending_orders: HashMap<Uuid, PendingOrderTracker>,
    // Secondary index for matching failures (which lack UUID)
    // Map of (Asset, Amount) -> Queue of UUIDs (FIFO)
    failure_matching: HashMap<(String, Decimal), VecDeque<Uuid>>,
}

#[async_trait]
impl ApiModule<State> for TradesApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = TradesHandle;

    fn new(
        shared_state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
        Self {
            state: shared_state,
            command_receiver,
            _command_responder: command_responder,
            message_receiver,
            to_ws_sender,
            pending_orders: HashMap::new(),
            failure_matching: HashMap::new(),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        TradesHandle {
            sender,
            _receiver: receiver,
        }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
              Ok(cmd) = self.command_receiver.recv() => {
                  match cmd {
                      Command::OpenOrder { asset, action, amount, time, req_id, responder } => {
                          // Register pending order
                          let tracker = PendingOrderTracker {
                              asset: asset.clone(),
                              amount,
                              responder,
                          };
                          self.pending_orders.insert(req_id, tracker);

                          // Add to failure matching queue
                          let key = (asset.clone(), amount);
                          self.failure_matching.entry(key).or_default().push_back(req_id);

                          // Create OpenOrder and send to WebSocket.
                          let asset_for_error = asset.clone();
                          let order = OpenOrder::new(amount, asset, action, time, self.state.is_demo() as u32, req_id);
                          if let Err(e) = self.to_ws_sender.send(Message::text(order.to_string())).await {
                              if let Some(tracker) = self.pending_orders.remove(&req_id) {
                                  let _ = tracker.responder.send(Err(CoreError::from(e).into()));
                              }
                              let key = (asset_for_error, amount);
                              if let Some(queue) = self.failure_matching.get_mut(&key) {
                                  queue.retain(|&id| id != req_id);
                              }
                          }
                      }
                  }
              },
              Ok(msg) = self.message_receiver.recv() => {
                  let response_result = match msg.as_ref() {
                      Message::Binary(data) => serde_json::from_slice::<ServerResponse>(data),
                      Message::Text(text) => {
                          if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                              Ok(res)
                          } else if let Some(start) = text.find('[') {
                              // Try parsing as a 1-step Socket.IO message: 42["successopenOrder", {...}]
                              match serde_json::from_str::<serde_json::Value>(&text[start..]) {
                                  Ok(serde_json::Value::Array(arr)) => {
                                      if arr.len() >= 2 && (arr[0] == "successopenOrder" || arr[0] == "failopenOrder") {
                                          serde_json::from_value::<ServerResponse>(arr[1].clone())
                                      } else {
                                          serde_json::from_str::<ServerResponse>(text)
                                      }
                                  }
                                  _ => serde_json::from_str::<ServerResponse>(text),
                              }
                          } else {
                              serde_json::from_str::<ServerResponse>(text)
                          }
                      },
                      _ => {
                          // Ignore other message types
                          continue;
                      }
                  };

                  if let Ok(response) = response_result {
                      match response {
                          ServerResponse::Success(deal) => {
                              self.state.trade_state.add_opened_deal(*deal.clone()).await;
                              info!(target: "TradesApiModule", "Trade opened: {}", deal.id);

                              let req_id = deal.request_id.unwrap_or_default();

                              if let Some(tracker) = self.pending_orders.remove(&req_id) {
                                  let _ = tracker.responder.send(Ok(*deal.clone()));

                                  let key = (tracker.asset, tracker.amount);
                                  if let Some(queue) = self.failure_matching.get_mut(&key) {
                                      queue.retain(|&id| id != req_id);
                                  }
                              } else {
                                  warn!(target: "TradesApiModule", "Received success for unknown request ID: {}", req_id);
                              }
                          }
                          ServerResponse::Fail(fail) => {
                              let key = (fail.asset.clone(), fail.amount);

                              let found_req_id = if let Some(queue) = self.failure_matching.get_mut(&key) {
                                  queue.pop_front()
                              } else {
                                  None
                              };

                              if let Some(req_id) = found_req_id {
                                  if let Some(tracker) = self.pending_orders.remove(&req_id) {
                                      let _ = tracker.responder.send(Err(PocketError::FailOpenOrder {
                                          error: fail.error.clone(),
                                          amount: fail.amount,
                                          asset: fail.asset.clone(),
                                      }));
                                  }
                              } else {
                                   warn!(target: "TradesApiModule", "Received failure for unknown order: {} {}", fail.asset, fail.amount);
                              }
                          }
                      }
                  } else {
                      // Warn if parsing failed, but don't crash
                      warn!(target: "TradesApiModule", "Failed to parse ServerResponse from message");
                  }
              }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        // This rule will match messages like:
        // 451-["successopenOrder",...]
        // 451-["failopenOrder",...]
        Box::new(MultiPatternRule::new(vec![
            "successopenOrder",
            "failopenOrder",
        ]))
    }
}
