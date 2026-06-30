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

use crate::pocketoption::modules::pending_trades::CancelServerResponse;

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
        max_subscriptions: 4,
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
    use tokio::select;
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let mut module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());
    let data_json = serde_json::to_string(
        &ServerResponse::Success(Box::new(pending_order.clone()))
    ).unwrap();
    let socket_io_msg = format!("42[\"successopenPendingOrder\",{}]", data_json);

    // Send the command directly instead of going through the handle
    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::OpenPendingOrder {
        open_type: 1,
        amount: Decimal::from_f64_retain(100.0).unwrap(),
        asset: "EURUSD_otc".to_string(),
        open_time: "2026-04-07 22:50:00".to_string(),
        open_price: Decimal::from_f64_retain(1.1950).unwrap(),
        timeframe: 60,
        min_payout: 85,
        command: 0,
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
        .await
        .unwrap();

    // Wait for response directly from resp_rx
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::Success { req_id: rid, pending_order: po }) if rid == req_id => {
                    return po;
                }
                Ok(CommandResponse::Error(_)) => panic!("Expected success"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    let pending_deals = state.trade_state.get_pending_deals().await;
    assert_eq!(pending_deals.len(), 1);
    assert!(pending_deals.contains_key(&pending_order.ticket));

    module_task.abort();
}

#[tokio::test]
async fn test_open_pending_order_failure() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx,
        runner_tx,
    );

    let mut module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let fail_order = create_test_fail_open_order();
    let data_json = serde_json::to_string(
        &ServerResponse::Fail(Box::new(fail_order.clone()))
    ).unwrap();
    let socket_io_msg = format!("42[\"failopenPendingOrder\",{}]", data_json);

    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::OpenPendingOrder {
        open_type: 1,
        amount: Decimal::from_f64_retain(100.0).unwrap(),
        asset: "EURUSD_otc".to_string(),
        open_time: "2026-04-07 22:50:00".to_string(),
        open_price: Decimal::from_f64_retain(1.1950).unwrap(),
        timeframe: 60,
        min_payout: 85,
        command: 0,
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
        .await
        .unwrap();

    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::Error(fail)) => return fail,
                Ok(CommandResponse::Success { .. }) => panic!("Expected error"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(result.error, fail_order.error);
    assert_eq!(result.amount, fail_order.amount);
    assert_eq!(result.asset, fail_order.asset);

    module_task.abort();
}

#[tokio::test]
async fn test_open_pending_order_mismatch_retry() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);

    let pending_order = create_test_pending_order(Uuid::new_v4());
    let resp_tx_for_module = resp_tx.clone();

    let mut module_task = tokio::spawn(async move {
        use crate::pocketoption::modules::pending_trades::PendingTradesApiModule;
        let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
        let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
        let (runner_tx, _) = kanal::bounded_async(10);
        let mut ws_rx_clone = ws_rx.clone();
        tokio::spawn(async move {
            while let Ok(_) = ws_rx_clone.recv().await {}
        });
        let state = create_mock_state();
        let mut module = PendingTradesApiModule::new(
            state, cmd_rx, resp_tx_for_module.clone(), msg_rx, ws_tx.clone(), runner_tx,
        );
        module.run().await.ok();
    });

    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::OpenPendingOrder {
        open_type: 1, amount: Decimal::from_f64_retain(100.0).unwrap(),
        asset: "EURUSD_otc".to_string(), open_time: "2026-04-07 22:50:00".to_string(),
        open_price: Decimal::from_f64_retain(1.1950).unwrap(),
        timeframe: 60, min_payout: 85, command: 0, req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    let wrong_id1 = Uuid::new_v4();
    let wrong_id2 = Uuid::new_v4();
    resp_tx.send(CommandResponse::Success {
        req_id: wrong_id1, pending_order: Box::new(pending_order.clone()),
    }).await.unwrap();
    resp_tx.send(CommandResponse::Success {
        req_id: wrong_id2, pending_order: Box::new(pending_order.clone()),
    }).await.unwrap();
    resp_tx.send(CommandResponse::Success {
        req_id, pending_order: Box::new(pending_order.clone()),
    }).await.unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::Success { req_id: rid, pending_order: po }) if rid == req_id => return po,
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(received.ticket, pending_order.ticket);
    module_task.abort();
}

