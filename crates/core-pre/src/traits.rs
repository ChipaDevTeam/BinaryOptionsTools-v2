use async_trait::async_trait;
use kanal::{AsyncReceiver, AsyncSender};
use std::fmt::Debug;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

use crate::error::CoreResult;

/// The contract for the application's shared state.
pub trait AppState: Send + Sync + 'static {
    /// Clears any temporary data from the state, called on a manual disconnect.
    fn clear_temporal_data(&self);
}

impl AppState for () {
    fn clear_temporal_data(&self) {
        // Default implementation does nothing.
    }
}

/// The contract for a self-contained, concurrent API module.
/// Generic over the `AppState` for type-safe access to shared data.
#[async_trait]
pub trait ApiModule<S: AppState>: Send + 'static {
    /// The specific command type this module accepts.
    type Command: Debug + Send;
    /// This specific CommandResponse type this module produces.
    type CommandResponse: Debug + Send;
    /// The handle that users will interact with. It must be clonable.
    type Handle: Clone + Send + Sync + 'static;

    /// Creates a new instance of the module.
    fn new(
        shared_state: Arc<S>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
    ) -> Self
    where
        Self: Sized;

    /// Creates a new handle for this module.
    /// This is used to send commands to the module.
    ///
    /// # Arguments
    /// * `sender`: The sender channel for commands.
    /// * `receiver`: The receiver channel for command responses.
    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle;

    fn new_combined(
        shared_state: Arc<S>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::Command>,
        command_response_receiver: AsyncReceiver<Self::CommandResponse>,
        command_response_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
    ) -> (Self, Self::Handle)
    where
        Self: Sized,
    {
        let module = Self::new(
            shared_state,
            command_receiver,
            command_response_responder,
            message_receiver,
            to_ws_sender,
        );
        let handle = Self::create_handle(command_responder, command_response_receiver);
        (module, handle)
    }

    /// The main run loop for the module's background task.
    async fn run(&mut self) -> CoreResult<()>;

    /// A function that determines if an incoming WebSocket message is intended for this module. It is very important
    /// that this function is efficient, as it will be called for every incoming message.
    /// # Arguments
    /// * `message`: The incoming WebSocket message to check.
    /// # Returns
    /// * `true` if the message is intended for this module, `false` otherwise.
    /// # Note
    /// This function should be as efficient as possible, as it will be called for
    /// every incoming WebSocket message. It should not perform any blocking operations.
    /// It is recommended to use pattern matching or other efficient checks to determine if the message is
    /// intended for this module.
    /// It is not intended to make sure the message is one of the expected types, but rather to check if it is
    /// routed to this module.
    fn routing_rule(message: &Message) -> bool;
}

/// A self‐contained module that runs independently,
/// owns its recv/sender channels and shared state,
/// and processes incoming WS messages according to its routing rule.
/// It's main difference from `ApiModule` is that it does not
/// require a command-response mechanism and is not intended to be used
/// as a part of the API, but rather as a lightweight module that can
/// process messages in a more flexible way.
/// It is useful for modules that need to handle messages without the overhead of a full API module
/// and can be used for tasks like logging, monitoring, or simple message transformations.
/// It is designed to be lightweight and efficient, allowing for quick processing of messages
/// without the need for a full command-response cycle.
/// It is also useful for modules that need to handle messages in a more flexible way,
/// such as forwarding messages to other parts of the system or performing simple transformations.
/// It is not intended to be used as a part of the API, but rather as a
/// lightweight module that can process messages in a more flexible way.
///
/// The main difference from the `LightweightHandler` type is that this trait is intended for
/// modules that need to manage their own state and processing logic and being run in a dedicated task.,
/// allowing easy automation of things like sending periodic messages to a websocket connection to keep it alive.
#[async_trait]
pub trait LightweightModule<S: AppState>: Send + 'static {
    /// Construct the module with:
    /// - shared app state
    /// - a sender for outgoing WS messages
    /// - a receiver for incoming WS messages
    fn new(
        state: Arc<S>,
        ws_sender: AsyncSender<Message>,
        ws_receiver: AsyncReceiver<Arc<Message>>,
    ) -> Self
    where
        Self: Sized;

    /// The module's asynchronous run loop.
    async fn run(&mut self) -> CoreResult<()>;

    /// Route only messages for which this returns true.
    fn routing_rule(msg: &Message) -> bool;
}
