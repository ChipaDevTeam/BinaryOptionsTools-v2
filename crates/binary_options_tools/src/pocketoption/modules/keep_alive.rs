use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use binary_options_tools_core::{
    error::{CoreError, CoreResult},
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{LightweightModule, Rule, RunnerCommand},
};
use tracing::{debug, warn};

use crate::pocketoption::state::State;
use crate::pocketoption::utils::socketio_event;

/// Socket.IO event names the server emits when it refuses the auth payload.
/// These are terminal: the server follows them with a disconnect (41).
const AUTH_REJECTED_EVENTS: [&str; 3] = ["NotAuthorized", "notAuthorized", "auth_error"];

/// Event the server uses to confirm the auth payload was accepted.
const SUCCESS_AUTH: &str = "successauth";

/// How long to wait for `successauth` before requesting the initial snapshot anyway.
///
/// Account sessions answer the auth frame explicitly, but guest ("try demo")
/// sessions are accepted without any confirmation, so waiting for one that never
/// arrives left those sessions without their initial sync.
const AUTH_INIT_GRACE: std::time::Duration = std::time::Duration::from_millis(1500);

/// Keys that identify the binary frame carrying the `successauth` confirmation.
///
/// The server has changed this payload over time: `{"serverName": ...}` became
/// `{"id": ..., "server": ...}`, and account sessions now receive only `{"id": ...}`.
/// Matching the shape instead of one key keeps every variant working. The bare-id
/// form must be the *whole* object, otherwise any payload that carries an `id`
/// (deals do) would look like an auth confirmation.
const AUTH_CONFIRMATION_KEYS: [&str; 2] = ["server", "serverName"];

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

impl InitModule {
    /// Handle an explicit authentication rejection from the server.
    ///
    /// Records the reason on the shared state so client construction can surface it
    /// instead of a generic timeout, then shuts the runner down.
    async fn reject_auth(&self, event: &str) -> CoreResult<()> {
        let reason = format!(
            "Server rejected the session with `{event}`. The SSID is invalid, expired,              or bound to a different IP/user agent. Generate a fresh SSID and retry."
        );
        tracing::error!(target: "InitModule", "{}", reason);

        // Log public IP on rejection to help user identify IP mismatch issues
        if let Ok(ip) = crate::pocketoption::utils::get_public_ip().await {
            tracing::warn!(target: "InitModule", "Session rejected while connecting from public IP: {}", ip);
        }

        self.state.set_auth_error(reason.clone());

        if let Err(e) = self.runner_command_tx.send(RunnerCommand::Shutdown).await {
            warn!(target: "InitModule", "Failed to send shutdown command to runner: {}", e);
        }

        Err(CoreError::SsidParsing(reason))
    }

