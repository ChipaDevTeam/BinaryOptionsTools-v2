use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use tracing::debug;

use crate::pocketoption::{
    state::State,
    types::{StreamData, TwoStepRule},
};

pub struct ServerTimeModule {
    receiver: AsyncReceiver<Arc<Message>>,
    state: Arc<State>,
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
            if let Message::Binary(data) = &*msg {
                if let Ok(candle) = serde_json::from_slice::<StreamData>(data) {
                    // Process the candle data
                    debug!("Received candle data: {:?}", candle);
                    self.state.update_server_time(candle.timestamp).await;
                }
            }
        }
        Err(CoreError::LightweightModuleLoop(
            "ServerTimeModule".to_string(),
        ))
    }

    /// Route only messages for which this returns true.
    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(TwoStepRule::new(r#"451-["updateStream","#))
    }
}
