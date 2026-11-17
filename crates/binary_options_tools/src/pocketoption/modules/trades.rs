use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use binary_options_tools_core::{
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
    types::{Action, Deal, FailOpenOrder, MultiPatternRule, OpenOrder, RequestId},
    utils::SocketIoFrame,
};

/// How long a caller waits for its order to be answered.
const TRADE_TIMEOUT: Duration = Duration::from_secs(60);

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
    /// Forget a request whose caller has stopped waiting for it.
    ///
    /// The caller is gone, so the entry must not answer it later; leaving it
    /// behind would also skew failure correlation for the orders still in flight.
    Cancel { req_id: Uuid },
}

/// Response channel for the `TradesApiModule`.
///
/// Every order is answered through the `oneshot` carried by
/// [`Command::OpenOrder`], so this type only exists to satisfy
/// [`ApiModule`]; nothing is ever sent on it.
#[derive(Debug)]
pub enum CommandResponse {}

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
        self.trade_with_id(asset, action, amount, time, Uuid::new_v4())
            .await
    }

    /// Places a new trade with a specific request ID.
    pub async fn trade_with_id(
        &self,
        asset: String,
        action: Action,
        amount: Decimal,
        time: u32,
        req_id: Uuid,
    ) -> PocketResult<Deal> {
        let (tx, rx) = oneshot::channel();
        let asset_for_error = asset.clone();

        self.sender
            .send(Command::OpenOrder {
                asset,
                action,
                amount,
                time,
                req_id,
                responder: tx,
            })
            .await
            .map_err(CoreError::from)?;

        // A dropped `failopenOrder` used to wedge the caller forever: the oneshot
        // only resolved when the module itself died. Give the wait a deadline and
        // tell the module to forget the request, so nothing answers a caller that
        // has already given up.
        match tokio::time::timeout(TRADE_TIMEOUT, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(PocketError::General(
                "TradesApiModule responder dropped".into(),
            )),
            Err(_) => {
                let _ = self.sender.send(Command::Cancel { req_id }).await;
                Err(PocketError::Timeout {
                    task: "trade_with_id".to_string(),
                    context: format!("{} {} {:?}", asset_for_error, amount, action),
                    duration: TRADE_TIMEOUT,
                })
            }
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
    /// In-flight orders keyed by `(asset, amount)`, oldest first.
    ///
    /// `failopenOrder` carries no `request_id`, so the only defensible
    /// correlation is send order: the broker answers in the order it received.
    /// Keying every request separately and then picking an arbitrary entry (as
    /// this used to) could hand a failure to a caller whose own order was fine.
    failure_matching: HashMap<(String, Decimal), VecDeque<Uuid>>,
}

impl TradesApiModule {
    /// Forgets `req_id` as an in-flight order, leaving the rest of the queue.
    fn forget_in_flight(&mut self, asset: &str, amount: Decimal, req_id: Uuid) {
        let key = (asset.to_string(), amount);
        if let Some(queue) = self.failure_matching.get_mut(&key) {
            if let Some(pos) = queue.iter().position(|id| *id == req_id) {
                queue.remove(pos);
            }
            if queue.is_empty() {
                self.failure_matching.remove(&key);
            }
        }
    }

    fn notify_waiters_module_stopped(&mut self) {
        let pending = std::mem::take(&mut self.pending_orders);
        if !pending.is_empty() {
            tracing::info!(
                "TradesApiModule: Notifying {} pending waiters that module has stopped",
                pending.len()
            );
        }
        for (req_id, tracker) in pending {
            let error = PocketError::ModuleStopped {
                module_name: "TradesApiModule".to_string(),
                context: format!("Request ID: {}", req_id),
            };
            let _ = tracker.responder.send(Err(error));
        }
    }
}

impl Drop for TradesApiModule {
    fn drop(&mut self) {
        self.notify_waiters_module_stopped();
    }
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
              cmd_res = self.command_receiver.recv() => {
                  match cmd_res {
                      Ok(Command::OpenOrder { asset, action, amount, time, req_id, responder }) => {
                          // Register pending order
                          let tracker = PendingOrderTracker {
                              asset: asset.clone(),
                              amount,
                              responder,
                          };
                          self.pending_orders.insert(req_id, tracker);

                          // Track send order: this is the only thing `failopenOrder`
                          // can be matched against.
                          let key = (asset.clone(), amount);
                          self.failure_matching.entry(key).or_default().push_back(req_id);

                          // Create OpenOrder and send to WebSocket.
                          let asset_for_error = asset.clone();
                          let order = OpenOrder::new(amount, asset, action, time, self.state.is_demo() as u32, req_id);
                          if let Err(e) = self.to_ws_sender.send(Message::text(order.to_string())).await {
                              if let Some(tracker) = self.pending_orders.remove(&req_id) {
                                  let _ = tracker.responder.send(Err(CoreError::from(e).into()));
                              }
                              self.forget_in_flight(&asset_for_error, amount, req_id);
                          }
                      }
                      Ok(Command::Cancel { req_id }) => {
                          // The caller stopped waiting: drop the tracker (which
                          // closes its responder) and every index entry for it.
                          let tracker = self.pending_orders.remove(&req_id);
                          if let Some(tracker) = tracker {
                              self.forget_in_flight(&tracker.asset, tracker.amount, req_id);
                          }
                          self.state
                              .trade_state
                              .pending_market_orders
                              .write()
                              .await
                              .remove(&req_id);
                      }
                      Err(_) => {
                          self.notify_waiters_module_stopped();
                          return Ok(());
                      }
                  }
              },
              msg_res = self.message_receiver.recv() => {
                  let msg = match msg_res {
                      Ok(msg) => msg,
                      Err(_) => {
                          self.notify_waiters_module_stopped();
                          return Ok(());
                      }
                  };
                  let response_result = match msg.as_ref() {
                      Message::Binary(data) => match serde_json::from_slice::<ServerResponse>(data) {
                          Ok(res) => Ok(res),
                          Err(e) => {
                              warn!(target: "TradesApiModule", "Failed to parse binary ServerResponse: {}", e);
                              Err(e)
                          }
                      },
                      Message::Text(text) => {
                          if let Ok(res) = serde_json::from_str::<ServerResponse>(text) {
                              Ok(res)
                          } else if let Some(frame) = SocketIoFrame::parse(text) {
                              if let Some((event, payload)) = frame.extract_event() {
                                  if event == "successopenOrder" || event == "failopenOrder" {
                                      serde_json::from_value::<ServerResponse>(payload)
                                  } else {
                                      serde_json::from_str::<ServerResponse>(text)
                                  }
                              } else {
                                  serde_json::from_str::<ServerResponse>(text)
                              }
                          } else {
                              serde_json::from_str::<ServerResponse>(text)
                          }
                      },
                      _ => {
                          warn!(target: "TradesApiModule", "Received unexpected message type: {:?}", msg);
                          continue;
                      }
                  };

                  if let Ok(response) = response_result {
                      match response {
                          ServerResponse::Success(deal) => {
                              self.state.trade_state.add_opened_deal(*deal.clone()).await;
                              info!(target: "TradesApiModule", "Trade opened: {}", deal.id);

                              let req_id = match deal.request_id.as_ref() {
                                  Some(RequestId::Uuid(id)) => Some(*id),
                                  Some(RequestId::Number(_)) | None => None,
                              };

                              // Clean up pending_market_orders in state and notify responder
                              if let Some(id) = req_id {
                                  self.state.trade_state.pending_market_orders.write().await.remove(&id);

                                  if let Some(tracker) = self.pending_orders.remove(&id) {
                                      let _ = tracker.responder.send(Ok(*deal.clone()));

                                      // This order is answered; it must no longer
                                      // absorb a later failure meant for another.
                                      self.forget_in_flight(&tracker.asset, tracker.amount, id);
                                  } else {
                                      warn!(target: "TradesApiModule", "Received success for unknown request ID: {}", id);
                                  }
                              } else {
                                  warn!(target: "TradesApiModule", "Could not correlate successopenOrder for {} {}", deal.asset, deal.amount);
                              }
                          }
                          ServerResponse::Fail(fail) => {
                              let asset = fail.asset.clone();
                              let amount = fail.amount;

                              // The oldest still-unanswered order for this
                              // (asset, amount) is the one the broker is
                              // reporting: it answers in the order it received.
                              let key = (asset.clone(), amount);
                              let found_req_id = self
                                  .failure_matching
                                  .get_mut(&key)
                                  .and_then(|queue| queue.pop_front());
                              if self
                                  .failure_matching
                                  .get(&key)
                                  .is_some_and(|queue| queue.is_empty())
                              {
                                  self.failure_matching.remove(&key);
                              }

                              if let Some(req_id) = found_req_id {

                                  // Clean up pending_market_orders in state
                                  self.state.trade_state.pending_market_orders.write().await.remove(&req_id);

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
