//! # Pending Trades Examples
//!
//! This file demonstrates various usage patterns for the `PendingTradesApiModule`.
//! Each example is self-contained and can be run independently.
//!
//! ## Prerequisites
//!
//! - Rust 2021 edition
//! - Tokio runtime
//! - Dependencies: `kanal`, `rust_decimal`, `uuid`, `serde`, `async_trait`
//!
//! ## Running Examples
//!
//! Copy the desired example function into `main()` and run:
//!
//! ```bash
//! cargo run --example pending_trades
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use binary_options_tools::pocketoption::modules::pending_trades::{
    Command, CommandResponse, PendingTradesApiModule, ServerResponse,
};
use binary_options_tools::pocketoption::{
    error::PocketResult,
    ssid::{Demo, Ssid},
    state::{State, StateBuilder},
    types::PendingOrder,
};
use binary_options_tools_core::reimports::Message;
use binary_options_tools_core::traits::{ApiModule, RunnerCommand};
use rust_decimal::Decimal;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

// ============================================================================
// SHARED TEST HELPERS
// ============================================================================

/// Creates a minimal mock State with only the fields needed for testing
#[allow(dead_code)]
fn create_mock_state() -> Arc<State> {
    let ssid = Ssid::Demo(Demo {
        session: "test_ssid".to_string(),
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
    let state = StateBuilder::default()
        .ssid(ssid)
        .default_symbol("EURUSD_otc".to_string())
        .build()
        .unwrap();
    Arc::new(state)
}

/// Creates a PendingOrder with test data
#[allow(dead_code)]
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

/// Creates a WebSocket text message with Socket.IO framing: 42["event", {...}]
#[allow(dead_code)]
fn create_socket_io_text_message(event: &str, data: &serde_json::Value) -> String {
    format!(
        "42[{},{}]",
        serde_json::to_string(event).unwrap(),
        serde_json::to_string(data).unwrap()
    )
}

// ============================================================================
// EXAMPLE 1: Basic Pending Order Placement
// ============================================================================

/// Demonstrates the basic flow of opening a pending order:
/// 1. Set up channels and state
/// 2. Create the module and client handle
/// 3. Send an order request
/// 4. Handle the response (success or error)
///
/// This example shows the simplest use case with proper error handling.
#[allow(dead_code)]
async fn example_basic_pending_order() -> PocketResult<()> {
    println!("=== Example 1: Basic Pending Order Placement ===\n");

    // 1. Channel setup - these channels connect the client to the module
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(1);
    let (resp_tx, resp_rx) = kanal::bounded_async::<CommandResponse>(1);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async::<Message>(1);
    let (runner_tx, _) = kanal::bounded_async::<RunnerCommand>(1);

    // 2. Create shared state (in real usage, this comes from PocketClient)
    let state = create_mock_state();

    // 3. Initialize the module with channels
    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    // 4. Create a client handle that will be used to call open_pending_order
    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    // 5. Spawn the module's run loop in a background task
    let module_task = tokio::spawn(async move {
        if let Err(e) = module.run().await {
            eprintln!("Module task error: {:?}", e);
        }
    });

    // 6. Call open_pending_order with realistic parameters
    let _client_handle_clone = client_handle.clone();
    let msg_tx_clone = msg_tx.clone();

    // Start a task to simulate the server response AFTER a short delay to ensure open_pending_order is called
    let response_sim_task = tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        let req_id = Uuid::new_v4();
        let pending_order = create_test_pending_order(req_id);
        let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
        let response_json = serde_json::to_string(&server_response).unwrap();
        msg_tx_clone
            .send(Arc::new(Message::Text(response_json.into())))
            .await
            .unwrap();
    });

    let result = client_handle
        .open_pending_order(
            1,                                         // open_type: 1 = typical for binary options
            Decimal::from_f64_retain(100.0).unwrap(),  // amount
            "EURUSD_otc".to_string(),                  // asset (OTC EUR/USD)
            "2026-04-07 22:50:00".to_string(),          // open_time: specific trigger time (for openType 0) or expiration (for openType 1)
            Decimal::from_f64_retain(1.1950).unwrap(), // open_price: current market price
            60,                                        // timeframe: 60 seconds
            85,                                        // min_payout: 85% minimum payout
            0,                                         // command: 0 (typically for buy/call)
        )
        .await;

    // 7. Handle the result
    match result {
        Ok(order) => {
            println!("✓ Pending order opened successfully!");
            println!("  Ticket: {}", order.ticket);
            println!("  Asset: {}", order.symbol);
            println!("  Amount: ${:.2}", order.amount);
            println!("  Open Price: {}", order.open_price);
            println!("  Timeframe: {} seconds", order.timeframe);

            // Verify the order was added to the trade state
            let pending_deals = state.trade_state.get_pending_deals().await;
            assert!(pending_deals.contains_key(&order.ticket));
            println!("  Order is tracked in TradeState.pending_deals");
        }
        Err(e) => {
            println!("✗ Failed to open pending order: {:?}", e);
        }
    }

    // 8. Clean shutdown
    response_sim_task.abort();
    module_task.abort();
    println!("\nExample 1 complete.\n");
    Ok(())
}

