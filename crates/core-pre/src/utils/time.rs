use std::time::Duration;

use core::future::Future;

use crate::error::{CoreError, CoreResult};

pub async fn timeout<F, T, E>(duration: Duration, future: F, task: String) -> CoreResult<T>
where
    E: Into<CoreError>,
    F: Future<Output = Result<T, E>>,
{
    let res = tokio::select! {
        _ = tokio::time::sleep(duration) => Err(CoreError::TimeoutError { task, duration }),
        result = future => match result {
            Ok(value) => Ok(value),
            Err(err) => Err(err.into()),
        },
    };
    res
}
