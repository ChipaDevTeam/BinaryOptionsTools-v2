use std::sync::Arc;

use bo2_macros::uniffi_doc;

use crate::error::UniError;
use binary_options_tools::error::BinaryOptionsError;
use binary_options_tools::{
    pocketoption::modules::raw::{Outgoing as InnerOutgoing, RawHandler as InnerRawHandler},
    stream::Message,
};

#[uniffi_doc(
    name = "RawHandler",
    path = "BinaryOptionsToolsUni/docs_json/raw_handler.json"
)]
#[derive(uniffi::Object)]
pub struct RawHandler {
    inner: InnerRawHandler,
}

#[uniffi::export]
impl RawHandler {
    #[uniffi_doc(
        name = "send_text",
        path = "BinaryOptionsToolsUni/docs_json/raw_handler.json"
    )]
    #[uniffi::method]
    pub async fn send_text(&self, message: String) -> Result<(), UniError> {
        self.inner
            .send_text(message)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    #[uniffi_doc(
        name = "send_binary",
        path = "BinaryOptionsToolsUni/docs_json/raw_handler.json"
    )]
    #[uniffi::method]
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<(), UniError> {
        self.inner
            .send_binary(data)
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))
    }

    #[uniffi_doc(
        name = "send_and_wait",
        path = "BinaryOptionsToolsUni/docs_json/raw_handler.json"
    )]
    #[uniffi::method]
    pub async fn send_and_wait(&self, message: String) -> Result<String, UniError> {
        let msg = self
            .inner
            .send_and_wait(InnerOutgoing::Text(message))
            .await
            .map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;

        Ok(message_to_string(msg.as_ref()))
    }

    #[uniffi_doc(
        name = "wait_next",
        path = "BinaryOptionsToolsUni/docs_json/raw_handler.json"
    )]
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
    pub(crate) fn from_inner(inner: InnerRawHandler) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

fn message_to_string(msg: &Message) -> String {
    match msg {
        Message::Text(text) => text.to_string(),
        Message::Binary(data) => String::from_utf8_lossy(data.as_ref()).into_owned(),
        _ => String::new(),
    }
}
