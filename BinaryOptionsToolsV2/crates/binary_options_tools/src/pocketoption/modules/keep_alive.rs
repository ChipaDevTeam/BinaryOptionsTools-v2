use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule},
};
use tracing::{debug, warn};
// use tracing::info;

use crate::pocketoption::state::State;

const SID_BASE: &str = r#"0{"sid":"#;
const SID: &str = r#"40{"sid":"#;
const SUCCESSAUTH: &str = r#"451-["successauth","#;

pub struct InitModule {
    ws_sender: AsyncSender<Message>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    state: Arc<State>,
}

pub struct KeepAliveModule {
    ws_sender: AsyncSender<Message>,
}

#[async_trait]
impl LightweightModule<State> for InitModule {
    fn new(
        state: Arc<State>,
        ws_sender: AsyncSender<Message>,
        ws_receiver: AsyncReceiver<Arc<Message>>,
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

    /// The module's asynchronous run loop.
    async fn run(&mut self) -> CoreResult<()> {
        loop {
            let msg = self.ws_receiver.recv().await;
            match msg {
                Ok(msg) => {
                    if let Message::Text(text) = &*msg {
                        match text {
                            _ if text.starts_with(SID_BASE) => {
                                self.ws_sender.send(Message::text("40")).await?;
                            }
                            _ if text.starts_with(SID) => {
                                self.ws_sender.send(Message::text(self.state.ssid.to_string())).await.inspect_err(|e| {
                                    warn!(target: "KeepAliveModule", "Failed to send SSID: {}", e);
                                })?;
                            }
                            _ if text.starts_with(SUCCESSAUTH) => {
                                self.ws_sender.send(Message::text(r#"42["indicator/load"]"#)).await.inspect_err(|e| {
                                    warn!(target: "KeepAliveModule", "Failed to send indicator/load message: {}", e);
                                })?;
                                self.ws_sender.send(Message::text(r#"42["favorite/load"]"#)).await.inspect_err(|e| {
                                    warn!(target: "KeepAliveModule", "Failed to send favorite/load message: {}", e);
                                })?;
                                self.ws_sender.send(Message::text(r#"42["price-alert/load"]"#)).await.inspect_err(|e| {
                                    warn!(target: "KeepAliveModule", "Failed to send price-alert/load message: {}", e);
                                })?;
                                self.ws_sender.send(Message::text(format!("42[\"changeSymbol\",{{\"asset\":\"{}\",\"period\":1}}]", self.state.default_symbol))).await.inspect_err(|e| {
                                    warn!(target: "KeepAliveModule", "Failed to send changeSymbol message: {}", e);
                                })?;
                                self.ws_sender.send(Message::text(format!("42[\"subfor\",\"{}\"]", self.state.default_symbol))).await.inspect_err(|e| {
                                    warn!(target: "KeepAliveModule", "Failed to send subfor message: {}", e);
                                })?;
                            }
                            _ if text == &"2" => {
                                self.ws_sender.send(Message::text("3")).await?;
                            }
                            _ => continue,
                        }
                    } else {
                        // If the message is not a text message, we can ignore it.
                        continue;
                    }
                }
                Err(e) => {
                    warn!(target: "InitModule", "Error receiving message: {}", e);
                    return Err(CoreError::LightweightModuleLoop(
                        "InitModule run loop exited unexpectedly".into(),
                    ));
                }
            }
        }
    }

    /// Route only messages for which this returns true.
    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(|msg: &Message| {
            debug!(target: "LightweightModule", "Routing rule for InitModule: {msg:?}");
            matches!(msg, Message::Text(text) if text.starts_with(SID_BASE) || text.starts_with(SID) || text.starts_with(SUCCESSAUTH) || text == &"2")
        })
    }
}

#[async_trait]
impl LightweightModule<State> for KeepAliveModule {
    fn new(_: Arc<State>, ws_sender: AsyncSender<Message>, _: AsyncReceiver<Arc<Message>>) -> Self {
        Self { ws_sender }
    }

    async fn run(&mut self) -> CoreResult<()> {
        loop {
            // Send a keep-alive message every 20 seconds.
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            self.ws_sender.send(Message::text(r#"42["ps"]"#)).await?;
        }
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(|msg: &Message| {
            debug!(target: "LightweightModule", "Routing rule for KeepAliveModule: {msg:?}");
            false
        })
    }
}
