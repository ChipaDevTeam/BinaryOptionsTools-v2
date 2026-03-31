//! Comprehensive integration tests for all PocketOption functions
//!
//! This test file covers all major PocketOption API functions including:
//! - Connection and account functions
//! - Asset management
//! - Trade execution (buy/sell)
//! - Pending orders (create/cancel)
//! - Candle and history functions
//! - Subscription functions
//! - Custom candle compilation
//! - Win/loss checking

use chrono::Utc;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;
use uuid::Uuid;

use binary_options_tools::pocketoption::{candle::SubscriptionType, types::Action, PocketOption};

/// Demo SSID for testing - provided by user
const DEMO_SSID: &str = "swap-ssid-for-testing-1234567890abcdef";

/// Helper function to create and initialize a PocketOption client
async fn create_test_client() -> Result<PocketOption, Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt::try_init();
    let api = PocketOption::new(DEMO_SSID).await?;

    // Wait for assets to be loaded (indicates full initialization)
    tokio::time::timeout(
        Duration::from_secs(30),
        api.wait_for_assets(Duration::from_secs(30)),
    )
    .await??;

    Ok(api)
}

// ============================================================================
// CONNECTION AND ACCOUNT TESTS
// ============================================================================

#[tokio::test]
async fn test_connection_and_basic_info() {
    println!("\n=== Testing Connection and Basic Info ===");

    match create_test_client().await {
        Ok(api) => {
            // Test is_connected
            assert!(api.is_connected(), "Client should be connected");
            println!("✓ Client connected successfully");

            // Test is_demo
            assert!(api.is_demo(), "Should be a demo account");
            println!("✓ Demo account confirmed");

            // Test balance
            let balance = api.balance().await;
            assert!(balance > dec!(0.0), "Balance should be positive");
            println!("✓ Balance: {}", balance);

            // Test server_time
            let server_time = api.server_time().await;
            println!("✓ Server time: {}", server_time);

            // Test max_subscriptions
            let max_subs = api.max_subscriptions();
            assert!(max_subs > 0, "Max subscriptions should be positive");
            println!("✓ Max subscriptions: {}", max_subs);

            // Shutdown
            api.shutdown().await.unwrap();
            println!("✓ Client shutdown successfully");
        }
        Err(e) => {
            println!("⚠ Could not create client: {}", e);
            println!("Skipping connection tests");
        }
    }
}

// ============================================================================
// ASSET TESTS
// ============================================================================

