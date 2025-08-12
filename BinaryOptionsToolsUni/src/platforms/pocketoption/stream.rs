use std::sync::Arc;
use tokio::sync::Mutex;

use binary_options_tools::pocketoption::modules::subscriptions::SubscriptionStream as OriginalSubscriptionStream;

use crate::error::UniError;

use super::types::Candle;

/// Represents a stream of subscription data.
///
/// This object is returned by the `subscribe` method on the `PocketOption` client.
/// It allows you to receive real-time data, such as candles, for a specific asset.
///
/// # Rationale
///
/// Since UniFFI does not support streams directly, this wrapper provides a way to
/// consume the stream by repeatedly calling the `next` method.
#[derive(uniffi::Object)]
pub struct SubscriptionStream {
    inner: Arc<Mutex<OriginalSubscriptionStream>>,
}

impl SubscriptionStream {
    // Internal helper to construct from the original stream (not exported to UniFFI)
    pub(crate) fn from_original(stream: OriginalSubscriptionStream) -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(stream)),
        })
    }
}

#[uniffi::export]
impl SubscriptionStream {
    /// Retrieves the next item from the stream.
    ///
    /// This method should be called in a loop to consume the data from the stream.
    /// It will return `None` when the stream is closed.
    ///
    /// # Returns
    ///
    /// An optional `Candle` object. It will be `None` if the stream has finished.
    ///
    /// # Examples
    ///
    /// ## Python
    /// ```python
    /// import asyncio
    ///
    /// async def main():
    ///     # ... (get api object)
    ///     stream = await api.subscribe("EURUSD_otc", 5)
    ///     while True:
    ///         candle = await stream.next()
    ///         if candle is None:
    ///             break
    ///         print(f"New candle: {candle}")
    ///
    /// asyncio.run(main())
    /// ```
    ///
    /// ## Swift
    /// ```swift
    /// func subscribe() async {
    ///     // ... (get api object)
    ///     let stream = try! await api.subscribe(asset: "EURUSD_otc", durationSecs: 5)
    ///     while let candle = try! await stream.next() {
    ///         print("New candle: \(candle)")
    ///     }
    /// }
    /// ```
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
