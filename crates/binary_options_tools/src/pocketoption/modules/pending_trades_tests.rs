#![allow(warnings)]
#![allow(unused_imports)]
use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools_core::{
    error::CoreError,
    reimports::{AsyncReceiver, AsyncSender, Message},
    traits::{ApiModule, Rule, RunnerCommand},
};
use kanal::bounded_async;
use rust_decimal::Decimal;
use tokio::{
    sync::Mutex,
    time::{timeout, Instant},
};
use uuid::Uuid;

use crate::pocketoption::modules::pending_trades::ServerResponse;
use crate::pocketoption::{
    error::{PocketError, PocketResult},
    state::{State, TradeState},
    types::{FailOpenOrder, MultiPatternRule, OpenPendingOrder, PendingOrder},
};

use crate::pocketoption::modules::pending_trades::{
    Command, CommandResponse, PendingTradesApiModule, PendingTradesHandle,
};

// ============== Mock Helpers ==============

/// Creates a minimal mock State with only the fields needed for testing
fn create_mock_state() -> Arc<State> {
    use crate::pocketoption::ssid::{Demo, Ssid};
    use crate::pocketoption::types::ServerTimeState;
    use std::collections::HashMap;
    // Construct a Demo SSID directly
    let demo_ssid = Ssid::Demo(Demo {
        session: "test_session_id".to_string(),
        is_demo: 1,
        uid: 12345,
        platform: 2,
        current_url: None,
        is_fast_history: None,
        is_optimized: None,
        raw: String::new(),
        json_raw: String::new(),
        extra: HashMap::new(),
    });
    Arc::new(State {
        ssid: demo_ssid,
        default_connection_url: None,
        default_symbol: "EURUSD_otc".to_string(),
        balance: tokio::sync::RwLock::new(None),
        balance_updated: Arc::new(tokio::sync::Notify::new()),
        server_time: ServerTimeState::default(),
        assets: tokio::sync::RwLock::new(None),
        assets_updated: Arc::new(tokio::sync::Notify::new()),
        trade_state: Arc::new(TradeState::default()),
        raw_validators: std::sync::RwLock::new(HashMap::new()),
        active_subscriptions: tokio::sync::RwLock::new(HashMap::new()),
        histories: tokio::sync::RwLock::new(Vec::new()),
        raw_sinks: tokio::sync::RwLock::new(HashMap::new()),
        raw_keep_alive: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        urls: Vec::new(),
    })
}

/// Creates a PendingOrder with test data
fn create_test_pending_order(req_id: Uuid) -> PendingOrder {
    PendingOrder {
        ticket: req_id,
        open_type: 1,
        amount: Decimal::from_f64_retain(100.0).unwrap(),
        symbol: "EURUSD_otc".to_string(),
        open_time: "2024-01-01 10:00:00".to_string(),
        open_price: Decimal::from_f64_retain(1.1950).unwrap(),
        timeframe: 60,
        min_payout: 85,
        command: 0,
        date_created: "2024-01-01 10:00:00".to_string(),
        id: 12345,
    }
}

/// Creates a FailOpenOrder with test data
fn create_test_fail_open_order() -> FailOpenOrder {
    FailOpenOrder {
        error: "Insufficient balance".to_string(),
        amount: Decimal::from_f64_retain(100.0).unwrap(),
        asset: "EURUSD_otc".to_string(),
    }
}

/// Creates a WebSocket text message with Socket.IO framing: 42["event", {...}]
fn create_socket_io_text_message(event: &str, data: &serde_json::Value) -> String {
    format!(
        "42[{},{}]",
        serde_json::to_string(event).unwrap(),
        serde_json::to_string(data).unwrap()
    )
}

/// Creates a binary message with JSON data
fn create_binary_message(data: &serde_json::Value) -> Message {
    Message::Binary(serde_json::to_vec(data).unwrap().into())
}

/// Creates a text message with JSON data
fn create_text_message(data: &serde_json::Value) -> Message {
    Message::Text(serde_json::to_string(data).unwrap().into())
}

// ============== Tests for PendingTradesHandle::open_pending_order ==============

