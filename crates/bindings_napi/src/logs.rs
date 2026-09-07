use std::fs::OpenOptions;

use napi_derive::napi;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::error::{napi_err, BinaryErrorNode};

/// Options accepted by [`start_logs`].
#[allow(dead_code)]
#[napi(object)]
pub struct LogOptions {
    /// Directory that receives `logs.log` and `error.log`. Defaults to `"."`.
    pub path: Option<String>,
    /// Minimum level to record: `TRACE`, `DEBUG`, `INFO`, `WARN` or `ERROR`.
    /// Defaults to `DEBUG`.
    pub level: Option<String>,
    /// Also print the logs to the terminal. Defaults to `false`.
    pub terminal: Option<bool>,
}

/// Installs the global tracing subscriber used by the library.
///
/// Writes `logs.log` (everything at or above `level`) and `error.log`
/// (warnings and errors) inside `options.path`. Calling this more than once is
/// a no-op: only the first subscriber to be installed wins.
#[napi]
#[allow(dead_code)]
pub fn start_logs(options: Option<LogOptions>) -> napi::Result<()> {
    let options = options.unwrap_or(LogOptions {
        path: None,
        level: None,
        terminal: None,
    });
    let path = options.path.unwrap_or_else(|| ".".to_string());
    let level: LevelFilter = options
        .level
        .and_then(|level| level.parse().ok())
        .unwrap_or_else(|| Level::DEBUG.into());
    let terminal = options.terminal.unwrap_or(false);

    let error_logs = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{path}/error.log"))
        .map_err(|e| napi_err(BinaryErrorNode::NotAllowed(e.to_string())))?;
    let logs = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{path}/logs.log"))
        .map_err(|e| napi_err(BinaryErrorNode::NotAllowed(e.to_string())))?;

    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(error_logs)
                .with_filter(LevelFilter::WARN),
        )
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(logs)
                .with_filter(level),
        );

    if terminal {
        let _ = subscriber.with(fmt::layer().with_filter(level)).try_init();
    } else {
        let _ = subscriber.try_init();
    }

    Ok(())
}
