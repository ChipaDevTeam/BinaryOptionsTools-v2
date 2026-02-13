use std::sync::Arc;

use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use serde_json::Value;
use tracing::warn;

use crate::expertoptions::{state::State, Action};

pub struct PongModule {
    ws_sender: AsyncSender<Message>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    state: Arc<State>,
}

#[async_trait::async_trait]
impl LightweightModule<State> for PongModule {
    fn new(
        state: Arc<State>,
        ws_sender: AsyncSender<Message>,
        ws_receiver: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            ws_sender,
            ws_receiver,
            state,
        }
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.ws_receiver.recv().await {
            if let Message::Binary(text) = &*msg {
                match Action::from_json::<Value>(text) {
                    Ok(action) => {
                        self.ws_sender
                            .send(
                                Action::new("pong".into(), self.state.token.clone(), 2, action)
                                    .to_message()?,
                            )
                            .await?;
                    }
                    Err(e) => {
                        warn!(target: "PongModule", "Failed to parse message into a `PongResponse` variant, {e}")
                    }
                }
            }
        }
        Err(CoreError::LightweightModuleLoop("PongModule".into()))
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(PongRule)
    }
}

struct PongRule;

impl Rule for PongRule {
    fn call(&self, msg: &Message) -> bool {
        if let Message::Binary(text) = msg {
            text.starts_with(b"{{\"action\":\"ping\"")
        } else {
            false
        }
    }

    fn reset(&self) {
        // No state to reset
    }
}