    /// Requests everything the client needs for a full sync after auth.
    ///
    /// Without this the server keeps the symbol unselected and never answers
    /// `loadHistoryPeriod` / `updateStream`, which made every candle and tick
    /// request block until it timed out.
    async fn request_initial_snapshot(&mut self) -> CoreResult<()> {
        debug!(target: "InitModule", "Requesting the initial snapshot.");

        let initialization_messages = vec![
            r#"42["assets/load"]"#.to_string(),
            r#"42["indicator/load"]"#.to_string(),
            r#"42["favorite/load"]"#.to_string(),
            r#"42["price-alert/load"]"#.to_string(),
            r#"42["getBalance"]"#.to_string(),
            format!(
                r#"42["changeSymbol",{{ "asset":"{}","period":60 }}]"#,
                self.state.default_symbol
            ),
            format!(r#"42["subfor","{}"]"#, self.state.default_symbol),
        ];

        for raw_msg in initialization_messages {
            self.ws_sender
                .send(Message::text(raw_msg))
                .await
                .inspect_err(|e| {
                    warn!(target: "InitModule", "Failed to send init message: {}", e);
                })?;
        }
        Ok(())
    }
}

/// Whether a binary frame is the confirmation payload of `successauth`.
///
/// Recognised by shape rather than by a single key because the server has shipped
/// several payloads for the same event (`{"serverName": ...}`, `{"id": ..., "server": ...}`,
/// and now `{"id": ...}` for account sessions).
fn is_auth_confirmation(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .is_some_and(|value| {
            value.as_object().is_some_and(|object| {
                AUTH_CONFIRMATION_KEYS
                    .iter()
                    .any(|key| object.contains_key(*key))
                    || (object.len() == 1 && object.contains_key("id"))
            })
        })
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
        // Whether the initial snapshot was already requested for the current connection.
        let mut initialized = false;
        // When to request the snapshot even if the server never confirms the auth.
        let mut grace_deadline: Option<tokio::time::Instant> = None;

        loop {
            let grace = grace_deadline.unwrap_or_else(|| {
                tokio::time::Instant::now() + std::time::Duration::from_secs(3600)
            });
            let msg = tokio::select! {
                _ = tokio::time::sleep_until(grace), if grace_deadline.is_some() => {
                    grace_deadline = None;
                    if !initialized {
                        debug!(
                            target: "InitModule",
                            "No `{SUCCESS_AUTH}` within {:?}; requesting the initial snapshot anyway (guest sessions are never confirmed).",
                            AUTH_INIT_GRACE
                        );
                        self.request_initial_snapshot().await?;
                        initialized = true;
                    }
                    authenticated = true;
                    continue;
                }
                msg = self.ws_receiver.recv() => msg,
            };

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
                                self.state.set_auth_error(
                                    "Connection closed before authentication completed; the session may be invalid or expired.",
                                );
                                let _ = self.runner_command_tx.send(RunnerCommand::Shutdown).await;
                            }
                        }
                        _ => {
                            warn!(target: "InitModule", "Received unexpected message type: {:?}", msg);
                        }
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
                            let mut ssid_str = self.state.ssid.to_string();

                            // Ensure SSID is correctly formatted for Socket.IO (starts with a packet type, usually 42)
                            if !ssid_str.starts_with('4') {
                                debug!(target: "InitModule", "SSID does not start with Socket.IO packet type; wrapping in 42[\"auth\",...]");
                                ssid_str = format!(r#"42["auth",{}]"#, ssid_str);
                            }

                            let redacted_ssid = if ssid_str.len() > 20 {
                                format!("{}...", &ssid_str[..20])
                            } else {
                                "REDACTED".to_string()
                            };
                            tracing::debug!(target: "InitModule", "Socket.IO session established ({}). Sending auth SSID: {}", text, redacted_ssid);

                            if let Err(e) = self.ws_sender.send(Message::text(ssid_str)).await {
                                warn!(target: "InitModule", "Failed to send SSID: {}", e);
                                return Err(e.into());
                            }
                            // A reconnect re-authenticates, so the snapshot must be requestable
                            // again. Guest sessions never answer with `successauth`, hence the
                            // grace fallback.
                            initialized = false;
                            authenticated = false;
                            grace_deadline = Some(tokio::time::Instant::now() + AUTH_INIT_GRACE);
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
                            let reason = format!("Server rejected session (41). Raw: {}", text);
                            self.state.set_auth_error(reason.clone());
                            return Err(CoreError::SsidParsing(reason));
                        }

                        if text.as_str() == "2" {
                            self.ws_sender.send(Message::text("3")).await?;
                            continue;
                        }

                        // Handle complex event messages (successauth, NotAuthorized, etc.)
                        let mut auth_confirmed = false;
                        if let Some((event_name, _)) = socketio_event(&text) {
                            if event_name == SUCCESS_AUTH {
                                auth_confirmed = true;
                            } else if AUTH_REJECTED_EVENTS.contains(&event_name.as_str()) {
                                return self.reject_auth(&event_name).await;
                            }
                        } else if is_binary && is_auth_confirmation(&text) {
                            // Binary half of `successauth`; its payload shape changed
                            // between server versions.
                            auth_confirmed = true;
                        }