#[tokio::test]
async fn test_open_pending_order_success_integrated() {
    // Channel setup
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, ws_rx) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let req_id = Uuid::new_v4();
    let pending_order = create_test_pending_order(req_id);

    // Spawn the open_pending_order call to allow the module to process the command first
    let result_handle = tokio::spawn(async move {
        client_handle
            .open_pending_order(
                1,
                Decimal::from_f64_retain(100.0).unwrap(),
                "EURUSD_otc".to_string(),
                60,
                Decimal::from_f64_retain(1.1950).unwrap(),
                60,
                85,
                0,
            )
            .await
    });

    // Wait for the command to be processed by the module
    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let response_json = serde_json::to_string(&server_response).unwrap();

    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    let result = result_handle.await.unwrap();

    assert!(result.is_ok());
    let received_order = result.unwrap();
    assert_eq!(received_order.ticket, pending_order.ticket);
    assert_eq!(received_order.amount, pending_order.amount);

    let pending_deals = state.trade_state.get_pending_deals().await;
    assert_eq!(pending_deals.len(), 1);
    assert!(pending_deals.contains_key(&pending_order.ticket));

    module_task.abort();
}

#[tokio::test]
async fn test_open_pending_order_failure() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, ws_rx) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx,
        runner_tx,
    );

    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let fail_order = create_test_fail_open_order();
    let server_response = ServerResponse::Fail(Box::new(fail_order.clone()));
    let response_json = serde_json::to_string(&server_response).unwrap();

    // Spawn the open_pending_order call to allow the module to process the command first
    let result_handle = tokio::spawn(async move {
        client_handle
            .open_pending_order(
                1,
                Decimal::from_f64_retain(100.0).unwrap(),
                "EURUSD_otc".to_string(),
                60,
                Decimal::from_f64_retain(1.1950).unwrap(),
                60,
                85,
                0,
            )
            .await
    });

    // Wait for the command to be processed by the module
    tokio::time::sleep(Duration::from_millis(10)).await;

    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    let result = result_handle.await.unwrap();

    assert!(result.is_err());
    match result.unwrap_err() {
        PocketError::FailOpenOrder {
            error,
            amount,
            asset,
        } => {
            assert_eq!(error, fail_order.error);
            assert_eq!(amount, fail_order.amount);
            assert_eq!(asset, fail_order.asset);
        }
        _ => panic!("Expected FailOpenOrder error"),
    }

    module_task.abort();
}

#[tokio::test]
async fn test_open_pending_order_mismatch_retry() {
    // Direct handle test without module
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);

    let handle = PendingTradesHandle::new(cmd_tx, resp_rx);

    let cmd_rx_clone = cmd_rx.clone();
    let command_task = tokio::spawn(async move {
        let mut cmd_rx = cmd_rx_clone;
        // Receive the command and return it to capture the req_id
        cmd_rx.recv().await.unwrap()
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());

    let result_handle = tokio::spawn(async move {
        handle
            .open_pending_order(
                1,
                Decimal::from_f64_retain(100.0).unwrap(),
                "EURUSD_otc".to_string(),
                60,
                Decimal::from_f64_retain(1.1950).unwrap(),
                60,
                85,
                0,
            )
            .await
    });

    // Wait for the command to be sent and capture it
    let cmd = command_task.await.unwrap();
    let req_id = match cmd {
        Command::OpenPendingOrder { req_id, .. } => req_id,
    };

    // Send two mismatched responses followed by the correct one
    let wrong_id1 = Uuid::new_v4();
    let wrong_id2 = Uuid::new_v4();
    let resp1: CommandResponse = CommandResponse::Success {
        req_id: wrong_id1,
        pending_order: Box::new(pending_order.clone()),
    };
    let resp2: CommandResponse = CommandResponse::Success {
        req_id: wrong_id2,
        pending_order: Box::new(pending_order.clone()),
    };
    let resp3: CommandResponse = CommandResponse::Success {
        req_id: req_id,
        pending_order: Box::new(pending_order.clone()),
    };

    resp_tx.send(resp1).await.unwrap();
    resp_tx.send(resp2).await.unwrap();
    resp_tx.send(resp3).await.unwrap();

    let result = result_handle.await.unwrap();
    assert!(result.is_ok());
    let received = result.unwrap();
    assert_eq!(received.ticket, pending_order.ticket);
}

#[tokio::test]
async fn test_open_pending_order_mismatch_max_retries_exceeded() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);

    let handle = PendingTradesHandle::new(cmd_tx, resp_rx);

    let cmd_rx_clone = cmd_rx.clone();
    let command_task = tokio::spawn(async move {
        let mut cmd_rx = cmd_rx_clone;
        let _ = cmd_rx.recv().await;
    });

    let result_handle = tokio::spawn(async move {
        handle
            .open_pending_order(
                1,
                Decimal::from_f64_retain(100.0).unwrap(),
                "EURUSD_otc".to_string(),
                60,
                Decimal::from_f64_retain(1.1950).unwrap(),
                60,
                85,
                0,
            )
            .await
    });

    let _cmd = command_task.await.unwrap();

    // Send 5 mismatches
    for _ in 0..5 {
        let wrong_id = Uuid::new_v4();
        let resp: CommandResponse = CommandResponse::Success {
            req_id: wrong_id,
            pending_order: Box::new(create_test_pending_order(Uuid::new_v4())),
        };
        resp_tx.send(resp).await.unwrap();
    }

    let result = result_handle.await.unwrap();
    assert!(matches!(result, Err(PocketError::Timeout { .. })));
}

