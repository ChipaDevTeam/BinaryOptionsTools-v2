use crate::expertoptions::modules::Command;
use crate::expertoptions::types::MultiRule;
use crate::utils::serialize::bool2int;

use std::sync::Arc;

use binary_options_tools_core_pre::error::{CoreError, CoreResult};
use binary_options_tools_core_pre::reimports::{AsyncReceiver, AsyncSender, Message};
use binary_options_tools_core_pre::traits::{ApiModule, ReconnectCallback, Rule};
use binary_options_tools_macros::ActionImpl;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::select;
use tracing::debug;

use crate::expertoptions::state::State;
use crate::expertoptions::{Action, ActionName};

#[derive(Debug)]
pub enum Request {
    SetContext(Demo),
}

#[derive(Debug)]
pub enum Response {
    Success,
    Error(String),
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct ProfileHandle {
    sender: AsyncSender<Command<Request>>,
    receiver: AsyncReceiver<Command<Response>>,
}

impl ProfileHandle {
    /// Request switching context to demo/real. Fire-and-forget.
    pub async fn set_context(&self, is_demo: bool) -> CoreResult<()> {
        let (_, cmd) = Command::new(Request::SetContext(Demo::new(is_demo)));
        self.sender.send(cmd).await?;
        Ok(())
    }
}
/// Profile module for maintaining session activity
/// Send the original connection messages, and handles changes from real to demo accounts
pub struct ProfileModule {
    ws_receiver: AsyncReceiver<Arc<Message>>,
    ws_sender: AsyncSender<Message>,
    command_receiver: AsyncReceiver<Command<Request>>,
    command_responder: AsyncSender<Command<Response>>,
    /// The current state of the module
    state: Arc<State>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ActionImpl)]
#[action(name = "setContext")]
pub struct Demo {
    #[serde(with = "bool2int")]
    is_demo: bool,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Res {
    result: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
#[serde(untagged)]
enum ProfileResponse {
    Change(Res),
}

impl Demo {
    pub fn new(is_demo: bool) -> Self {
        Demo { is_demo }
    }

    pub fn to_demo(self) -> Self {
        Demo { is_demo: true }
    }

    pub fn to_real(self) -> Self {
        Demo { is_demo: false }
    }

    pub fn is_demo(&self) -> bool {
        self.is_demo
    }
}

#[async_trait::async_trait]
impl ApiModule<State> for ProfileModule {
    type Command = Command<Request>;
    type CommandResponse = Command<Response>;
    type Handle = ProfileHandle;

    fn new(
        shared_state: Arc<State>,
        command_receiver: AsyncReceiver<Self::Command>,
        command_responder: AsyncSender<Self::CommandResponse>,
        message_receiver: AsyncReceiver<Arc<Message>>,
        to_ws_sender: AsyncSender<Message>,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            ws_receiver: message_receiver,
            ws_sender: to_ws_sender,
            command_receiver,
            command_responder,
            state: shared_state,
        }
    }

    /// Creates a new handle for this module.
    /// This is used to send commands to the module.
    ///
    /// # Arguments
    /// * `sender`: The sender channel for commands.
    /// * `receiver`: The receiver channel for command responses.
    fn create_handle(
        sender: AsyncSender<Self::Command>,
        receiver: AsyncReceiver<Self::CommandResponse>,
    ) -> Self::Handle {
        ProfileHandle { sender, receiver }
    }

    /// The main run loop for the module's background task.
    async fn run(&mut self) -> CoreResult<()> {
        // Send initial multipleAction and ensure demo context on first run
        self.send_startup_messages().await?;

        loop {
            select! {
                Ok(msg) = self.ws_receiver.recv() => {
            if let Message::Binary(data) = msg.as_ref() {
                        // Handle specific profile response variants if needed
                        match Action::from_json::<ProfileResponse>(data) {
                            Ok(_res) => {
                                // Currently ignored; extend when needed
                            },
                            Err(e) => {
                                // Not all messages are Profile responses; keep quiet unless parse looked relevant
                                debug!(target: "ProfileModule", "Non-profile or unparsable message: {}", e);
                            }
                        }
                    }
                },
                Ok(cmd) = self.command_receiver.recv() => {
                    match cmd.data() {
                        Request::SetContext(demo) => {
                            // Update state and send setContext
                self.state.set_demo(demo.clone()).await;
                            let token = self.state.token.clone();
                let msg = demo.clone().action(token).map_err(|e| CoreError::Other(e.to_string()))?.to_message()?;
                            self.ws_sender.send(msg).await?;
                            // For now always respond with Success
                            self.command_responder.send(Command::new(Response::Success).1).await?;
                        }
                    }
                }
            }
        }
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiRule::new(vec![Box::new(MultipleActionRule)]))
    }

