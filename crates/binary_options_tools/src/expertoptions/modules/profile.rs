use crate::expertoptions::modules::Command;
use crate::expertoptions::types::{Asset, Assets, MultiRule};
use crate::utils::serialize::bool2int;

use std::collections::HashMap;
use std::sync::Arc;

use binary_options_tools_core_pre::error::{CoreError, CoreResult};
use binary_options_tools_core_pre::reimports::{AsyncReceiver, AsyncSender, Message};
use binary_options_tools_core_pre::traits::{ApiModule, ReconnectCallback, Rule, RunnerCommand};
use binary_options_tools_macros::ActionImpl;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::select;
use tracing::debug;

use crate::expertoptions::state::{Balance, State};
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

#[derive(Deserialize, Debug)]
struct ProfileAction {
    actions: Vec<Action>,
}

// List of ids for Action responses
const ASSETS: &str = "assets";
const PROFILE: &str = "profile";
const GET_CANDLES_TIMEFRAMES: &str = "getCandlesTimeFrames";

// List of structs to get important data
#[derive(Deserialize)]
struct Profile {
    demo_balance: Decimal,
    real_balance: Decimal,
    #[serde(with = "bool2int")]
    is_demo: bool,
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CandlesTimeFrames {
    candles_time_frames: Vec<u32>,
    points_timeframe: Decimal,
}

#[derive(Clone)]
pub struct ProfileHandle {
    sender: AsyncSender<Command<Request>>,
    receiver: AsyncReceiver<Command<Response>>,
}

impl ProfileHandle {
    /// Request switching context to demo/real. Fire-and-forget.
    pub async fn set_context(&self, is_demo: bool) -> CoreResult<()> {
        let (id, cmd) = Command::new(Request::SetContext(Demo::new(is_demo)));
        self.sender.send(cmd).await?;
        loop {
            match self.receiver.recv().await {
                Ok(cmd) => {
                    if id == cmd.id() {
                        match cmd.data() {
                            Response::Success => return Ok(()),
                            Response::Error(e) => return Err(CoreError::Other(e.to_string())),
                        }
                    }
                    // Continue waiting for the correct response
                }
                Err(e) => return Err(CoreError::from(e)),
            }
        }
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

#[derive(Deserialize, Debug)]
struct Res {
    result: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ProfileResponse {
    Change(Res),
    Profile(ProfileAction),
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

impl ProfileModule {
    async fn parse_profile(&self, actions: Vec<Action>) -> CoreResult<()> {
        for action in actions {
            match action.id() {
                ASSETS => {
                    // Handle assets response
                    let assets: HashMap<String, Vec<Asset>> = action.take()?;
                    let assets = Assets::new(
                        assets
                            .into_iter()
                            .next()
                            .ok_or_else(|| CoreError::Other("No assets found".to_string()))?
                            .1,
                    );
                    self.state.set_assets(assets).await;
                    // Process assets as needed
                }
                PROFILE => {
                    // Handle profile response
                    let profile_action: HashMap<String, Profile> = action.take()?;
                    let balance_profile = profile_action
                        .into_iter()
                        .next()
                        .ok_or_else(|| CoreError::Other("No profile found".to_string()))?
                        .1;
                    let balance = Balance {
                        demo: balance_profile.demo_balance,
                        real: balance_profile.real_balance,
                    };
                    self.state.set_balance(balance).await;
                    self.state
                        .set_demo(Demo::new(balance_profile.is_demo))
                        .await;
                    // Process profile data
                }
                GET_CANDLES_TIMEFRAMES => {
                    // Handle get candles timeframes response
                    let timeframes: CandlesTimeFrames = action.take()?;
                    self.state
                        .set_timeframes(timeframes.candles_time_frames, timeframes.points_timeframe)
                        .await;
                }
                _ => {
                    debug!("Unhandled action response: {}", action.id());
                }
            }
        }
        Ok(())
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
        _: AsyncSender<RunnerCommand>,
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
        println!("Here");
        self.send_startup_messages().await?;

        loop {
            select! {
                biased;
                msg_res = self.ws_receiver.recv() => {
                    match msg_res {
                        Ok(msg) => {
                            if let Message::Binary(data) = msg.as_ref() {
                                // Handle specific profile response variants if needed
                                match Action::from_json::<ProfileResponse>(data) {
                                    Ok(res) => {
                                        match res {
                                            ProfileResponse::Change(res) => {
                                                debug!(target: "ProfileModule", "Profile mode changed: {}", res.result);
                                            }
                                            ProfileResponse::Profile(profile) => {
                                                debug!(target: "ProfileModule", "Profile received: {:?}", profile);
                                                self.parse_profile(profile.actions).await?;
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        // Not all messages are Profile responses; keep quiet unless parse looked relevant
                                        debug!(target: "ProfileModule", "Non-profile or unparsable message: {}", e);
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                },
                cmd_res = self.command_receiver.recv() => {
                    match cmd_res {
                        Ok(cmd) => {
                            let id = cmd.id();
                            match cmd.data() {
                                Request::SetContext(demo) => {
                                    // Update state and send setContext
                                    self.state.set_demo(demo.clone()).await;
                                    let token = self.state.token.clone();
                                    let msg = demo.clone().action(token).map_err(|e| CoreError::Other(e.to_string()))?.to_message()?;
                                    self.ws_sender.send(msg).await?;
                                    // For now always respond with Success
                                    self.command_responder.send(Command::from_id(id, Response::Success)).await?;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        Ok(())
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiRule::new(vec![Box::new(MultipleActionRule)]))
    }

    fn callback(
        _shared_state: Arc<State>,
        _command_receiver: AsyncReceiver<Self::Command>,
        _command_responder: AsyncSender<Self::CommandResponse>,
        _message_receiver: AsyncReceiver<Arc<Message>>,
        _to_ws_sender: AsyncSender<Message>,
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
        // Ensure demo context if currently demo
        if dbg!(self.state.is_demo().await) {
            dbg!("Sent demo message");
            let demo = Demo::new(true);
            let msg = demo
                .action(token.clone())
                .map_err(|e| CoreError::Other(e.to_string()))?
                .to_message()?;
            self.ws_sender.send(msg).await?;
        }
        // Send multipleAction with basic actions placeholder (can be extended)
        let multi = multiple_action_action(token, *timezone)?.to_message()?;
        self.ws_sender.send(multi).await?;
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
        {"action":"getCurrency","ns":2,"token":token},
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
