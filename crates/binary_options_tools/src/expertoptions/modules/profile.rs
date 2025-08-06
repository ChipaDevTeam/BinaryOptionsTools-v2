use crate::expertoptions::modules::Command;
use crate::expertoptions::types::MultiRule;
use crate::utils::serialize::bool2int;

use std::sync::Arc;

use binary_options_tools_core_pre::error::CoreResult;
use binary_options_tools_core_pre::reimports::{AsyncReceiver, AsyncSender, Message};
use binary_options_tools_core_pre::traits::{ApiModule, Rule};
use binary_options_tools_macros::ActionImpl;
use serde::{Deserialize, Serialize};
use tokio::select;
use tracing::warn;

use crate::expertoptions::{Action, ActionName};
use crate::expertoptions::state::State;

#[derive(Debug)]
pub enum Request {
    SetContext(Demo)
}

#[derive(Debug)]
pub enum Response {
    Success,
    Error(String)
}

#[derive(Clone)]
pub struct ProfileHandle {
    sender: AsyncSender<Command<Request>>,
    receiver: AsyncReceiver<Command<Response>>,
}
/// Profile module for maintaining session activity
/// Send the original connection messages, and handles changes from real to demo accounts
pub struct ProfileModule {
    ws_receiver: AsyncReceiver<Arc<Message>>,
    ws_sender: AsyncSender<Message>,
    command_receiver: AsyncReceiver<Command<Request>>,
    command_responder: AsyncSender<Command<Response>>,
    /// The current state of the module
    state: Arc<State>,
}

#[derive(Debug, Serialize, Deserialize, ActionImpl)]
#[action(name = "setContext")]
pub struct Demo {
    #[serde(with = "bool2int")]
    is_demo: bool
}

#[derive(Deserialize)]
struct Res {
    result: String
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ProfileResponse {
    Change(Res)
}

impl Demo {
    pub fn new(is_demo: bool) -> Self {
        Demo { is_demo }
    }

    pub fn to_demo(self) -> Self {
        Demo { is_demo: true }
    }

    pub fn to_real(self) -> Self {
        Demo { is_demo: false }
    }

    pub fn is_demo(&self) -> bool {
        self.is_demo
    }
}

#[async_trait::async_trait]
impl ApiModule<State> for ProfileModule {
    type Command = Command<Request>;
    type CommandResponse = Command<Response>;
    type Handle = ProfileHandle;


    fn new(
        shared_state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
    ) -> Self
    where
        Self: Sized {
        Self {
            ws_receiver: message_receiver,
            ws_sender: to_ws_sender,
            command_receiver,
            command_responder,
            state: shared_state,
        }
    }

    /// Creates a new handle for this module.
    /// This is used to send commands to the module.
    ///
    /// # Arguments
    /// * `sender`: The sender channel for commands.
    /// * `receiver`: The receiver channel for command responses.
    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        ProfileHandle { sender, receiver }
    }

    /// The main run loop for the module's background task.
    async fn run(&mut self) -> CoreResult<()> {
        loop {
            select! {
                Ok(msg) = self.ws_receiver.recv() => {
                    if let Message::Binary(data) = msg.as_ref() {
                        match Action::from_json::<ProfileResponse>(data) {
                            Ok(res) => {
                                
                            },
                            Err(e) => warn!(target: "ProfileModule", "Failed to parse message into a `ProfileResponse` variant, {e}")
                        }
                    }

                },
                Ok(cmd) = self.command_receiver.recv() => {
                    
                }
            }
        }
    }



    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiRule::new(vec![
            Box::new(DemoRule)
        ]))
    }
}