#[tokio::test]
async fn test_open_pending_order_channel_error_sender_closed() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module =
        PendingTradesApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);

    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    // Abort the task to drop the module and close channels
    module_task.abort();
    // Wait for task to finish
    let _ = module_task.await;

    let result = client_handle
        .open_pending_order(
            1,
            Decimal::from_f64_retain(100.0).unwrap(),
            "EURUSD_otc".to_string(),
            60,
            Decimal::from_f64_retain(1.1950).unwrap(),
            60,
            85,
            0,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PocketError::Core(_) => {}
        _ => panic!("Expected Core error from channel"),
    }
}

#[tokio::test]
async fn test_open_pending_order_with_socket_io_framing() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, ws_rx) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx,
        runner_tx,
    );

    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());

    // Spawn the open_pending_order call to allow the module to process the command first
    let result_handle = tokio::spawn(async move {
        client_handle
            .open_pending_order(
                1,
                Decimal::from_f64_retain(100.0).unwrap(),
                "EURUSD_otc".to_string(),
                60,
                Decimal::from_f64_retain(1.1950).unwrap(),
                60,
                85,
                0,
            )
            .await
    });

    // Wait for the command to be processed by the module
    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let data_json = serde_json::to_string(&server_response).unwrap();
    let socket_io_msg = format!("42[\"successopenPendingOrder\",{}]", data_json);
    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
        .await
        .unwrap();

    let result = result_handle.await.unwrap();

    assert!(result.is_ok());
    let received = result.unwrap();
    assert_eq!(received.ticket, pending_order.ticket);

    module_task.abort();
}

// ============== Tests for PendingTradesApiModule::run ==============

#[tokio::test]
async fn test_run_routes_command_to_websocket() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, _) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, ws_rx) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx,
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let open_order = OpenPendingOrder {
        open_type: 1,
        amount: Decimal::from_f64_retain(100.0).unwrap(),
        asset: "EURUSD_otc".to_string(),
        open_time: 60,
        open_price: Decimal::from_f64_retain(1.1950).unwrap(),
        timeframe: 60,
        min_payout: 85,
        command: 0,
    };
    let expected_ws_message = open_order.to_string();

    let cmd_tx_clone = cmd_tx.clone();
    tokio::spawn(async move {
        let _ = cmd_tx_clone
            .send(Command::OpenPendingOrder {
                open_type: 1,
                amount: Decimal::from_f64_retain(100.0).unwrap(),
                asset: "EURUSD_otc".to_string(),
                open_time: 60,
                open_price: Decimal::from_f64_retain(1.1950).unwrap(),
                timeframe: 60,
                min_payout: 85,
                command: 0,
                req_id: Uuid::new_v4(),
            })
            .await;
    });

    let ws_msg = ws_rx.recv().await.unwrap();
    assert_eq!(ws_msg.to_string(), expected_ws_message);

    module_task.abort();
}

#[tokio::test]
async fn test_run_handles_binary_success_response() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, _) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module =
        PendingTradesApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());
    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let binary_data = serde_json::to_vec(&server_response).unwrap();
    msg_tx
        .send(Arc::new(Message::Binary(binary_data.into())))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let pending_deals = state.trade_state.get_pending_deals().await;
    assert_eq!(pending_deals.len(), 1);
    assert!(pending_deals.contains_key(&pending_order.ticket));

    module_task.abort();
}