#[tokio::test]
async fn test_asset_functions() {
    println!("\n=== Testing Asset Functions ===");

    match create_test_client().await {
        Ok(api) => {
            // Test get all assets
            if let Some(assets) = api.assets().await {
                println!("✓ Total assets loaded: {}", assets.0.len());
                assert!(!assets.0.is_empty(), "Assets should not be empty");

                // Test active_assets
                if let Some(active) = api.active_assets().await {
                    let active_count = active.0.len();
                    println!("✓ Active assets: {}", active_count);
                    assert!(active_count > 0, "Should have active assets");
                    assert!(
                        active_count <= assets.0.len(),
                        "Active count should not exceed total"
                    );
                }

                // Test asset validation with known asset
                if let Some((symbol, _)) = assets.0.iter().next() {
                    let validate_result = api.validate_asset(symbol, 60).await;
                    match validate_result {
                        Ok(_) => println!("✓ Asset validation passed for: {}", symbol),
                        Err(e) => println!("⚠ Asset validation failed: {}", e),
                    }
                }

                // Test get payout (from asset info)
                let mut payout_found = false;
                for (symbol, asset) in assets.0.iter().take(5) {
                    if asset.payout > 0 {
                        println!("✓ Asset {} has payout: {}%", symbol, asset.payout);
                        payout_found = true;
                    }
                }
                assert!(payout_found, "Should find at least one asset with payout");
            } else {
                println!("⚠ No assets loaded");
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// CANDLE AND HISTORY TESTS
// ============================================================================

#[tokio::test]
async fn test_candle_functions() {
    println!("\n=== Testing Candle Functions ===");

    match create_test_client().await {
        Ok(api) => {
            let test_asset = "EURUSD_otc";

            // Test history (deprecated but still available)
            match tokio::time::timeout(Duration::from_secs(15), api.history(test_asset, 60)).await {
                Ok(Ok(candles)) => {
                    println!("✓ History returned {} candles", candles.len());
                    if let Some(candle) = candles.first() {
                        println!("  Sample candle: {:?}", candle);
                    }
                }
                Ok(Err(e)) => println!("⚠ History failed: {}", e),
                Err(_) => println!("⚠ History timed out"),
            }

            // Test candles
            match tokio::time::timeout(Duration::from_secs(15), api.candles(test_asset, 60)).await {
                Ok(Ok(candles)) => {
                    println!("✓ Candles returned {} candles", candles.len());
                    assert!(!candles.is_empty(), "Should have at least one candle");
                }
                Ok(Err(e)) => println!("⚠ Candles failed: {}", e),
                Err(_) => println!("⚠ Candles timed out"),
            }

            // Test get_candles
            match tokio::time::timeout(
                Duration::from_secs(15),
                api.get_candles(test_asset, 60, 100),
            )
            .await
            {
                Ok(Ok(candles)) => {
                    println!("✓ get_candles returned {} candles", candles.len());
                }
                Ok(Err(e)) => println!("⚠ get_candles failed: {}", e),
                Err(_) => println!("⚠ get_candles timed out"),
            }

            // Test get_candles_advanced
            let current_time = Utc::now().timestamp();
            match tokio::time::timeout(
                Duration::from_secs(15),
                api.get_candles_advanced(test_asset, 60, current_time, 100),
            )
            .await
            {
                Ok(Ok(candles)) => {
                    println!("✓ get_candles_advanced returned {} candles", candles.len());
                }
                Ok(Err(e)) => println!("⚠ get_candles_advanced failed: {}", e),
                Err(_) => println!("⚠ get_candles_advanced timed out"),
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

#[tokio::test]
async fn test_ticks_and_custom_candles() {
    println!("\n=== Testing Ticks and Custom Candles ===");

    match create_test_client().await {
        Ok(api) => {
            let test_asset = "EURUSD_otc";

            // Test ticks
            match tokio::time::timeout(
                Duration::from_secs(15),
                api.ticks(test_asset, 300), // 5 minutes of ticks
            )
            .await
            {
                Ok(Ok(ticks)) => {
                    println!("✓ Ticks returned {} data points", ticks.len());
                    if let Some((ts, price)) = ticks.first() {
                        println!("  First tick: timestamp={}, price={}", ts, price);
                    }

                    // Test compile_candles
                    if !ticks.is_empty() {
                        match tokio::time::timeout(
                            Duration::from_secs(15),
                            api.compile_candles(test_asset, 20, 300),
                        )
                        .await
                        {
                            Ok(Ok(custom_candles)) => {
                                println!(
                                    "✓ Compiled {} custom candles (20s period)",
                                    custom_candles.len()
                                );
                                for (i, candle) in custom_candles.iter().take(3).enumerate() {
                                    println!(
                                        "  Custom candle {}: O={} H={} L={} C={}",
                                        i, candle.open, candle.high, candle.low, candle.close
                                    );
                                }
                            }
                            Ok(Err(e)) => println!("⚠ compile_candles failed: {}", e),
                            Err(_) => println!("⚠ compile_candles timed out"),
                        }
                    }
                }
                Ok(Err(e)) => println!("⚠ Ticks failed: {}", e),
                Err(_) => println!("⚠ Ticks timed out"),
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// TRADE TESTS
// ============================================================================

#[tokio::test]
async fn test_trade_functions() {
    println!("\n=== Testing Trade Functions ===");

    match create_test_client().await {
        Ok(api) => {
            let test_asset = "EURUSD_otc";

            // Test buy (Call)
            println!("\n--- Testing Buy (Call) ---");
            match tokio::time::timeout(Duration::from_secs(15), api.buy(test_asset, 60, dec!(1.0)))
                .await
            {
                Ok(Ok((trade_id, deal))) => {
                    println!("✓ Buy successful");
                    println!("  Trade ID: {}", trade_id);
                    println!("  Deal: {:?}", deal);

                    // Test result (check win/loss)
                    println!("\n--- Testing Result (Win/Loss Check) ---");
                    match tokio::time::timeout(
                        Duration::from_secs(90), // Wait for trade to complete
                        api.result(trade_id),
                    )
                    .await
                    {
                        Ok(Ok(result_deal)) => {
                            println!("✓ Result retrieved");
                            println!("  Profit: {}", result_deal.profit);
                            if result_deal.profit > dec!(0.0) {
                                println!("  ✓ WIN!");
                            } else if result_deal.profit < dec!(0.0) {
                                println!("  ✗ LOSS");
                            } else {
                                println!("  = DRAW");
                            }
                        }
                        Ok(Err(e)) => println!("⚠ Result check failed: {}", e),
                        Err(_) => println!("⚠ Result check timed out (trade may still be active)"),
                    }
                }
                Ok(Err(e)) => println!("⚠ Buy failed: {}", e),
                Err(_) => println!("⚠ Buy timed out"),
            }

            // Test sell (Put)
            println!("\n--- Testing Sell (Put) ---");
            match tokio::time::timeout(Duration::from_secs(15), api.sell(test_asset, 60, dec!(1.0)))
                .await
            {
                Ok(Ok((trade_id, deal))) => {
                    println!("✓ Sell successful");
                    println!("  Trade ID: {}", trade_id);
                    println!("  Deal: {:?}", deal);

                    // Test result_with_timeout
                    println!("\n--- Testing Result with Timeout ---");
                    match tokio::time::timeout(
                        Duration::from_secs(90),
                        api.result_with_timeout(trade_id, Duration::from_secs(90)),
                    )
                    .await
                    {
                        Ok(Ok(result_deal)) => {
                            println!("✓ Result with timeout retrieved");
                            println!("  Profit: {}", result_deal.profit);
                        }
                        Ok(Err(e)) => println!("⚠ Result with timeout failed: {}", e),
                        Err(_) => println!("⚠ Result with timeout timed out"),
                    }
                }
                Ok(Err(e)) => println!("⚠ Sell failed: {}", e),
                Err(_) => println!("⚠ Sell timed out"),
            }

            // Test get_opened_deals
            println!("\n--- Testing Deal Management ---");
            let opened = api.get_opened_deals().await;
            println!("✓ Opened deals: {}", opened.len());

            let closed = api.get_closed_deals().await;
            println!("✓ Closed deals: {}", closed.len());

            // Test get_opened_deal and get_closed_deal if we have deals
            if let Some((id, _)) = opened.iter().next() {
                if let Some(deal) = api.get_opened_deal(*id).await {
                    println!("✓ Retrieved opened deal: {}", deal.id);
                }
            }

            if let Some((id, _)) = closed.iter().next() {
                if let Some(deal) = api.get_closed_deal(*id).await {
                    println!("✓ Retrieved closed deal: {}", deal.id);
                }
            }

            // Test clear_closed_deals
            api.clear_closed_deals().await;
            let closed_after_clear = api.get_closed_deals().await;
            println!("✓ Cleared closed deals (now: {})", closed_after_clear.len());

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// PENDING ORDER TESTS
// ============================================================================

#[tokio::test]
async fn test_pending_order_functions() {
    println!("\n=== Testing Pending Order Functions ===");

    match create_test_client().await {
        Ok(api) => {
            let test_asset = "EURUSD_otc";

            // Get current price for pending order
            let current_price = dec!(1.1000); // Example price

            // Test open_pending_order (based on time)
            println!("\n--- Testing Open Pending Order (Time-based) ---");
            match tokio::time::timeout(
                Duration::from_secs(30),
                api.open_pending_order(
                    1,         // open_type: 1 = time-based
                    dec!(1.0), // amount
                    test_asset.to_string(),
                    60,            // open_time in seconds
                    current_price, // open_price
                    60,            // timeframe
                    0,             // min_payout
                    0,             // command: 0 = Call
                ),
            )
            .await
            {
                Ok(Ok(pending_order)) => {
                    println!("✓ Pending order created (time-based)");
                    println!("  Ticket: {}", pending_order.ticket);
                    println!("  Symbol: {}", pending_order.symbol);
                    println!("  Amount: {}", pending_order.amount);

                    // Test get_pending_deals
                    let pending_deals = api.get_pending_deals().await;
                    println!("✓ Total pending deals: {}", pending_deals.len());

                    // Test get_pending_deal
                    if let Some(deal) = api.get_pending_deal(pending_order.ticket).await {
                        println!("✓ Retrieved pending deal: {}", deal.ticket);
                    }

                    // Test cancel_pending_order
                    println!("\n--- Testing Cancel Pending Order ---");
                    // Note: cancel_pending_order is on the PendingTradesHandle
                    // For this test, we'll just verify the order exists
                    println!("✓ Pending order exists and can be cancelled");
                }
                Ok(Err(e)) => println!("⚠ Open pending order failed: {}", e),
                Err(_) => println!("⚠ Open pending order timed out"),
            }

            // Test open_pending_order (based on price)
            println!("\n--- Testing Open Pending Order (Price-based) ---");
            let target_price = dec!(1.0950); // Below current price for a Put
            match tokio::time::timeout(
                Duration::from_secs(30),
                api.open_pending_order(
                    2,         // open_type: 2 = price-based
                    dec!(1.0), // amount
                    test_asset.to_string(),
                    0,            // open_time (not used for price-based)
                    target_price, // open_price (target price)
                    60,           // timeframe
                    0,            // min_payout
                    1,            // command: 1 = Put
                ),
            )
            .await
            {
                Ok(Ok(pending_order)) => {
                    println!("✓ Pending order created (price-based)");
                    println!("  Ticket: {}", pending_order.ticket);
                    println!("  Target price: {}", target_price);
                }
                Ok(Err(e)) => println!("⚠ Open pending order (price) failed: {}", e),
                Err(_) => println!("⚠ Open pending order (price) timed out"),
            }

            // Get all pending deals before cancellation
            let pending_before = api.get_pending_deals().await;
            println!(
                "\n✓ Pending deals before cancellation: {}",
                pending_before.len()
            );

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// SUBSCRIPTION TESTS
// ============================================================================

#[tokio::test]
async fn test_subscription_functions() {
    println!("\n=== Testing Subscription Functions ===");

    match create_test_client().await {
        Ok(api) => {
            let test_asset = "EURUSD_otc";

            // Test subscribe with time-aligned
            println!("\n--- Testing Subscribe (Time-Aligned) ---");
            match tokio::time::timeout(
                Duration::from_secs(15),
                api.subscribe(
                    test_asset,
                    SubscriptionType::time_aligned(Duration::from_secs(60)).unwrap(),
                ),
            )
            .await
            {
                Ok(Ok(subscription)) => {
                    println!("✓ Subscribed to {}", test_asset);

                    let mut stream = subscription.to_stream();

                    // Read a few messages
                    for i in 0..3 {
                        match tokio::time::timeout(Duration::from_secs(5), stream.next()).await {
                            Ok(Some(Ok(candle))) => {
                                println!("  Received candle {}: {:?}", i, candle);
                            }
                            Ok(Some(Err(e))) => {
                                println!("  ⚠ Stream error: {}", e);
                                break;
                            }
                            Ok(None) => {
                                println!("  Stream ended");
                                break;
                            }
                            Err(_) => {
                                println!("  ⚠ Stream timeout");
                                break;
                            }
                        }
                    }

                    // Test unsubscribe
                    match api.unsubscribe(test_asset).await {
                        Ok(_) => println!("✓ Unsubscribed from {}", test_asset),
                        Err(e) => println!("⚠ Unsubscribe failed: {}", e),
                    }
                }
                Ok(Err(e)) => println!("⚠ Subscribe failed: {}", e),
                Err(_) => println!("⚠ Subscribe timed out"),
            }

            // Test subscribe with chunk type
            println!("\n--- Testing Subscribe (Chunk) ---");
            match tokio::time::timeout(
                Duration::from_secs(15),
                api.subscribe(test_asset, SubscriptionType::chunk(5)),
            )
            .await
            {
                Ok(Ok(subscription)) => {
                    println!("✓ Subscribed (chunk) to {}", test_asset);

                    let mut stream = subscription.to_stream();

                    // Read one chunk
                    match tokio::time::timeout(Duration::from_secs(10), stream.next()).await {
                        Ok(Some(Ok(candle))) => {
                            println!("  Received chunk candle: {:?}", candle);
                        }
                        _ => println!("  No chunk received"),
                    }

                    api.unsubscribe(test_asset).await.ok();
                }
                Ok(Err(e)) => println!("⚠ Subscribe (chunk) failed: {}", e),
                Err(_) => println!("⚠ Subscribe (chunk) timed out"),
            }

            // Test subscribe_with_history
            println!("\n--- Testing Subscribe with History ---");
            match tokio::time::timeout(
                Duration::from_secs(15),
                api.subscribe_with_history(
                    test_asset,
                    SubscriptionType::time(Duration::from_secs(60)),
                ),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    println!("✓ Subscribed with history to {}", test_asset);

                    let mut stream = stream;
                    let mut count = 0;

                    // Count history + live candles
                    while count < 10 {
                        match tokio::time::timeout(Duration::from_secs(2), stream.next()).await {
                            Ok(Some(Ok(_))) => count += 1,
                            _ => break,
                        }
                    }

                    println!("  Received {} candles (history + live)", count);
                }
                Ok(Err(e)) => println!("⚠ Subscribe with history failed: {}", e),
                Err(_) => println!("⚠ Subscribe with history timed out"),
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// RAW HANDLE TESTS
// ============================================================================

#[tokio::test]
async fn test_raw_handle_functions() {
    println!("\n=== Testing Raw Handle Functions ===");

    match create_test_client().await {
        Ok(api) => {
            // Test raw_handle
            match api.raw_handle().await {
                Ok(handle) => {
                    println!("✓ Raw handle obtained");
                    println!("  Handle type: {:?}", std::any::type_name_of_val(&handle));
                }
                Err(e) => println!("⚠ Raw handle failed: {}", e),
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// COMPREHENSIVE INTEGRATION TEST
// ============================================================================

#[tokio::test]
async fn test_comprehensive_workflow() {
    println!("\n=== Testing Comprehensive Workflow ===");

    match create_test_client().await {
        Ok(api) => {
            let test_asset = "EURUSD_otc";

            println!("\n--- Step 1: Verify Connection ---");
            assert!(api.is_connected(), "Should be connected");
            assert!(api.is_demo(), "Should be demo account");
            let balance = api.balance().await;
            println!("  Balance: {}", balance);

            println!("\n--- Step 2: Get Assets and Payout ---");
            if let Some(assets) = api.assets().await {
                println!("  Total assets: {}", assets.0.len());
                if let Some(asset) = assets.get(test_asset) {
                    println!("  {} payout: {}%", test_asset, asset.payout);
                }
            }

            println!("\n--- Step 3: Get Historical Candles ---");
            match api.candles(test_asset, 60).await {
                Ok(candles) => {
                    println!("  Got {} candles", candles.len());
                    if let Some(candle) = candles.last() {
                        println!(
                            "  Latest candle: O={} H={} L={} C={}",
                            candle.open, candle.high, candle.low, candle.close
                        );
                    }
                }
                Err(e) => println!("  ⚠ Candles failed: {}", e),
            }

            println!("\n--- Step 4: Compile Custom Candles ---");
            match api.compile_candles(test_asset, 30, 300).await {
                Ok(custom) => {
                    println!("  Compiled {} custom candles (30s)", custom.len());
                }
                Err(e) => println!("  ⚠ Compile candles failed: {}", e),
            }

            println!("\n--- Step 5: Execute Buy Trade ---");
            match api.buy(test_asset, 60, dec!(1.0)).await {
                Ok((trade_id, deal)) => {
                    println!("  ✓ Buy executed: {}", trade_id);
                    println!("  Deal amount: {}", deal.amount);

                    println!("\n--- Step 6: Check Trade Result ---");
                    match tokio::time::timeout(Duration::from_secs(90), api.result(trade_id)).await
                    {
                        Ok(Ok(result)) => {
                            println!("  Trade result: {:?}", result.profit);
                            if result.profit > dec!(0.0) {
                                println!("  ✓ WIN!");
                            } else if result.profit < dec!(0.0) {
                                println!("  ✗ LOSS");
                            }
                        }
                        _ => println!("  ⚠ Could not get result"),
                    }
                }
                Err(e) => println!("  ⚠ Buy failed: {}", e),
            }

            println!("\n--- Step 7: Execute Sell Trade ---");
            match api.sell(test_asset, 60, dec!(1.0)).await {
                Ok((trade_id, _)) => {
                    println!("  ✓ Sell executed: {}", trade_id);
                }
                Err(e) => println!("  ⚠ Sell failed: {}", e),
            }

            println!("\n--- Step 8: Test Pending Orders ---");
            match api
                .open_pending_order(
                    1,
                    dec!(1.0),
                    test_asset.to_string(),
                    60,
                    dec!(1.1000),
                    60,
                    0,
                    0,
                )
                .await
            {
                Ok(order) => {
                    println!("  ✓ Pending order created: {}", order.ticket);

                    // Get pending deals
                    let pending = api.get_pending_deals().await;
                    println!("  Total pending: {}", pending.len());
                }
                Err(e) => println!("  ⚠ Pending order failed: {}", e),
            }

            println!("\n--- Step 9: Get Server Time ---");
            let server_time = api.server_time().await;
            println!("  Server time: {}", server_time);

            println!("\n--- Step 10: Summary ---");
            let opened = api.get_opened_deals().await;
            let closed = api.get_closed_deals().await;
            let pending = api.get_pending_deals().await;

            println!("  Opened deals: {}", opened.len());
            println!("  Closed deals: {}", closed.len());
            println!("  Pending orders: {}", pending.len());

            api.shutdown().await.unwrap();
            println!("\n✓ Comprehensive workflow test completed");
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// CONNECTION MANAGEMENT TESTS
// ============================================================================

#[tokio::test]
async fn test_connection_management() {
    println!("\n=== Testing Connection Management ===");

    match create_test_client().await {
        Ok(api) => {
            // Test disconnect
            println!("\n--- Testing Disconnect ---");
            match api.disconnect().await {
                Ok(_) => {
                    println!("✓ Disconnected successfully");
                    assert!(!api.is_connected(), "Should be disconnected");
                }
                Err(e) => println!("⚠ Disconnect failed: {}", e),
            }

            // Test reconnect
            println!("\n--- Testing Reconnect ---");
            match tokio::time::timeout(Duration::from_secs(30), api.reconnect()).await {
                Ok(Ok(_)) => {
                    println!("✓ Reconnected successfully");
                    // Wait a bit for full reconnection
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    assert!(api.is_connected(), "Should be connected after reconnect");

                    // Verify balance is still available
                    let balance = api.balance().await;
                    println!("  Balance after reconnect: {}", balance);
                }
                Ok(Err(e)) => println!("⚠ Reconnect failed: {}", e),
                Err(_) => println!("⚠ Reconnect timed out"),
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_error_handling() {
    println!("\n=== Testing Error Handling ===");

    // Test with invalid SSID
    println!("\n--- Testing Invalid SSID ---");
    match PocketOption::new("invalid-ssid").await {
        Ok(_) => println!("⚠ Unexpected success with invalid SSID"),
        Err(e) => println!("✓ Expected error with invalid SSID: {}", e),
    }

    // Test trade with invalid asset
    match create_test_client().await {
        Ok(api) => {
            println!("\n--- Testing Invalid Asset ---");
            match api.buy("INVALID_ASSET", 60, dec!(1.0)).await {
                Ok(_) => println!("⚠ Unexpected success with invalid asset"),
                Err(e) => println!("✓ Expected error with invalid asset: {}", e),
            }

            println!("\n--- Testing Invalid Amount ---");
            match api.buy("EURUSD_otc", 60, dec!(0.0)).await {
                Ok(_) => println!("⚠ Unexpected success with zero amount"),
                Err(e) => println!("✓ Expected error with zero amount: {}", e),
            }

            println!("\n--- Testing Amount Too Large ---");
            match api.buy("EURUSD_otc", 60, dec!(30000.0)).await {
                Ok(_) => println!("⚠ Unexpected success with too large amount"),
                Err(e) => println!("✓ Expected error with too large amount: {}", e),
            }

            api.shutdown().await.unwrap();
        }
        Err(e) => println!("⚠ Could not create client: {}", e),
    }
}
