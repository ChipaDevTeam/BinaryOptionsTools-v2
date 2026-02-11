use std::{fs::OpenOptions, io::Write, time::Duration};

use async_channel::{bounded, Sender};
use serde_json::Value;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer, Registry,
};

use crate::{constants::MAX_LOGGING_CHANNEL_CAPACITY, general::stream::RecieverStream};

pub fn start_tracing(terminal: bool) -> anyhow::Result<()> {
    let error_logs = OpenOptions::new()
        .append(true)
        .create(true)
        .open("errors.log")?;

    let sub = tracing_subscriber::registry()
        // .with(filtered_layer)
        .with(
            // log-error file, to log the errors that arise
            fmt::layer()
                .with_ansi(false)
                .with_writer(error_logs)
                .with_filter(LevelFilter::WARN),
        );
    if terminal {
        sub.with(fmt::Layer::default().with_filter(LevelFilter::DEBUG))
            .try_init()?;
    } else {
        sub.try_init()?;
    }

    Ok(())
}

pub fn start_tracing_leveled(terminal: bool, level: LevelFilter) -> anyhow::Result<()> {
    let error_logs = OpenOptions::new()
        .append(true)
        .create(true)
        .open("errors.log")?;

    let sub = tracing_subscriber::registry()
        // .with(filtered_layer)
        .with(
            // log-error file, to log the errors that arise
            fmt::layer()
                .with_ansi(false)
                .with_writer(error_logs)
                .with_filter(LevelFilter::WARN),
        );
    if terminal {
        sub.with(fmt::Layer::default().with_filter(level))
            .try_init()?;
    } else {
        sub.try_init()?;
    }

    Ok(())
}

#[derive(Clone)]
pub struct StreamWriter {
    sender: Sender<String>,
}

impl Write for StreamWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(item) = serde_json::from_slice::<Value>(buf) {
            self.sender
                .send_blocking(item.to_string())
                .map_err(std::io::Error::other)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for StreamWriter {
    type Writer = StreamWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

pub fn stream_logs_layer(
    level: LevelFilter,
    timout: Option<Duration>,
) -> (
    Box<dyn Layer<Registry> + Send + Sync>,
    RecieverStream<String>,
) {
    let (sender, receiver) = bounded(MAX_LOGGING_CHANNEL_CAPACITY);
    let receiver = RecieverStream::new_timed(receiver, timout);
    let writer = StreamWriter { sender };
    let layer = tracing_subscriber::fmt::layer::<Registry>()
        .json()
        .flatten_event(true)
        .with_writer(writer)
        .with_filter(level)
        .boxed();
    (layer, receiver)
}