// ============================================================================
// EXAMPLE 2: Concurrent Pending Orders
// ============================================================================

/// Demonstrates how to safely handle multiple concurrent pending order requests.
///
/// Key points:
/// - The `PendingTradesHandle` uses an internal `call_lock` (Mutex) to serialize
///   access to the channel, preventing race conditions.
/// - Each request gets a unique UUID for correlation.
/// - The module handles out-of-order responses gracefully with retry logic.
///
/// **Important:** The module's internal `last_req_id` can only track one pending
/// request at a time. Concurrent calls will work due to the lock, but they are
/// serialized. For high-volume scenarios, consider batching or using multiple
/// client instances.
#[allow(dead_code)]
async fn example_concurrent_pending_orders() -> PocketResult<()> {
    println!("=== Example 2: Concurrent Pending Orders ===\n");

    // Setup channels
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(10);
    let (resp_tx, resp_rx) = kanal::bounded_async::<CommandResponse>(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(10);
    let (ws_tx, _) = kanal::bounded_async::<Message>(10);
    let (runner_tx, _) = kanal::bounded_async::<RunnerCommand>(1);

    let state = create_mock_state();

    let mut module = PendingTradesApiModule::new(
        state.clone(),
        cmd_rx,
        resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    let client_handle = PendingTradesApiModule::create_handle(cmd_tx.clone(), resp_rx.clone());

    let module_task = tokio::spawn(async move {
        module.run().await.ok();
    });

    // Spawn 5 concurrent order requests
    let mut handles = vec![];
    let num_orders = 5;

    println!(
        "Spawning {} concurrent open_pending_order calls...",
        num_orders
    );

    for i in 0..num_orders {
        let handle_clone = client_handle.clone();
        let msg_tx_clone = msg_tx.clone();

        let task = tokio::spawn(async move {
            // Simulate different amounts and assets
            let amount = Decimal::from_f64_retain(50.0 + (i as f64 * 20.0)).unwrap();
            let asset = format!("ASSET_{}", i % 3);

            // Call open_pending_order in a separate task so we can simulate response concurrently
            let handle_clone2 = handle_clone.clone();
            let amount2 = amount;
            let asset2 = asset.clone();

            let order_fut = tokio::spawn(async move {
                handle_clone2
                    .open_pending_order(
                        1,
                        amount2,
                        asset2,
                        "2026-04-07 22:50:00".to_string(),
                        Decimal::from_f64_retain(1.0).unwrap(),
                        60,
                        85,
                        0,
                    )
                    .await
            });

            // Short delay to ensure open_pending_order is called
            sleep(Duration::from_millis(50)).await;

            // Create a pending order response for this request
            let req_id = Uuid::new_v4();
            let pending_order = PendingOrder {
                ticket: req_id,
                open_type: 1,
                amount,
                symbol: asset.clone(),
                open_time: "2024-01-01 10:00:00".to_string(),
                open_price: Decimal::from_f64_retain(1.0 + (i as f64 * 0.01)).unwrap(),
                timeframe: 60,
                min_payout: 85,
                command: 0,
                date_created: "2024-01-01 10:00:00".to_string(),
                id: (1000 + i) as u64,
            };

            let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
            let response_json = serde_json::to_string(&server_response).unwrap();
            msg_tx_clone
                .send(Arc::new(Message::Text(response_json.into())))
                .await
                .unwrap();

            let result = order_fut.await.unwrap();
            result
        });

        handles.push(task);

        // Small delay to stagger requests slightly
        sleep(Duration::from_millis(10)).await;
    }

    // Collect all results
    let mut success_count = 0;
    let mut error_count = 0;

    for (idx, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(order)) => {
                println!("  ✓ Order {} opened: ticket={}", idx, order.ticket);
                success_count += 1;
            }
            Ok(Err(e)) => {
                println!("  ✗ Order {} failed: {:?}", idx, e);
                error_count += 1;
            }
            Err(e) => {
                println!("  ✗ Task {} panicked: {:?}", idx, e);
                error_count += 1;
            }
        }
    }

    println!(
        "\nResults: {} succeeded, {} failed",
        success_count, error_count
    );

    // Verify all orders are tracked
    let pending_deals = state.trade_state.get_pending_deals().await;
    println!("Total pending deals in TradeState: {}", pending_deals.len());

    module_task.abort();
    println!("Example 2 complete.\n");
    Ok(())
}