                        if auth_confirmed {
                            authenticated = true;
                            grace_deadline = None;
                            if !initialized {
                                self.request_initial_snapshot().await?;
                                initialized = true;
                            }
                            // Marked after the snapshot is queued so that callers waiting on
                            // it know `changeSymbol`/`subfor` are already on the wire.
                            self.state.mark_session_confirmed();
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
                if let Some((event_name, is_placeholder)) = socketio_event(text) {
                    if event_name == SUCCESS_AUTH {
                        // The header is routed even when it is only a placeholder: the module
                        // treats it as the confirmation, and `valid` makes it receive the
                        // binary half as well. Before this, the binary half was the only
                        // routed frame and its payload no longer carries `serverName`, so
                        // authentication was never detected and the initial snapshot was
                        // never requested.
                        self.valid.store(is_placeholder, Ordering::SeqCst);
                        return true;
                    }
                    if AUTH_REJECTED_EVENTS.contains(&event_name.as_str()) {
                        // Explicit auth rejection, must reach the module.
                        return true;
                    }
                    // It's an event, but not successauth.
                    return false;
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
            Message::Close(_) => true,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_rule_routes_auth_rejections() {
        let rule = InitRule::new();
        for event in AUTH_REJECTED_EVENTS {
            let msg = Message::text(format!(r#"42["{event}"]"#));
            assert!(
                rule.call(&msg),
                "`{event}` must be routed to InitModule so the rejection is surfaced"
            );
        }
    }

    #[test]
    fn init_rule_ignores_unrelated_events() {
        let rule = InitRule::new();
        assert!(!rule.call(&Message::text(r#"42["updateAssets",{}]"#)));
    }

    #[test]
    fn init_rule_routes_two_step_successauth() {
        let rule = InitRule::new();

        // Header announcing the binary attachment must reach the module: it is the
        // only frame that still reliably says "authentication succeeded".
        let header = Message::text(r#"451-["successauth",{"_placeholder":true,"num":0}]"#);
        assert!(
            rule.call(&header),
            "the successauth header must be routed to InitModule"
        );
        // And the attachment that follows it.
        assert!(
            rule.call(&Message::Binary(
                r#"{"id":"abc","server":"api-try-demo-2"}"#.into()
            )),
            "the successauth binary half must be routed too"
        );
    }

    #[test]
    fn auth_confirmation_accepts_every_server_payload_shape() {
        // Payload shapes seen across server versions.
        for payload in [
            r#"{"id":"idxacaSXClvO64GWAD2i"}"#,
            r#"{"id":"I2USehzmo4gjJF7IAsbl","server":"api-try-demo-2 CLIENT STREAMER_3"}"#,
            r#"{"serverName":"api-try-demo-2"}"#,
        ] {
            assert!(
                is_auth_confirmation(payload),
                "`{payload}` is a successauth confirmation"
            );
        }
    }

    #[test]
    fn auth_confirmation_rejects_other_payloads() {
        for payload in [
            r##"[[5,"#AAPL","Apple"]]"##,
            r#"{"isDemo":1,"balance":50000}"#,
            // A deal carries an `id` too, but it is not a bare-id payload.
            r#"{"id":"2f561661-334c-4de3-920f-f095c7b1193f","asset":"EURUSD_otc","amount":1}"#,
            "not json",
            "0",
        ] {
            assert!(
                !is_auth_confirmation(payload),
                "`{payload}` must not be mistaken for successauth"
            );
        }
    }

    #[tokio::test]
    async fn guest_session_requests_the_snapshot_without_successauth() {
        use crate::pocketoption::ssid::Ssid;
        use crate::pocketoption::state::StateBuilder;
        use binary_options_tools_core::reimports::bounded_async;
        use std::time::Duration;

        let ssid = Ssid::parse(r#"42["auth",{"token":"9PRxPzq6mQ","balance":50000}]"#).unwrap();
        let state = Arc::new(StateBuilder::default().ssid(ssid).build().unwrap());
        let (ws_tx, ws_rx) = bounded_async::<Message>(64);
        let (msg_tx, msg_rx) = bounded_async::<Arc<Message>>(64);
        let (runner_tx, _runner_rx) = bounded_async::<RunnerCommand>(4);

        let mut module = InitModule::new(state, ws_tx, msg_rx, runner_tx);
        tokio::spawn(async move {
            let _ = module.run().await;
        });

        msg_tx
            .send(Arc::new(Message::text(
                r#"0{"sid":"engine","upgrades":[],"pingInterval":25000,"pingTimeout":20000}"#,
            )))
            .await
            .unwrap();
        assert_eq!(recv_text(&ws_rx).await, "40");

        msg_tx
            .send(Arc::new(Message::text(r#"40{"sid":"socket"}"#)))
            .await
            .unwrap();
        assert!(recv_text(&ws_rx).await.starts_with(r#"42["auth""#));

        // Guest servers never confirm the auth frame; the grace timer must still
        // request the snapshot so candles and ticks work.
        let mut sent = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), async {
            while sent.len() < 7 {
                sent.push(recv_text(&ws_rx).await);
            }
        })
        .await
        .expect("the snapshot must be requested without a successauth confirmation");

        assert!(sent.iter().any(|m| m.contains("getBalance")));
        assert!(sent.iter().any(|m| m.contains("changeSymbol")));
    }

    #[tokio::test]
    async fn successauth_sends_the_snapshot_once() {
        use crate::pocketoption::ssid::Ssid;
        use crate::pocketoption::state::StateBuilder;
        use binary_options_tools_core::reimports::bounded_async;
        use std::time::Duration;

        let ssid =
            Ssid::parse(r#"42["auth",{"session":"abc","isDemo":1,"uid":1,"platform":2}]"#).unwrap();
        let state = Arc::new(StateBuilder::default().ssid(ssid).build().unwrap());
        let (ws_tx, ws_rx) = bounded_async::<Message>(64);
        let (msg_tx, msg_rx) = bounded_async::<Arc<Message>>(64);
        let (runner_tx, _runner_rx) = bounded_async::<RunnerCommand>(4);

        let mut module = InitModule::new(state, ws_tx, msg_rx, runner_tx);
        tokio::spawn(async move {
            let _ = module.run().await;
        });

        msg_tx
            .send(Arc::new(Message::text(
                r#"0{"sid":"engine","upgrades":[],"pingInterval":25000}"#,
            )))
            .await
            .unwrap();
        assert_eq!(recv_text(&ws_rx).await, "40");
        msg_tx
            .send(Arc::new(Message::text(r#"40{"sid":"socket"}"#)))
            .await
            .unwrap();
        assert!(recv_text(&ws_rx).await.starts_with(r#"42["auth""#));

        msg_tx
            .send(Arc::new(Message::text(
                r#"451-["successauth",{"_placeholder":true,"num":0}]"#,
            )))
            .await
            .unwrap();
        msg_tx
            .send(Arc::new(Message::Binary(r#"{"id":"socket"}"#.into())))
            .await
            .unwrap();

        let mut sent = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), async {
            while sent.len() < 7 {
                sent.push(recv_text(&ws_rx).await);
            }
        })
        .await
        .expect("successauth must trigger the snapshot");

        // The binary half arrives after the header and must not repeat the snapshot.
        tokio::time::sleep(Duration::from_secs(2)).await;
        let extra = tokio::time::timeout(Duration::from_millis(500), recv_text(&ws_rx)).await;
        assert!(
            extra.is_err(),
            "the snapshot must be requested exactly once per connection, extra: {:?}",
            extra.ok()
        );
    }

    /// Receives the next outbound frame as text.
    async fn recv_text(receiver: &AsyncReceiver<Message>) -> String {
        match receiver.recv().await.unwrap() {
            Message::Text(text) => text.to_string(),
            other => panic!("expected a text frame, got {other:?}"),
        }
    }
}
