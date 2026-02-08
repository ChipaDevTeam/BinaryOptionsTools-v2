use std::time::Duration;

use crate::error::{BinaryOptionsResult, BinaryOptionsToolsError};
use core::future::Future;

pub async fn timeout<F, T, E>(duration: Duration, future: F, task: String) -> BinaryOptionsResult<T>
where
    E: Into<BinaryOptionsToolsError>,
    F: Future<Output = Result<T, E>>,
{
    let res = tokio::select! {
        _ = tokio::time::sleep(duration) => Err(BinaryOptionsToolsError::TimeoutError { task, duration }),
        result = future => match result {
            Ok(value) => Ok(value),
            Err(err) => Err(err.into()),
        },
    };
    res
}