    fn callback(
        &self,
    ) -> binary_options_tools_core_pre::error::CoreResult<Option<Box<dyn ReconnectCallback<State>>>>
    {
        struct CB;
        #[async_trait::async_trait]
        impl ReconnectCallback<State> for CB {
            async fn call(
                &self,
                state: Arc<State>,
                ws_sender: &AsyncSender<Message>,
            ) -> CoreResult<()> {
                // On reconnect, re-send multipleAction and ensure context if demo
                let token = state.token.clone();
                let timezone = state.timezone.read().await;
                let multi = multiple_action_action(token.clone(), *timezone)?.to_message()?;
                ws_sender.send(multi).await?;
                if state.is_demo().await {
                    let demo = Demo::new(true);
                    let msg = demo
                        .action(token)
                        .map_err(|e| CoreError::Other(e.to_string()))?
                        .to_message()?;
                    ws_sender.send(msg).await?;
                }
                Ok(())
            }
        }
        Ok(Some(Box::new(CB)))
    }
}

impl ProfileModule {
    async fn send_startup_messages(&self) -> CoreResult<()> {
        let token = self.state.token.clone();
        let timezone = self.state.timezone.read().await;
        // Send multipleAction with basic actions placeholder (can be extended)
        let multi = multiple_action_action(token.clone(), *timezone)?.to_message()?;
        self.ws_sender.send(multi).await?;
        // Ensure demo context if currently demo
        if self.state.is_demo().await {
            let demo = Demo::new(true);
            let msg = demo
                .action(token)
                .map_err(|e| CoreError::Other(e.to_string()))?
                .to_message()?;
            self.ws_sender.send(msg).await?;
        }
        Ok(())
    }
}

/// Build a multipleAction Action with a minimal placeholder payload.
pub fn multiple_action_action(
    token: String,
    timezone: i32,
) -> binary_options_tools_core_pre::error::CoreResult<Action> {
    // Placeholder minimal structure; extend actions list as needed
    let payload = json!({"actions":[
        {"action":"userGroup","ns":1,"token":token},
        {"action":"profile","ns":2,"token":token},
        {"action":"assets","ns":3,"token":token},
        {"action":"getCurrency","ns":4,"token":token},
        {"action":"getCountries","ns":5,"token":token},
        {"action":"environment","message":{"supportedFeatures":["achievements","trade_result_share","tournaments","referral","twofa","inventory","deposit_withdrawal_error_handling","report_a_problem_form","ftt_trade","stocks_trade","stocks_trade_demo","predictions_trade","predictions_trade_demo"],"supportedAbTests":["tournament_glow","floating_exp_time","tutorial","tutorial_account_type","tutorial_account_type_reg","tutorial_stocks","tutorial_first_deal","tutorial_predictions","hide_education_section","in_app_update_android_3","auto_consent_reg","battles_4th_5th_place_rewards","show_achievements_bottom_sheet","promo_story_priority","force_lang_in_app","one_click_deposit","app_theme_select","achievents_badge","chart_hide_soc_trade","candles_autozoom_off","ra_welcome_popup","required_report_msg","2fa_hide_havecode_msg","show_welcome_screen_learn_earn","confirm_event_deals"],"supportedInventoryItems":["riskless_deal","profit","eopoints","tournaments_prize_x3","mystery_box","special_deposit_bonus","cashback_offer"]},"ns":6,"token":token},
        {"action":"defaultSubscribeCandles","message":{"timeframes":[0,5]},"ns":7,"token":token},
        {"action":"setTimeZone","message":{"timeZone":timezone},"ns":8,"token":token},
        {"action":"getCandlesTimeframes","ns":9,"token":token}
    ]});
    Ok(Action::new("multipleAction".to_string(), token, 2, payload))
}

/// Rule that matches messages containing the string "multipleAction".
struct MultipleActionRule;

impl Rule for MultipleActionRule {
    fn call(&self, msg: &Message) -> bool {
        match msg {
            Message::Binary(data) => {
                // quick substring check to avoid full JSON parse
                if let Ok(s) = std::str::from_utf8(data) {
                    s.contains("\"action\":\"multipleAction\"") || s.contains("multipleAction")
                } else {
                    false
                }
            }
            Message::Text(s) => s.contains("multipleAction"),
            _ => false,
        }
    }

    fn reset(&self) { /* stateless */
    }
}
