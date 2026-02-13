use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core_pre::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use tracing::{debug, warn};

use crate::pocketoption::state::State;

const SID_BASE: &str = r#"0{"sid":"#;
const SID: &str = r#"40{"sid":"#;

pub struct InitModule {
    ws_sender: AsyncSender<Message>,
    ws_receiver: AsyncReceiver<Arc<Message>>,
    state: Arc<State>,
    runner_command_tx: AsyncSender<RunnerCommand>,
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
        runner_command_tx: AsyncSender<RunnerCommand>,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            ws_sender,
            ws_receiver,
            state,
            runner_command_tx,
        }
    }

    /// The module's asynchronous run loop.
    async fn run(&mut self) -> CoreResult<()> {
        let mut authenticated = false;
        loop {
            let msg = self.ws_receiver.recv().await;
            match msg {
                Ok(msg) => {
                    let mut process_text = None;
                    let mut is_binary = false;
                    match &*msg {
                        Message::Text(text) => {
                            debug!(target: "InitModule", "Processing text message: {}", text);
                            process_text = Some(text.to_string());
                        }
                        Message::Binary(data) => {
                            debug!(target: "InitModule", "Processing binary message ({} bytes)", data.len());
                            is_binary = true;
                            if let Ok(text) = String::from_utf8(data.to_vec()) {
                                process_text = Some(text);
                            }
                        }
                        Message::Close(_) => {
                            if !authenticated {
                                tracing::error!(target: "InitModule", "Connection closed before authentication was completed. Session may be invalid.");
                                let _ = self.runner_command_tx.send(RunnerCommand::Shutdown).await;
                            }
                        }
                        _ => {}
                    }

                    if let Some(text) = process_text {
                        // Handle simple Socket.IO control messages
                        if text.starts_with(SID_BASE) {
                            tracing::debug!(target: "InitModule", "Received Engine.IO handshake (0). Sending Socket.IO connect (40)...");

                            if let Err(e) = self.ws_sender.send(Message::text("40")).await {
                                warn!(target: "InitModule", "Failed to send 40: {}", e);
                                return Err(e.into());
                            }
                            continue;
                        }

                        // Socket.IO 4.x established connection SID message: 40{"sid":"..."}
                        if text.starts_with("40") {
                            let ssid_str = self.state.ssid.to_string();
                            let redacted_ssid = if ssid_str.len() > 20 {
                                format!("{}...", &ssid_str[..20])
                            } else {
                                "REDACTED".to_string()
                            };
                            tracing::debug!(target: "InitModule", "Socket.IO session established ({}). Sending auth SSID: {}", text, redacted_ssid);

                            if let Err(e) = self.ws_sender.send(Message::text(ssid_str)).await {
                                let err_str = e.to_string().to_lowercase();
                                if !err_str.contains("closed") && !err_str.contains("broken pipe") {
                                    warn!(target: "InitModule", "Failed to send SSID: {}", e);
                                    return Err(e.into());
                                }
                                debug!(target: "InitModule", "Socket closed before SSID could be sent");
                            }
                            continue;
                        }

                        if text == "41" {
                            tracing::error!(target: "InitModule", "Server sent Socket.IO disconnect signal (41). Authentication rejected or session expired. Message: {}", text);

                            // Log public IP on rejection to help user identify IP mismatch issues
                            if let Ok(ip) = crate::pocketoption::utils::get_public_ip().await {
                                tracing::warn!(target: "InitModule", "Session rejected while connecting from public IP: {}", ip);
                            }

                            // Signal shutdown to the runner because auth failed
                            if let Err(e) =
                                self.runner_command_tx.send(RunnerCommand::Shutdown).await
                            {
                                warn!(target: "InitModule", "Failed to send shutdown command to runner: {}", e);
                            }

                            // If we get 41, it's a permanent rejection for this session
                            return Err(CoreError::SsidParsing(format!(
                                "Server rejected session (41). Raw: {}",
                                text
                            )));
                        }

                        if text.as_str() == "2" {
                            self.ws_sender.send(Message::text("3")).await?;
                            continue;
                        }

                        // Handle complex event messages (successauth, etc.)
                        let mut trigger_auth = false;
                        if let Some(start) = text.find('[') {
                            if let Ok(value) =
                                serde_json::from_str::<serde_json::Value>(&text[start..])
                            {
                                if let Some(arr) = value.as_array() {
                                    let event_name = arr.first().and_then(|v| v.as_str());
                                    if event_name == Some("successauth") {
                                        trigger_auth = true;
                                    }
                                }
                            }
                        } else if is_binary && text.contains("serverName") {
                            // Binary part of successauth
                            trigger_auth = true;
                        }

                        if trigger_auth {
                            authenticated = true;
                            tracing::debug!(target: "InitModule", "Authentication successful! Triggering data load.");

                            // Explicitly request everything needed for a full sync
                            let initialization_messages = vec![
                                r#"42["assets/load"]"#.to_string(),
                                r#"42["indicator/load"]"#.to_string(),
                                r#"42["favorite/load"]"#.to_string(),
                                r#"42["price-alert/load"]"#.to_string(),
                                format!(
                                    r#"42["changeSymbol",{{ "asset":"{}","period":60 }}]"#,
                                    self.state.default_symbol
                                ),
                                format!(r#"42["subfor","{}"]"#, self.state.default_symbol),
                            ];

                            for raw_msg in initialization_messages {
                                self.ws_sender.send(Message::text(raw_msg)).await.inspect_err(|e| {
                                    warn!(target: "InitModule", "Failed to send init message: {}", e);
                                })?;
                            }
                            continue;
                        }
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
        Box::new(InitRule::new())
    }
}

struct InitRule {
    valid: AtomicBool,
}

impl InitRule {
    fn new() -> Self {
        Self {
            valid: AtomicBool::new(false),
        }
    }
}

impl Rule for InitRule {
    fn call(&self, msg: &Message) -> bool {
        match msg {
            Message::Text(text) => {
                if text.starts_with(SID_BASE)
                    || text.starts_with(SID)
                    || text.as_str() == "41"
                    || text.as_str() == "2"
                {
                    return true;
                }

                // Check for successauth in a Socket.IO array
                if let Some(start) = text.find('[') {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text[start..]) {
                        if let Some(arr) = value.as_array() {
                            if let Some(event_name) = arr.first().and_then(|v| v.as_str()) {
                                if event_name == "successauth" {
                                    // Detect if this is a binary placeholder
                                    let has_placeholder = arr.iter().skip(1).any(|v| {
                                        v.as_object()
                                            .is_some_and(|obj| obj.contains_key("_placeholder"))
                                    });

                                    if arr.len() == 1 || has_placeholder {
                                        self.valid.store(true, Ordering::SeqCst);
                                        return false; // Wait for binary part
                                    } else {
                                        self.valid.store(false, Ordering::SeqCst);
                                        return true;
                                    }
                                } else {
                                    // It's an event, but not successauth.
                                    return false;
                                }
                            }
                        }
                    }
                }

                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    return true;
                }
                false
            }
            Message::Binary(_) => {
                if self.valid.load(Ordering::SeqCst) {
                    self.valid.store(false, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn reset(&self) {
        self.valid.store(false, Ordering::SeqCst)
    }
}

#[async_trait]
impl LightweightModule<State> for KeepAliveModule {
    fn new(
        _: Arc<State>,
        ws_sender: AsyncSender<Message>,
        _: AsyncReceiver<Arc<Message>>,
        _: AsyncSender<RunnerCommand>,
    ) -> Self {
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
