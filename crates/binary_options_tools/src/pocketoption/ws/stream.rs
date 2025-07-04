use std::sync::Arc;
use std::time::Duration;

use crate::pocketoption::error::PocketOptionError;
use binary_options_tools_core::error::BinaryOptionsToolsError;
use chrono::{DateTime, Utc};
use tracing::debug;
// use pin_project_lite::pin_project;
use crate::pocketoption::{
    error::PocketResult, parser::message::WebSocketMessage, types::update::DataCandle,
};

use async_channel::{Receiver, RecvError};
use futures_util::Stream;
use futures_util::stream::unfold;

#[derive(Clone)]
pub struct StreamAsset {
    reciever: Receiver<WebSocketMessage>,
    asset: String,
    condition: ConditonnalUpdate,
}

/// This enum tells the StreamAsset when to send new data
#[derive(Clone)]
pub enum ConditonnalUpdate {
    None, // No condition, once data is received, data is sent
    Size {
        count: usize,        // Current count of candles
        target: usize,       // Target size to reach
        current: DataCandle, // Aggregated candle data
    },
    Time {
        start_time: Option<DateTime<Utc>>, // Time of first candle
        duration: Duration,                // Target duration
        current: DataCandle,               // Aggregated candle data
    },
}

impl ConditonnalUpdate {
    fn new_size(size: usize) -> Self {
        Self::Size {
            count: 0,
            target: size,
            current: DataCandle::default(), // You'll need to implement Default
        }
    }

    fn new_time(duration: Duration) -> Self {
        Self::Time {
            start_time: None,
            duration,
            current: DataCandle::default(),
        }
    }

    pub fn update_and_check(&mut self, new_candle: &DataCandle) -> PocketResult<bool> {
        match self {
            Self::None => Ok(true),

            Self::Size {
                count,
                target,
                current,
            } => {
                // Update the aggregated candle
                if *count == 0 {
                    *current = new_candle.clone();
                } else {
                    current.time = new_candle.time;
                    current.high = current.high.max(new_candle.high);
                    current.low = current.low.min(new_candle.low);
                    current.close = new_candle.close;
                }
                *count += 1;

                if *count >= *target {
                    *count = 0; // Reset for next batch
                    Ok(true)
                } else {
                    Ok(false)
                }
            }

            Self::Time {
                start_time,
                duration,
                current,
            } => {
                if start_time.is_none() {
                    *start_time = Some(new_candle.time);
                    *current = new_candle.clone();
                    return Ok(false);
                }

                // Update the aggregated candle
                current.time = new_candle.time;
                current.high = current.high.max(new_candle.high);
                current.low = current.low.min(new_candle.low);
                current.close = new_candle.close;

                let elapsed = (new_candle.time - start_time.unwrap())
                    .to_std()
                    .map_err(|_| {
                        PocketOptionError::UnreachableError(
                            "Time calculation error in conditional update".to_string(),
                        )
                    })?;

                if elapsed >= *duration {
                    *start_time = None; // Reset for next period
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    pub fn get_current_candle(&self) -> Option<DataCandle> {
        match self {
            Self::None => None,
            Self::Size { current, .. } => Some(current.clone()),
            Self::Time { current, .. } => Some(current.clone()),
        }
    }
}

impl StreamAsset {
    pub fn new(reciever: Receiver<WebSocketMessage>, asset: String) -> Self {
        Self {
            reciever,
            asset,
            condition: ConditonnalUpdate::None,
        }
    }

    pub fn new_chuncked(
        reciever: Receiver<WebSocketMessage>,
        asset: String,
        chunk_size: usize,
    ) -> Self {
        Self {
            reciever,
            asset,
            condition: ConditonnalUpdate::new_size(chunk_size),
        }
    }

    pub fn new_timed(reciever: Receiver<WebSocketMessage>, asset: String, time: Duration) -> Self {
        Self {
            reciever,
            asset,
            condition: ConditonnalUpdate::new_time(time),
        }
    }

    pub async fn recieve(&self) -> PocketResult<DataCandle> {
        let mut condition = self.condition.clone();

        while let Ok(msg) = self.reciever.recv().await {
            debug!(target: "StreamAsset", "Received UpdateStream!");
            if let WebSocketMessage::UpdateStream(stream) = msg {
                if let Some(candle) = stream.0.first().take_if(|x| x.active == self.asset) {
                    let data_candle: DataCandle = candle.into();
                    if condition.update_and_check(&data_candle)? {
                        return Ok(condition.get_current_candle().unwrap_or(data_candle));
                    }
                }
            }
        }

        Err(BinaryOptionsToolsError::ChannelRequestRecievingError(RecvError).into())
    }

    // pub async fn _recieve(&self) -> PocketResult<DataCandle> {
    //     while let Ok(candle) = self.reciever.recv().await {
    //         debug!(target: "StreamAsset", "Recieved UpdateStream!");
    //         if let WebSocketMessage::UpdateStream(candle) = candle {
    //             if let Some(candle) = candle.0.first().take_if(|x| x.active == self.asset) {
    //                 return Ok(candle.into());
    //             }
    //         }
    //     }

    //     unreachable!(
    //         "This should never happen, please contact Rick-29 at https://github.com/Rick-29"
    //     )
    // }

    // pub async fn recieve_chunked(&self) -> PocketResult<DataCandle> {
    //     let mut chunk = vec![];
    //     while let Ok(candle) = self.reciever.recv().await {
    //         debug!(target: "StreamAsset", "Recieved UpdateStream!");
    //         if let WebSocketMessage::UpdateStream(candle) = candle {
    //             if let Some(candle) = candle.0.first().take_if(|x| x.active == self.asset) {
    //                 chunk.push(candle.into());
    //                 if chunk.len() >= self.chunk_size {
    //                     return chunk.try_into();
    //                 }
    //             }
    //         }
    //     }

    //     unreachable!(
    //         "This should never happen, please contact Rick-29 at https://github.com/Rick-29"
    //     )
    // }

    pub fn to_stream(&self) -> impl Stream<Item = PocketResult<DataCandle>> + '_ {
        Box::pin(unfold(self, |state| async move {
            let item = state.recieve().await;
            Some((item, state))
        }))
    }

    pub fn to_stream_static(
        self: Arc<Self>,
    ) -> impl Stream<Item = PocketResult<DataCandle>> + 'static {
        Box::pin(unfold(self, |state| async move {
            let item = state.recieve().await;
            Some((item, state))
        }))
    }
}

// impl Stream for StreamAsset {
//     type Item = Candle;

//     fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
//         match self.reciever.recv()

//         }
//     }
// }
