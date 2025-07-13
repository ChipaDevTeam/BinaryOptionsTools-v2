use async_trait::async_trait;
use kanal::{AsyncReceiver, AsyncSender};
use std::fmt::Debug;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

use crate::error::CoreResult;

/// The contract for the application's shared state.
#[async_trait]
pub trait AppState: Send + Sync + 'static {
    /// Clears any temporary data from the state, called on a manual disconnect.
    async fn clear_temporal_data(&self);
}

#[async_trait]
impl AppState for () {
    async fn clear_temporal_data(&self) {
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

    /// An optional callback that can be executed when a reconnection event occurs.
    /// This function is useful for modules that need to perform specific actions
    /// when a reconnection happens, such as reinitializing state or resending messages.
    /// It allows for custom behavior to be defined that can be executed in the context of the
    /// module, providing flexibility and extensibility to the module's functionality.
    fn callback(&self) -> CoreResult<Option<Box<dyn ReconnectCallback<S>>>> {
        // Default implementation does nothing.
        // This is useful for modules that do not require a callback.
        Ok(None)
    }

    /// Route only messages for which this returns true.
    /// This function is used to determine whether a message should be processed by this module.
    /// It allows for flexible and reusable rules that can be applied to different modules.
    fn rule() -> Box<dyn Rule + Send + Sync>;
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
    fn rule() -> Box<dyn Rule + Send + Sync>;
}

/// Data returned by the rule function of a module.
/// This trait is used to define the rules that determine whether a message should be processed by a module.
/// It allows for flexible and reusable rules that can be applied to different modules.
/// The rules can be implemented as standalone functions or as methods on the module itself.
/// The rules should be lightweight and efficient, as they will be called for every incoming message.
/// The rules should not perform any blocking operations and should be designed to be as efficient as possible
/// to avoid slowing down the message processing pipeline.
/// The rules can be used to filter messages, transform them, or perform any other necessary operations
pub trait Rule {
    /// Validate wherever the messsage follows the rule and needs to be processed by this module.
    fn call(&self, msg: &Message) -> bool;

    /// Resets the rule to its initial state.
    /// This is useful for rules that maintain state and need to be reset
    /// when the module is reset or reinitialized.
    /// Implementations should ensure that the rule is in a clean state after this call.
    /// # Note
    /// This method is not required to be asynchronous, as it is expected to be a lightweight
    /// operation that does not involve any I/O or long-running tasks.
    /// It should be implemented in a way that allows the rule to be reused without
    /// needing to recreate it, thus improving performance and reducing overhead.
    fn reset(&self);
}

/// A trait for callback functions that can be executed within the context of a module.
/// This trait is designed to allow modules to define custom behavior that can be executed
/// when a reconnection event occurs.
#[async_trait]
pub trait ReconnectCallback<S: AppState>: Send + Sync {
    /// The asynchronous function that will be called when a reconnection event occurs.
    /// This function receives the shared state and a sender for outgoing WebSocket messages.
    /// It should return a `CoreResult<()>` indicating the success or failure of the operation.
    /// /// # Arguments
    /// * `state`: The shared application state that the callback can use.
    /// * `ws_sender`: The sender for outgoing WebSocket messages, allowing the callback to
    ///   send messages to the WebSocket connection.
    /// # Returns
    /// A `CoreResult<()>` indicating the success or failure of the operation.
    /// # Note
    /// This function is expected to be asynchronous, allowing it to perform I/O operations
    /// or other tasks that may take time without blocking the event loop.
    /// Implementations should ensure that they handle any potential errors gracefully
    /// and return appropriate results.
    /// It is also important to ensure that the callback does not block the event loop,
    /// as this could lead to performance issues in the application.
    /// Implementations should be designed to be efficient and non-blocking,
    /// allowing the application to continue processing other events while the callback is executed.
    /// This trait is useful for defining custom behavior that can be executed when a reconnection event
    /// occurs, allowing modules to handle reconnections in a flexible and reusable way.    
    async fn call(&self, state: Arc<S>, ws_sender: &AsyncSender<Message>) -> CoreResult<()>;
}

impl<F> Rule for F
where
    F: Fn(&Message) -> bool + Send + Sync + 'static,
{
    fn call(&self, msg: &Message) -> bool {
        self(msg)
    }

    fn reset(&self) {
        // Default implementation does nothing.
        // This is useful for stateless rules.
    }
}

#[async_trait]
impl<S: AppState> ReconnectCallback<S> for () {
    async fn call(&self, _state: Arc<S>, _ws_sender: &AsyncSender<Message>) -> CoreResult<()> {
        Ok(())
    }
}
