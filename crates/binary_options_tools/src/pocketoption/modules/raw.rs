use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core_pre::error::CoreError;
use binary_options_tools_core_pre::reimports::{
    AsyncReceiver, AsyncSender, Message, bounded_async,
};
use binary_options_tools_core_pre::traits::{ApiModule, Rule};
use tokio::select;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::pocketoption::error::PocketResult;
use crate::pocketoption::state::State;
use crate::traits::ValidatorTrait;
use crate::validator::Validator;

/// Outgoing message to WS
#[derive(Clone, Debug)]
pub enum Outgoing {
    Text(String),
    Binary(Vec<u8>),
}

/// Commands for RawApiModule
#[derive(Debug)]
pub enum Command {
    Create {
        validator: Validator,
        keep_alive: Option<Outgoing>,
        command_id: Uuid,
    },
    Remove {
        id: Uuid,
        command_id: Uuid,
    },
    Send(Outgoing),
}

/// Responses for RawApiModule
#[derive(Debug)]
pub enum CommandResponse {
    Created {
        command_id: Uuid,
        id: Uuid,
        stream_receiver: AsyncReceiver<Arc<Message>>,
    },
    Removed {
        command_id: Uuid,
        id: Uuid,
        existed: bool,
    },
}

/// Handle used by clients to create per-validator RawHandlers
#[derive(Clone)]
pub struct RawHandle {
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<CommandResponse>,
}

impl RawHandle {
    /// Create a new RawHandler bound to the given validator
    pub async fn create(
        &self,
        validator: Validator,
        keep_alive: Option<Outgoing>,
    ) -> PocketResult<RawHandler> {
        let command_id = Uuid::new_v4();
        self.sender
            .send(Command::Create {
                validator,
                keep_alive,
                command_id,
            })
            .await
            .map_err(CoreError::from)?;
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::Created {
                    command_id: cid,
                    id,
                    stream_receiver,
                }) if cid == command_id => {
                    return Ok(RawHandler {
                        id,
                        sender: self.sender.clone(),
                        receiver: stream_receiver,
                    });
                }
                Ok(_) => continue,
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }

    /// Remove an existing handler by ID
    pub async fn remove(&self, id: Uuid) -> PocketResult<bool> {
        let command_id = Uuid::new_v4();
        self.sender
            .send(Command::Remove { id, command_id })
            .await
            .map_err(CoreError::from)?;
        loop {
            match self.receiver.recv().await {
                Ok(CommandResponse::Removed {
                    command_id: cid,
                    id: rid,
                    existed,
                }) if cid == command_id && rid == id => return Ok(existed),
                Ok(_) => continue,
                Err(e) => return Err(CoreError::from(e).into()),
            }
        }
    }
}

/// Per-validator raw handler: send, wait and subscribe to messages matching its validator
pub struct RawHandler {
    id: Uuid,
    sender: AsyncSender<Command>,
    receiver: AsyncReceiver<Arc<Message>>,
}

impl RawHandler {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub async fn send_text(&self, text: impl Into<String>) -> PocketResult<()> {
        self.sender
            .send(Command::Send(Outgoing::Text(text.into())))
            .await
            .map_err(CoreError::from)?;
        Ok(())
    }

    pub async fn send_binary(&self, data: impl Into<Vec<u8>>) -> PocketResult<()> {
        self.sender
            .send(Command::Send(Outgoing::Binary(data.into())))
            .await
            .map_err(CoreError::from)?;
        Ok(())
    }

    /// Send a message and wait for the next matching response
    pub async fn send_and_wait(&self, msg: Outgoing) -> PocketResult<Arc<Message>> {
        self.sender
            .send(Command::Send(msg))
            .await
            .map_err(CoreError::from)?;
        self.wait_next().await
    }

    /// Wait for next message that matches this handler's validator
    pub async fn wait_next(&self) -> PocketResult<Arc<Message>> {
        self.receiver
            .recv()
            .await
            .map_err(CoreError::from)
            .map_err(Into::into)
    }

    /// Get a clone of the underlying stream receiver
    pub fn subscribe(&self) -> AsyncReceiver<Arc<Message>> {
        self.receiver.clone()
    }
}

impl Drop for RawHandler {
    fn drop(&mut self) {
        // best-effort removal
        let _ = self.sender.as_sync().send(Command::Remove {
            id: self.id,
            command_id: Uuid::new_v4(),
        });
    }
}

