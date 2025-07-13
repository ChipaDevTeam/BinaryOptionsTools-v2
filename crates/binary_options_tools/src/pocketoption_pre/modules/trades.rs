use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule},
};
use serde::Deserialize;
use tokio::select;
use tracing::{info, warn};
use uuid::Uuid;

use crate::pocketoption_pre::{
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
        amount: f64,
        time: u32,
        req_id: Uuid,
    },
}

/// CommandResponse enum for the `TradesApiModule`.
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
    receiver: AsyncReceiver<CommandResponse>,
}

impl TradesHandle {
    /// Places a new trade.
    pub async fn trade(
        &self,
        asset: String,
        action: Action,
        amount: f64,
        time: u32,
    ) -> PocketResult<Deal> {
        // let order = OpenOrder::new(amount, asset, action, time, demo)
        // Implement logic to create an OpenOrder and send the command.
        // 1. Send `Command::OpenOrder`.
        // 2. Await and return `CommandResponse::OpenOrder`.
        let id = Uuid::new_v4(); // Generate a unique request ID for this order
        self.sender
            .send(Command::OpenOrder {
                asset,
                action,
                amount,
                time,
                req_id: id,
            })
            .await
            .map_err(CoreError::from)?;
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::Success { req_id, deal }) => {
                    if req_id == id {
                        return Ok(*deal);
                    } else {
                        // If the request ID does not match, continue waiting for the correct response
                        continue;
                    }
                }
                Ok(CommandResponse::Error(fail)) => {
                    return Err(PocketError::FailOpenOrder {
                        error: fail.error,
                        amount: fail.amount,
                        asset: fail.asset,
                    });
                }
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }

    /// Places a new BUY trade.
    pub async fn buy(&self, asset: String, amount: f64, time: u32) -> PocketResult<Deal> {
        self.trade(asset, Action::Call, amount, time).await
    }

    /// Places a new SELL trade.
    pub async fn sell(&self, asset: String, amount: f64, time: u32) -> PocketResult<Deal> {
        self.trade(asset, Action::Put, amount, time).await
    }
}

/// The API module for handling all trade-related operations.
pub struct TradesApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
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
    ) -> Self {
        Self {
            state: shared_state,
            command_receiver,
            command_responder,
            message_receiver,
            to_ws_sender,
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        TradesHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        // TODO: Implement the main run loop.
        // This loop should handle both incoming commands from the handle
        // and incoming WebSocket messages for trade responses.
        //
        loop {
            select! {
              Ok(cmd) = self.command_receiver.recv() => {
                  match cmd {
                      Command::OpenOrder { asset, action, amount, time, req_id } => {
                      // Create OpenOrder and send to WebSocket.
                      let order = OpenOrder::new(amount, asset, action, time, self.state.is_demo() as u32, req_id);
                      self.to_ws_sender.send(Message::text(order.to_string())).await?;
                      }
                  }
                // Handle OpenOrder: send to websocket.
                // Handle CheckResult: check state, maybe wait for update.
              },
              Ok(msg) = self.message_receiver.recv() => {
                  if let Message::Binary(data) = &*msg {
                      // Parse the message as a server response.
                      if let Ok(response) = serde_json::from_slice::<ServerResponse>(data) {
                          match response {
                              ServerResponse::Success(deal) => {
                                  // Handle successopenOrder.
                                  // Send CommandResponse::Success to command_responder.
                                  self.state.trade_state.add_opened_deal(*deal.clone()).await;
                                  info!(target: "TradesApiModule", "Trade opened: {}", deal.id);
                                  self.command_responder.send(CommandResponse::Success {
                                      req_id: deal.request_id.unwrap_or_default(), // A request should always have a request_id, only for when returning updateOpenedDeals or updateClosedDeals it can not have any
                                      deal,
                                  }).await?;
                              }
                              ServerResponse::Fail(fail) => {
                                  // Handle failopenOrder.
                                  // Send CommandResponse::Error to command_responder.
                                  self.command_responder.send(CommandResponse::Error(fail)).await?;
                              }
                          }
                      } else {
                          // Handle other messages or errors.
                          warn!(target: "TradesApiModule", "Received unrecognized message: {:?}", msg);
                      }
                  }
                // Handle successopenOrder/failopenOrder.
                // Find the corresponding pending request and send response via command_responder.
              }
            }
        }
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        // This rule will match messages like:
        // 451-["successopenOrder",...]
        // 451-["failopenOrder",...]
        Box::new(MultiPatternRule::new(vec![
            "451-[\"successopenOrder\"",
            "451-[\"failopenOrder\"",
        ]))
    }
}
