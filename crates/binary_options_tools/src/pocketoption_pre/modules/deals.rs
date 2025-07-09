use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::CoreError, reimports::{AsyncReceiver, AsyncSender, Message}, traits::{ApiModule, Rule}
};
use uuid::Uuid;

use crate::pocketoption_pre::{error::PocketResult, state::State, types::{Deal, MultiPatternRule}};


#[derive(Debug)]
pub enum Command {
    CheckResult(Uuid),
}

#[derive(Debug)]
pub enum CommandResponse {
    CheckResult(PocketResult<Deal>),
}

#[derive(Clone)]
pub struct DealsHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
}

impl DealsHandle {
    pub async fn check_result(&self, trade_id: Uuid) -> PocketResult<Deal> {
        self.sender.send(Command::CheckResult(trade_id)).await.map_err(CoreError::from)?;
        match self.receiver.recv().await.map_err(CoreError::from)? {
            CommandResponse::CheckResult(result) => result,
        }
    }
}

/// An API module responsible for listening to deal updates,
/// maintaining the shared `TradeState`, and checking trade results.
pub struct DealsUpdateModule {
    state: Arc<State>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
}

#[async_trait]
impl ApiModule<State> for DealsUpdateModule {
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
        Ok(())
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        // This rule will match messages like:
        // 451-["updateOpenedDeals",...]
        // 451-["updateClosedDeals",...]
        // 451-["successcloseOrder",...]
        Box::new(MultiPatternRule::new(vec![
            r#"451-\["updateOpenedDeals"#,
            r#"451-\["updateClosedDeals"#,
            r#"451-\["successcloseOrder"#,
        ])) 
    }
}