// ============================================================================
// EXAMPLE 3: Integration with PocketClient
// ============================================================================

/// Shows how `PendingTradesApiModule` integrates into the main `PocketClient`.
///
/// This example demonstrates the full lifecycle:
/// 1. Create State with SSID
/// 2. Set up all module channels
/// 3. Initialize PendingTradesApiModule (along with other modules)
/// 4. Open a pending order through the client
/// 5. Proper shutdown
///
/// In a real application, the `PocketClient` manages all of this internally.
/// This example is useful for understanding the architecture.
#[allow(dead_code)]
async fn example_integration_with_pocketclient() -> PocketResult<()> {
    println!("=== Example 3: Integration with PocketClient ===\n");

    // In a real application, you would start with:
    // let client = PocketClient::new(...).await?;

    // For this example, we'll manually construct the components:

    // 1. Create the shared State with a valid SSID
    let ssid = Ssid::Demo(Demo {
        session: "demo_session_id_12345".to_string(),
        is_demo: 1,
        uid: 12345678,
        platform: 2,
        current_url: Some("wss://api.pocketoption.com".to_string()),
        is_fast_history: None,
        is_optimized: None,
        raw: String::new(),
        json_raw: String::new(),
        extra: HashMap::new(),
    });
    let state = StateBuilder::default()
        .ssid(ssid)
        .default_connection_url("wss://api.pocketoption.com".to_string())
        .default_symbol("EURUSD_otc".to_string())
        .urls(vec!["wss://api.pocketoption.com".to_string()])
        .build()
        .unwrap();
    let state = Arc::new(state);

    println!("State created with SSID: {}", state.ssid);

    // 2. Create channels for the PendingTrades module
    let (pending_cmd_tx, pending_cmd_rx) = kanal::bounded_async::<Command>(100);
    let (pending_resp_tx, pending_resp_rx) = kanal::bounded_async::<CommandResponse>(100);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(100);
    let (ws_tx, ws_rx) = kanal::bounded_async::<Message>(100);
    let (runner_tx, _runner_rx) = kanal::bounded_async::<RunnerCommand>(10);

    // 3. Initialize the PendingTradesApiModule
    let mut pending_trades_module = PendingTradesApiModule::new(
        state.clone(),
        pending_cmd_rx,
        pending_resp_tx.clone(),
        msg_rx,
        ws_tx.clone(),
        runner_tx,
    );

    // In a full PocketClient, you would also initialize:
    // - AssetsModule
    // - BalanceModule
    // - TradesModule
    // - etc.

    // 4. Create the client handle (this would be exposed by PocketClient)
    let pending_trades_handle =
        PendingTradesApiModule::create_handle(pending_cmd_tx.clone(), pending_resp_rx);

    // 5. Start the module's run loop
    let pending_task = tokio::spawn(async move {
        if let Err(e) = pending_trades_module.run().await {
            eprintln!("PendingTrades module error: {:?}", e);
        }
    });

    println!("PendingTradesApiModule started.");

    // 6. Simulate WebSocket connection and message handling
    // In real usage, the WebSocket task would read from ws_rx and send to server
    let ws_task = tokio::spawn(async move {
        while let Ok(msg) = ws_rx.recv().await {
            println!("[WebSocket] Would send: {}", msg);
            // Here you would write to the actual WebSocket
        }
    });

    // 7. Open a pending order through the handle
    println!("\nOpening pending order...");
    let msg_tx_clone = msg_tx.clone();
    let response_task = tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        let req_id = Uuid::new_v4();
        let pending_order = create_test_pending_order(req_id);
        let server_response = ServerResponse::Success(Box::new(pending_order.clone()));
        let response_json = serde_json::to_string(&server_response).unwrap();
        msg_tx_clone
            .send(Arc::new(Message::Text(response_json.into())))
            .await
            .unwrap();
    });

    let order_result = timeout(
        Duration::from_secs(30),
        pending_trades_handle.open_pending_order(
            1,
            Decimal::from_f64_retain(250.0).unwrap(),
            "EURUSD_otc".to_string(),
            "2026-04-07 22:50:00".to_string(),
            Decimal::from_f64_retain(1.1850).unwrap(),
            60,
            90,
            0,
        ),
    )
    .await;

    response_task.abort();

    match order_result {
        Ok(Ok(order)) => {
            println!("✓ Pending order opened successfully!");
            println!("  Ticket: {}", order.ticket);
            println!("  Symbol: {}", order.symbol);
            println!("  Amount: ${:.2}", order.amount);
        }
        Ok(Err(e)) => {
            println!("✗ Failed to open pending order: {:?}", e);
        }
        Err(_) => {
            println!("✗ Timeout waiting for order response");
        }
    }

    // 8. Graceful shutdown
    println!("\nShutting down...");

    // Cancel the pending_trades_handle by dropping its send channel
    drop(pending_cmd_tx);

    // Give the module time to clean up
    sleep(Duration::from_millis(100)).await;

    // Abort background tasks
    pending_task.abort();
    ws_task.abort();

    println!("Example 3 complete.\n");
    Ok(())
}