#[tokio::test]
async fn test_run_handles_socket_io_text_success() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, ws_rx) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx,
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());
    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let data_json = serde_json::to_string(&server_response).unwrap();
    let socket_io_msg = format!("42[\"successopenPendingOrder\",{}]", data_json);

    let cmd_req_id = Uuid::new_v4();
    cmd_tx
        .send(Command::OpenPendingOrder {
            open_type: 1,
            amount: Decimal::from_f64_retain(100.0).unwrap(),
            asset: "EURUSD_otc".to_string(),
            open_time: 60,
            open_price: Decimal::from_f64_retain(1.1950).unwrap(),
            timeframe: 60,
            min_payout: 85,
            command: 0,
            req_id: cmd_req_id,
        })
        .await
        .unwrap();

    // Wait for the command to be processed by the module
    tokio::time::sleep(Duration::from_millis(10)).await;

    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
        .await
        .unwrap();

    let response = resp_rx.recv().await.unwrap();
    match response {
        CommandResponse::Success {
            req_id,
            pending_order: po,
        } => {
            assert_eq!(req_id, cmd_req_id);
            assert_eq!(po.ticket, pending_order.ticket);
        }
        _ => panic!("Unexpected response"),
    }

    module_task.abort();
}

#[tokio::test]
async fn test_run_handles_failure_response() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, ws_rx) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx,
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let fail_order = create_test_fail_open_order();
    let server_response = ServerResponse::Fail(Box::new(fail_order.clone()));
    let response_json = serde_json::to_string(&server_response).unwrap();

    let cmd_req_id = Uuid::new_v4();
    cmd_tx
        .send(Command::OpenPendingOrder {
            open_type: 1,
            amount: Decimal::from_f64_retain(100.0).unwrap(),
            asset: "EURUSD_otc".to_string(),
            open_time: 60,
            open_price: Decimal::from_f64_retain(1.1950).unwrap(),
            timeframe: 60,
            min_payout: 85,
            command: 0,
            req_id: cmd_req_id,
        })
        .await
        .unwrap();

    // Wait for the command to be processed by the module
    tokio::time::sleep(Duration::from_millis(10)).await;

    let socket_io_msg = format!("42[\"failopenPendingOrder\",{}]", response_json);
    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
        .await
        .unwrap();

    let response = resp_rx.recv().await.unwrap();
    match response {
        CommandResponse::Error(fail) => {
            assert_eq!(fail.error, fail_order.error);
            assert_eq!(fail.asset, fail_order.asset);
        }
        _ => panic!("Expected Error response"),
    }

    module_task.abort();
}

#[tokio::test]
async fn test_run_handles_deserialization_error() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, _) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module =
        PendingTradesApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let invalid_json = "invalid json data".to_string();
    msg_tx
        .send(Arc::new(Message::Text(invalid_json.into())))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    module_task.abort();
}

#[tokio::test]
async fn test_run_success_without_pending_request() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(1);
    let (runner_tx, _) = kanal::bounded_async(1);

    let state = create_mock_state();

    let mut module =
        PendingTradesApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());
    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let response_json = serde_json::to_string(&server_response).unwrap();
    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // No response should be sent because there was no pending request
    let recv_result = resp_rx.try_recv();
    assert!(matches!(recv_result, Ok(None)));

    let pending_deals = state.trade_state.get_pending_deals().await;
    assert_eq!(pending_deals.len(), 1);

    module_task.abort();
}

// ============== Tests for new, create_handle, rule ==============

#[test]
fn test_new_creates_module() {
    let state = create_mock_state();
    let (cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx) = {
        let (a, b) = kanal::bounded_async::<Command>(1);
        let (c, d) = kanal::bounded_async::<CommandResponse>(1);
        let (e, f) = kanal::bounded_async::<Arc<Message>>(1);
        let (g, h) = kanal::bounded_async::<Message>(1);
        let (i, j) = kanal::bounded_async::<RunnerCommand>(1);
        (b, c, f, g, i) // cmd_rx=b, resp_tx=c (sender), msg_rx=f, ws_tx=g (sender), runner_tx=i (sender)
    };

    let _module =
        PendingTradesApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);

    // Verify rule patterns by behavioral test
    let rule = PendingTradesApiModule::rule(state.clone());
    assert!(rule.call(&Message::Text("42[\"successopenPendingOrder\",{}]".into())));
    assert!(rule.call(&Message::Text("42[\"failopenPendingOrder\",{}]".into())));
    assert!(!rule.call(&Message::Text("42[\"unknown\",{}]".into())));
}

#[test]
fn test_create_handle_returns_valid_handle() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(1);

    let handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);
    let _handle2 = handle.clone();
}

#[test]
fn test_rule_returns_multi_pattern_rule() {
    let state = create_mock_state();
    let rule = PendingTradesApiModule::rule(state);
    // Verify rule patterns by behavioral test
    assert!(rule.call(&Message::Text("42[\"successopenPendingOrder\",{}]".into())));
    assert!(rule.call(&Message::Text("42[\"failopenPendingOrder\",{}]".into())));
    assert!(!rule.call(&Message::Text("42[\"unknown\",{}]".into())));
}