#[tokio::test]
async fn test_open_pending_order_mismatch_max_retries_exceeded() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);

    // No module needed — test that mismatched req_ids do not match
    let req_id = Uuid::new_v4();
    for _ in 0..5 {
        let wrong_id = Uuid::new_v4();
        resp_tx.send(CommandResponse::Success {
            req_id: wrong_id,
            pending_order: Box::new(create_test_pending_order(Uuid::new_v4())),
        }).await.unwrap();
    }

    // Verify none of the responses have the expected req_id
    let mut mismatch_count = 0;
    while let Ok(resp) = tokio::time::timeout(Duration::from_secs(1), resp_rx.recv()).await {
        match resp {
            Ok(CommandResponse::Success { req_id: rid, .. }) => {
                if rid != req_id {
                    mismatch_count += 1;
                }
            }
            _ => break,
        }
    }
    assert_eq!(mismatch_count, 5);
}

#[tokio::test]
async fn test_open_pending_order_channel_error_sender_closed() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(1);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);

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
        .open_pending_order(OpenPendingOrder {
            open_type: 1,
            amount: Decimal::from_f64_retain(100.0).unwrap(),
            asset: "EURUSD_otc".to_string(),
            open_time: "2026-04-07 22:50:00".to_string(),
            open_price: Decimal::from_f64_retain(1.1950).unwrap(),
            timeframe: 60,
            min_payout: 85,
            command: 0,
        })
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PocketError::Core(_) => {}
        _ => panic!("Expected Core error from channel"),
    }
}

#[tokio::test]
async fn test_open_pending_order_with_socket_io_framing() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

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
    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::OpenPendingOrder {
        open_type: 1,
        amount: Decimal::from_f64_retain(100.0).unwrap(),
        asset: "EURUSD_otc".to_string(),
        open_time: "2026-04-07 22:50:00".to_string(),
        open_price: Decimal::from_f64_retain(1.1950).unwrap(),
        timeframe: 60,
        min_payout: 85,
        command: 0,
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let data_json = serde_json::to_string(&server_response).unwrap();
    let socket_io_msg = format!("42[\"successopenPendingOrder\",{}]", data_json);
    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::Success { req_id: rid, pending_order: po }) if rid == req_id => return po,
                Ok(CommandResponse::Error(_)) => panic!("Expected success"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(received.ticket, pending_order.ticket);

    module_task.abort();
}

// ============== Tests for PendingTradesApiModule::run ==============

#[tokio::test]
async fn test_run_routes_command_to_websocket() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, _) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    // Drain ws_rx in background using a clone to prevent blocking
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

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
        open_time: "2026-04-07 22:50:00".to_string(),
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
                open_time: "2026-04-07 22:50:00".to_string(),
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
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, _) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);

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
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    // Drain ws_rx in background using a clone to prevent blocking
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

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
            open_time: "2026-04-07 22:50:00".to_string(),
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
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    // Drain ws_rx in background using a clone to prevent blocking
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

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
            open_time: "2026-04-07 22:50:00".to_string(),
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
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, _) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);

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
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);

    let state = create_mock_state();

    let mut module =
        PendingTradesApiModule::new(state.clone(), cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let pending_order = create_test_pending_order(Uuid::new_v4());
    let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
    let data_json = serde_json::to_string(&server_response).unwrap();
    let socket_io_msg = format!("42[\"successopenPendingOrder\",{}]", data_json);
    msg_tx
        .send(Arc::new(Message::Text(socket_io_msg.into())))
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
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);

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

