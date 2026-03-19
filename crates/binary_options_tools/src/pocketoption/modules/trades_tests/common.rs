#![allow(warnings)]
#![allow(unused_imports)]
use binary_options_tools_core::{
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, RunnerCommand},
};
use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::time::timeout;
use uuid::Uuid;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    ssid::{Real, SessionData, Ssid as PocketSsid},
    state::{State, StateBuilder, TradeState},
    types::{Action, Deal, FailOpenOrder, OpenOrder},
};

use crate::pocketoption::modules::trades::{
    Command, CommandResponse, TradesApiModule, TradesHandle,
};

// Helper to create a minimal mock State for testing
pub fn create_mock_state() -> Arc<State> {
    // Construct a real SSID (non-demo) for testing
    let real_ssid = PocketSsid::Real(Real {
        session: SessionData {
            session_id: "test_session".to_string(),
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            last_activity: 0,
        },
        session_raw: "dummy".to_string(),
        is_demo: 0,
        uid: 12345,
        platform: 2,
        raw: "dummy".to_string(),
        json_raw: "dummy".to_string(),
        is_fast_history: None,
        is_optimized: None,
        extra: std::collections::HashMap::new(),
    });
    let state = StateBuilder::default()
        .ssid(real_ssid)
        .default_symbol("EURUSD_otc".to_string())
        .build()
        .unwrap();
    Arc::new(state)
}

// Helper to create test deal
pub fn create_test_deal(req_id: Uuid, asset: &str) -> Deal {
    Deal {
        id: Uuid::new_v4(),
        open_time: "2024-01-01 10:00:00".to_string(),
        close_time: "2024-01-01 10:01:00".to_string(),
        open_timestamp: chrono::Utc::now(),
        close_timestamp: chrono::Utc::now(),
        refund_time: None,
        refund_timestamp: None,
        uid: 123456789,
        request_id: Some(req_id),
        amount: Decimal::from(10),
        profit: Decimal::from(5),
        percent_profit: 50,
        percent_loss: -100,
        open_price: Decimal::from(1_2345),
        close_price: Decimal::from(1_2350),
        command: 0,
        asset: asset.to_string(),
        is_demo: 1,
        copy_ticket: "".to_string(),
        open_ms: 0,
        close_ms: None,
        option_type: 100,
        is_rollover: None,
        is_copy_signal: None,
        is_ai: None,
        currency: "USD".to_string(),
        amount_usd: None,
        amount_usd2: None,
    }
}

// Helper to create test fail response
pub fn create_test_fail(asset: &str, amount: Decimal) -> FailOpenOrder {
    FailOpenOrder {
        error: "Insufficient balance".to_string(),
        amount,
        asset: asset.to_string(),
    }
}

// Helper to send a message to the module's message receiver
async fn send_message_to_module(msg: Message, sender: &AsyncSender<Message>) {
    let _ = sender.send(msg).await;
}

// Helper to create message channels and module
async fn setup_module_with_msg_tx() -> (
    Arc<State>,
    TradesHandle,
    AsyncSender<Command>,
    AsyncSender<Arc<Message>>, // This is msg_tx to send TO module
    AsyncReceiver<Message>,    // This is ws_rx to read messages sent TO WebSocket
) {
    let state = create_mock_state();

    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(100);
    let (cmd_resp_tx, cmd_resp_rx) = kanal::bounded_async::<CommandResponse>(100);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(100);
    let (ws_tx, ws_rx) = kanal::bounded_async::<Message>(100);
    let (_runner_tx, _runner_rx) = kanal::bounded_async::<RunnerCommand>(1);

    let mut module = TradesApiModule::new(
        state.clone(),
        cmd_rx,
        cmd_resp_tx,
        msg_rx,
        ws_tx,
        _runner_tx,
    );

    let handle = TradesApiModule::create_handle(cmd_tx.clone(), cmd_resp_rx);

    tokio::spawn(async move {
        let _ = module.run().await;
    });

    (state, handle, cmd_tx, msg_tx, ws_rx)
}

// Let's create a more flexible setup that returns all necessary channels
pub struct TestSetup {
    pub state: Arc<State>,
    pub handle: TradesHandle,
    pub cmd_tx: AsyncSender<Command>,
    pub msg_tx: AsyncSender<Arc<Message>>,
    pub ws_tx: AsyncSender<Message>,
    pub ws_rx: AsyncReceiver<Message>,
}

pub async fn create_test_setup() -> TestSetup {
    let state = create_mock_state();

    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(100);
    let (cmd_resp_tx, cmd_resp_rx) = kanal::bounded_async::<CommandResponse>(100);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(100);
    let (ws_tx, ws_rx) = kanal::bounded_async::<Message>(100);
    let (runner_tx, runner_rx) = kanal::bounded_async::<RunnerCommand>(1);

    let mut module = TradesApiModule::new(
        state.clone(),
        cmd_rx,
        cmd_resp_tx,
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let handle = TradesApiModule::create_handle(cmd_tx.clone(), cmd_resp_rx);

    tokio::spawn(async move {
        let _ = module.run().await;
    });

    TestSetup {
        state,
        handle,
        cmd_tx,
        msg_tx,
        ws_tx,
        ws_rx,
    }
}