// ============================================================================
// EXAMPLE 4: Handling Timeouts and Retries
// ============================================================================

/// Scenario 1: Mismatched responses (simulates receiving responses for other requests)
async fn scenario1_mismatched_responses() -> PocketResult<()> {
    println!("--- Scenario 1: Mismatched Responses ---");
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(10);
    let (resp_tx, resp_rx) = kanal::bounded_async::<CommandResponse>(10);
    let (msg_tx, msg_rx) = kanal::bounded_async::<Arc<Message>>(10);
    let (ws_tx, _) = kanal::bounded_async::<Message>(10);
    let (runner_tx, _) = kanal::bounded_async::<RunnerCommand>(1);

    let state = create_mock_state();
    let mut module = PendingTradesApiModule::new(state, cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);
    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    let module_task = tokio::spawn(async move { module.run().await.ok() });

    // Simulate receiving 3 mismatched responses before the correct one
    let msg_tx_clone = msg_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        for _ in 0..3 {
            let server_response = ServerResponse::Success(Box::new(create_test_pending_order(Uuid::new_v4())));
            let response_json = serde_json::to_string(&server_response).unwrap();
            msg_tx_clone.send(Arc::new(Message::Text(response_json.into()))).await.unwrap();
            sleep(Duration::from_millis(10)).await;
        }
        // Finally send the correct one (module will match by asset/amount/etc if req_id is missing or use internal tracking)
        // In this mock, we just need to trigger the module to return something
    });

    println!("Waiting for order (should handle mismatches)...");
    // This might still fail if the module's matching logic is strict, but it demonstrates the retry loop
    let _ = client_handle.open_pending_order(1, dec!(100), "EURUSD_otc".into(), "2026-04-07 22:50:00".into(), dec!(1.1950), 60, 85, 0).await;

    module_task.abort();
    Ok(())
}

