use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use tracing::debug;

use crate::pocketoption::{
    state::State,
    types::{MultiPatternRule, StreamBatch},
};

pub struct ServerTimeModule {
    receiver: AsyncReceiver<Arc<Message>>,
    state: Arc<State>,
}

impl ServerTimeModule {
    /// Processes a successfully deserialized batch by logging and updating server time
    /// from its most recent tick.
    async fn handle_stream_data(&self, batch: StreamBatch) {
        debug!("Received candle data: {:?}", batch);
        if let Some(latest) = batch.0.iter().map(|t| t.timestamp).max() {
            self.state.update_server_time(latest).await;
        }
    }
}

#[async_trait]
impl LightweightModule<State> for ServerTimeModule {
    fn new(
        state: Arc<State>,
        _: AsyncSender<Message>,
        ws_receiver: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            receiver: ws_receiver,
            state,
        }
    }

    /// The module's asynchronous run loop.
    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            match msg.as_ref() {
                Message::Binary(data) => match serde_json::from_slice::<StreamBatch>(data) {
                    Ok(candle) => self.handle_stream_data(candle).await,
                    Err(e) => {
                        debug!(
                            "Failed to parse StreamData (binary, {} bytes): {}",
                            data.len(),
                            e
                        );
                    }
                },
                Message::Text(text) => match serde_json::from_str::<StreamBatch>(text) {
                    Ok(candle) => self.handle_stream_data(candle).await,
                    Err(e) => {
                        debug!(
                            "Failed to parse StreamData (text, {} chars): {}",
                            text.len(),
                            e
                        );
                    }
                },
                _ => {
                    tracing::warn!(target: "ServerTimeModule", "Received unexpected message type: {:?}", msg);
                }
            }
        }
        Err(CoreError::LightweightModuleLoop(
            "ServerTimeModule".to_string(),
        ))
    }

    /// Route only messages for which this returns true.
    ///
    /// `MultiPatternRule` handles both shapes of `updateStream`: the two-step
    /// `451-["updateStream",{"_placeholder":true,...}]` + binary tick used by the
    /// current servers, and the 1-step `42["updateStream",[...]]` variant.
    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec!["updateStream"]))
    }
}
