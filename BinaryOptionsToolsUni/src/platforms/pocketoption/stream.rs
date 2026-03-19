use bo2_macros::uniffi_doc;
use std::sync::Arc;
use tokio::sync::Mutex;

use binary_options_tools::pocketoption::modules::subscriptions::SubscriptionStream as OriginalSubscriptionStream;

use crate::error::UniError;

use super::types::Candle;

#[uniffi_doc(
    name = "SubscriptionStream",
    path = "BinaryOptionsToolsUni/docs_json/stream.json"
)]
#[derive(uniffi::Object)]
pub struct SubscriptionStream {
    inner: Arc<Mutex<OriginalSubscriptionStream>>,
}

impl SubscriptionStream {
    pub(crate) fn from_original(stream: OriginalSubscriptionStream) -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(stream)),
        })
    }
}

#[uniffi::export]
impl SubscriptionStream {
    #[uniffi_doc(name = "next", path = "BinaryOptionsToolsUni/docs_json/stream.json")]
    pub async fn next(&self) -> Result<Candle, UniError> {
        let mut stream = self.inner.lock().await;
        match stream.receive().await {
            Ok(candle) => Ok(candle.into()),
            Err(e) => Err(UniError::from(
                binary_options_tools::error::BinaryOptionsError::from(e),
            )),
        }
    }
}