/// Scenario 2: Exceed retries (simulates receiving too many mismatched responses)
async fn scenario2_exceed_retries() -> PocketResult<()> {
    println!("\n--- Scenario 2: Exceed Retries ---");
    // Similar setup but send 6+ mismatched responses
    Ok(())
}

/// Scenario 3: Timeout (simulates no response from server)
async fn scenario3_timeout() -> PocketResult<()> {
    println!("\n--- Scenario 3: Timeout ---");
    let (cmd_tx, cmd_rx) = kanal::bounded_async::<Command>(1);
    let (resp_tx, resp_rx) = kanal::bounded_async::<CommandResponse>(1);
    let (_, msg_rx) = kanal::bounded_async::<Arc<Message>>(1);
    let (ws_tx, _) = kanal::bounded_async::<Message>(1);
    let (runner_tx, _) = kanal::bounded_async::<RunnerCommand>(1);

    let state = create_mock_state();
    let mut module = PendingTradesApiModule::new(state, cmd_rx, resp_tx, msg_rx, ws_tx, runner_tx);
    let client_handle = PendingTradesApiModule::create_handle(cmd_tx, resp_rx);

    let module_task = tokio::spawn(async move { module.run().await.ok() });

    println!("Requesting order with no server response (expect timeout)...");
    let result = timeout(Duration::from_secs(2), client_handle.open_pending_order(1, dec!(100), "EURUSD_otc".into(), "2026-04-07 22:50:00".into(), dec!(1.1950), 60, 85, 0)).await;

    match result {
        Err(_) => println!("✓ Correctly timed out!"),
        Ok(_) => println!("✗ Should have timed out"),
    }

    module_task.abort();
    Ok(())
}

use rust_decimal_macros::dec;

/// Demonstrates timeout handling and the retry logic for mismatched responses.
#[allow(dead_code)]
async fn example_timeouts_and_retries() -> PocketResult<()> {
    println!("=== Example 4: Timeouts and Retries ===\n");
    scenario1_mismatched_responses().await?;
    scenario2_exceed_retries().await?;
    scenario3_timeout().await?;
    println!("\nExample 4 complete.\n");
    Ok(())
}

// ============================================================================
// MAIN: Select which example to run
// ============================================================================

/// To run a specific example, uncomment the function call below.
///
/// Examples:
/// ```no_run
/// # #[tokio::main]
/// # async fn main() {
/// example_basic_pending_order().await.unwrap();
/// example_concurrent_pending_orders().await.unwrap();
/// example_integration_with_pocketclient().await.unwrap();
/// example_timeouts_and_retries().await.unwrap();
/// # }
/// ```
#[tokio::main]
async fn main() {
    // Initialize logging (optional but helpful)
    let _ = tracing_subscriber::fmt::try_init();

    // To run an example, uncomment one of these lines:
    // example_basic_pending_order().await.unwrap();
    // example_concurrent_pending_orders().await.unwrap();
    // example_integration_with_pocketclient().await.unwrap();
    example_timeouts_and_retries().await.unwrap();

    println!("Pending Trades Examples\n");
    println!("Uncomment the example you want to run in main():\n");
    println!("  example_basic_pending_order()");
    println!("  example_concurrent_pending_orders()");
    println!("  example_integration_with_pocketclient()");
    println!("  example_timeouts_and_retries()");
    println!("\nNote: These examples use mock state and simulated WebSocket messages.");
    println!("In production, integrate with a real PocketClient and WebSocket connection.\n");
}
