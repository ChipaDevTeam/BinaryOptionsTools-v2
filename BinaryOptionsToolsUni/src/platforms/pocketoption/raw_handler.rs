use std::sync::Arc;

use binary_options_tools::{pocketoption::modules::raw::{
    Outgoing as InnerOutgoing, RawHandler as InnerRawHandler,
}, stream::Message};
use crate::error::UniError;
use binary_options_tools::error::BinaryOptionsError;

/// Handler for advanced raw WebSocket message operations.
///
/// Provides low-level access to send messages and receive filtered responses
/// based on a validator. Each handler maintains its own message stream.
#[derive(uniffi::Object)]
pub struct RawHandler {
    inner: InnerRawHandler,
}

#[uniffi::export]
impl RawHandler {
    /// Send a text message through this handler.
    ///
    /// # Arguments
    ///
    /// * `message` - Text message to send
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// await handler.send_text('42["ping"]')
    /// ```
    #[uniffi::method]
    pub async fn send_text(&self, message: String) -> Result<(), UniError> {
        self.inner
            .send_text(message)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    /// Send a binary message through this handler.
    ///
    /// # Arguments
    ///
    /// * `data` - Binary data to send
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// await handler.send_binary(b'\\x00\\x01\\x02')
    /// ```
    #[uniffi::method]
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<(), UniError> {
        self.inner
            .send_binary(data)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    /// Send a message and wait for the next matching response.
    ///
    /// # Arguments
    ///
    /// * `message` - Message to send
    ///
    /// # Returns
    ///
    /// The first response that matches this handler's validator
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// response = await handler.send_and_wait('42["getBalance"]')
    /// data = json.loads(response)
    /// ```
    #[uniffi::method]
    pub async fn send_and_wait(&self, message: String) -> Result<String, UniError> {
        let msg = self
            .inner
            .send_and_wait(InnerOutgoing::Text(message))
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;

        Ok(message_to_string(msg.as_ref()))
    }

    /// Wait for the next message that matches this handler's validator.
    ///
    /// # Returns
    ///
    /// The next matching message
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// message = await handler.wait_next()
    /// print(f"Received: {message}")
    /// ```
    #[uniffi::method]
    pub async fn wait_next(&self) -> Result<String, UniError> {
        let msg = self
            .inner
            .wait_next()
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;

        Ok(message_to_string(msg.as_ref()))
    }
}

impl RawHandler {
    /// Creates a RawHandler from an inner RawHandler
    pub(crate) fn from_inner(inner: InnerRawHandler) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

/// Helper function to convert Message to String
fn message_to_string(msg: &Message) -> String {
    match msg {
        Message::Text(text) => text.to_string(),
        Message::Binary(data) => String::from_utf8_lossy(data.as_ref()).into_owned(),
        _ => String::new(),
    }
}