/// Main module processing and routing messages to per-validator streams
pub struct RawApiModule {
    state: Arc<State>,
    command_receiver: AsyncReceiver<Command>,
    command_responder: AsyncSender<CommandResponse>,
    message_receiver: AsyncReceiver<Arc<Message>>,
    to_ws_sender: AsyncSender<Message>,
    sinks: Arc<RwLock<HashMap<Uuid, AsyncSender<Arc<Message>>>>>,
    keep_alive_msgs: Arc<RwLock<HashMap<Uuid, Outgoing>>>,
}

pub struct RawRule {
    state: Arc<State>,
}

impl Rule for RawRule {
    fn call(&self, msg: &Message) -> bool {
        // Convert to string view for validator check
        let msg_str = match msg {
            Message::Binary(bin) => String::from_utf8_lossy(bin.as_ref()).into_owned(),
            Message::Text(text) => text.to_string(),
            _ => return false,
        };
        let validators = self.state.raw_validators.blocking_read();
        for (_id, v) in validators.iter() {
            if v.call(msg_str.as_str()) {
                return true;
            }
        }
        false
    }

    fn reset(&self) {
        // Do not clear validators on reconnect; handlers remain valid
    }
}

#[async_trait]
impl ApiModule<State> for RawApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = RawHandle;

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
            sinks: Arc::new(RwLock::new(HashMap::new())),
            keep_alive_msgs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        RawHandle { sender, receiver }
    }

    async fn run(&mut self) -> binary_options_tools_core_pre::error::CoreResult<()> {
        loop {
            select! {
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd {
                        Command::Create { validator, keep_alive, command_id } => {
                            let id = Uuid::new_v4();
                            self.state.add_raw_validator(id, validator).await;
                            if let Some(msg) = keep_alive.clone() {
                                self.keep_alive_msgs.write().await.insert(id, msg);
                            }
                            let (tx, rx) = bounded_async(64);
                            self.sinks.write().await.insert(id, tx);
                            self.command_responder.send(CommandResponse::Created { command_id, id, stream_receiver: rx }).await?;
                        }
                        Command::Remove { id, command_id } => {
                            let existed_state = self.state.remove_raw_validator(&id).await;
                            let existed_sink = self.sinks.write().await.remove(&id).is_some();
                            self.keep_alive_msgs.write().await.remove(&id);
                            self.command_responder.send(CommandResponse::Removed { command_id, id, existed: existed_state || existed_sink }).await?;
                        }
                        Command::Send(Outgoing::Text(text)) => {
                            self.to_ws_sender.send(Message::text(text)).await.map_err(CoreError::from)?;
                        }
                        Command::Send(Outgoing::Binary(data)) => {
                            self.to_ws_sender.send(Message::binary(data)).await.map_err(CoreError::from)?;
                        }
                    }
                },
                Ok(msg) = self.message_receiver.recv() => {
                    // When a message arrives, route it to all matching validators
                    let content = match msg.as_ref() {
                        Message::Binary(bin) => String::from_utf8_lossy(bin.as_ref()).into_owned(),
                        Message::Text(t) => t.to_string(),
                        _ => String::new(),
                    };
                    if content.is_empty() { continue; }
                    let validators = self.state.raw_validators.read().await.clone();
                    let sinks = self.sinks.read().await.clone();
                    for (id, validator) in validators.into_iter() {
                        if validator.call(content.as_str())
                            && let Some(tx) = sinks.get(&id) {
                                let _ = tx.send(msg.clone()).await; // best effort
                            }
                    }
                }
            }
        }
    }

    fn rule(state: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(RawRule { state })
    }

    fn callback(
        &self,
    ) -> binary_options_tools_core_pre::error::CoreResult<
        Option<Box<dyn binary_options_tools_core_pre::traits::ReconnectCallback<State>>>,
    > {
        // On reconnect, re-send any keep-alive messages configured per handler
        struct CB {
            msgs: Arc<RwLock<HashMap<Uuid, Outgoing>>>,
        }
        #[async_trait]
        impl binary_options_tools_core_pre::traits::ReconnectCallback<State> for CB {
            async fn call(
                &self,
                _state: Arc<State>,
                ws_sender: &AsyncSender<Message>,
            ) -> binary_options_tools_core_pre::error::CoreResult<()> {
                let msgs = self.msgs.read().await.clone();
                for (_id, msg) in msgs.into_iter() {
                    match msg {
                        Outgoing::Text(t) => {
                            let _ = ws_sender.send(Message::text(t)).await;
                        }
                        Outgoing::Binary(b) => {
                            let _ = ws_sender.send(Message::binary(b)).await;
                        }
                    }
                }
                Ok(())
            }
        }
        Ok(Some(Box::new(CB {
            msgs: self.keep_alive_msgs.clone(),
        })))
    }
}
