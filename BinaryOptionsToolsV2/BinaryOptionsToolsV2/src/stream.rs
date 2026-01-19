use std::sync::Arc;

use futures_util::{
    StreamExt,
    stream::{BoxStream, Fuse},
};
use pyo3::{
    PyResult,
    exceptions::{PyRuntimeError, PyStopAsyncIteration, PyStopIteration},
};
use tokio::sync::Mutex;

pub type PyStream<T, E> = Fuse<BoxStream<'static, Result<T, E>>>;

pub async fn next_stream<T, E>(stream: Arc<Mutex<PyStream<T, E>>>, sync: bool) -> PyResult<T>
where
    E: std::error::Error,
{
    let mut stream = stream.lock().await;
    match stream.next().await {
        Some(item) => match item {
            Ok(itm) => Ok(itm),
            Err(e) => Err(PyRuntimeError::new_err(e.to_string())),
        },
        None => match sync {
            true => Err(PyStopIteration::new_err("Stream exhausted")),
            false => Err(PyStopAsyncIteration::new_err("Stream exhausted")),
        },
    }
}
