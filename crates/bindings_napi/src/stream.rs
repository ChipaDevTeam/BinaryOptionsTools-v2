use std::sync::Arc;

use futures_util::stream::{BoxStream, Fuse};
use futures_util::StreamExt;
use napi_derive::napi;
use serde_json::Value;
use tokio::sync::Mutex;

use binary_options_tools::pocketoption::candle::Candle;
use binary_options_tools::pocketoption::error::PocketResult;

use crate::error::napi_err;

pub type SharedStream<T> = Arc<Mutex<Fuse<BoxStream<'static, PocketResult<T>>>>>;

pub fn shared_stream<T: Send + 'static>(
    stream: BoxStream<'static, PocketResult<T>>,
) -> SharedStream<T> {
    Arc::new(Mutex::new(stream.fuse()))
}

/// Async iterator over the candles produced by a symbol subscription.
///
/// `next()` resolves to `null` once the underlying stream is exhausted, which
/// is what the `Symbol.asyncIterator` wrapper in the JS package relies on.
#[napi]
pub struct CandleStream {
    pub(crate) stream: SharedStream<Candle>,
}

#[napi]
impl CandleStream {
    /// Resolves with the next candle, or `null` when the stream ends.
    #[napi]
    pub async fn next(&self) -> napi::Result<Option<Value>> {
        let mut stream = self.stream.lock().await;
        match stream.next().await {
            Some(Ok(candle)) => {
                let value = serde_json::to_value(&candle).map_err(napi_err)?;
                Ok(Some(value))
            }
            Some(Err(e)) => Err(napi_err(e)),
            None => Ok(None),
        }
    }
}

/// Async iterator over raw WebSocket messages, yielded as strings.
#[napi]
pub struct RawStream {
    pub(crate) stream: SharedStream<String>,
}

#[napi]
impl RawStream {
    /// Resolves with the next message, or `null` when the stream ends.
    #[napi]
    pub async fn next(&self) -> napi::Result<Option<String>> {
        let mut stream = self.stream.lock().await;
        match stream.next().await {
            Some(Ok(message)) => Ok(Some(message)),
            Some(Err(e)) => Err(napi_err(e)),
            None => Ok(None),
        }
    }
}
