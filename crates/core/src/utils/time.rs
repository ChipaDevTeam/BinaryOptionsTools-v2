use std::time::Duration;

use crate::error::{Error, Result};
use core::future::Future;

pub async fn timeout<F, T, E>(duration: Duration, future: F, task: String) -> Result<T>
where
    E: Into<Error>,
    F: Future<Output = std::result::Result<T, E>>,
{
    let res = tokio::select! {
        _ = tokio::time::sleep(duration) => Err(Error::TimeoutError { task, duration }),
        result = future => match result {
            Ok(value) => Ok(value),
            Err(err) => Err(err.into()),
        },
    };
    res
}