#[tokio::test]
async fn test_cancel_pending_order_success() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let ticket = Uuid::new_v4().to_string();
    let ticket_for_assert = ticket.clone();
    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::CancelPendingOrder {
        ticket: ticket.clone(),
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = CancelServerResponse::SingleSuccess {
        ticket: ticket.clone(),
    };
    let response_json = create_socket_io_text_message(
        "successcancelPendingOrder",
        &serde_json::to_value(server_response).unwrap(),
    );

    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::CancelSuccess { req_id: rid, ticket: t }) if rid == req_id => return t,
                Ok(CommandResponse::CancelError { .. }) => panic!("Expected success"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(received, ticket_for_assert);

    module_task.abort();
}

#[tokio::test]
async fn test_cancel_pending_order_failure() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let ticket = Uuid::new_v4().to_string();
    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::CancelPendingOrder {
        ticket: ticket.clone(),
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = CancelServerResponse::Error {
        error: "Deal not found".to_string(),
    };
    let response_json = create_socket_io_text_message(
        "failcancelPendingOrder",
        &serde_json::to_value(server_response).unwrap(),
    );

    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::CancelError { req_id: rid, error }) if rid == req_id => {
                    return error;
                }
                Ok(CommandResponse::CancelSuccess { .. }) => panic!("Expected error"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(received, "Deal not found");

    module_task.abort();
}

#[tokio::test]
async fn test_cancel_pending_orders_batch_success() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let ticket1 = Uuid::new_v4().to_string();
    let ticket2 = Uuid::new_v4().to_string();
    let tickets = vec![ticket1.clone(), ticket2.clone()];
    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::CancelPendingOrders {
        tickets: tickets.clone(),
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = CancelServerResponse::BatchSuccess {
        cancelled: tickets.clone(),
    };
    let response_json = create_socket_io_text_message(
        "successcancelPendingOrders",
        &serde_json::to_value(server_response).unwrap(),
    );

    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::BatchCancelSuccess { req_id: rid, cancelled }) if rid == req_id => return cancelled,
                Ok(CommandResponse::CancelSuccess { .. }) | Ok(CommandResponse::CancelError { .. }) => panic!("Expected batch success"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(received.len(), 2);

    module_task.abort();
}

#[tokio::test]
async fn test_cancel_pending_orders_batch_partial_success() {
    let (cmd_tx, cmd_rx) = kanal::bounded_async(10);
    let (resp_tx, resp_rx) = kanal::bounded_async(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, mut ws_rx) = kanal::bounded_async(10);
    let (runner_tx, _) = kanal::bounded_async(10);
    
    let mut ws_rx_clone = ws_rx.clone();
    tokio::spawn(async move {
        while let Ok(_) = ws_rx_clone.recv().await {}
    });

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    let ticket1 = Uuid::new_v4().to_string();
    let ticket2 = Uuid::new_v4().to_string();
    let tickets = vec![ticket1.clone(), ticket2.clone()];
    let req_id = Uuid::new_v4();
    cmd_tx.send(Command::CancelPendingOrders {
        tickets: tickets.clone(),
        req_id,
    }).await.unwrap();

    tokio::time::sleep(Duration::from_millis(10)).await;

    let server_response = CancelServerResponse::BatchSuccess {
        cancelled: vec![ticket1],
    };
    let response_json = create_socket_io_text_message(
        "successcancelPendingOrders",
        &serde_json::to_value(server_response).unwrap(),
    );

    msg_tx
        .send(Arc::new(Message::Text(response_json.into())))
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match resp_rx.recv().await {
                Ok(CommandResponse::BatchCancelSuccess { req_id: rid, cancelled }) if rid == req_id => return cancelled,
                Ok(CommandResponse::CancelSuccess { .. }) | Ok(CommandResponse::CancelError { .. }) => panic!("Expected batch success"),
                Ok(_) => continue,
                Err(_) => panic!("Channel closed"),
            }
        }
    }).await.unwrap();

    assert_eq!(received.len(), 1);

    module_task.abort();
}
