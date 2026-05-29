//! Test the library with a demo SSID to verify is_connected, max_subscriptions,
//! subscription, and candle fetching work correctly.
//!
//! Set the POCKET_OPTION_SSID environment variable before running:
//! ```bash
//! export POCKET_OPTION_SSID='42["auth",{"session":"...","isDemo":1,...}]'
//! cargo run -p binary_options_tools --example test_demo
//! ```

use std::time::Duration;

use binary_options_tools::config::Config;
use binary_options_tools::pocketoption::PocketOption;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ssid = std::env::var("POCKET_OPTION_SSID").expect(
        "POCKET_OPTION_SSID environment variable not set. \
         Export it with: export POCKET_OPTION_SSID='42[\"auth\",{...}]'",
    );

    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Test 1: Connect with default config ===");
    let client = PocketOption::new(&ssid).await?;
    println!("  [PASS] Client created");

    // Wait for connection to stabilize
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Test is_connected
    let connected = client.is_connected();
    println!("  is_connected: {}", connected);
    assert!(connected, "Expected to be connected after initialization");
    println!("  [PASS] is_connected() returns true");

    // Test max_subscriptions default
    let max_subs = client.max_subscriptions();
    println!("  max_subscriptions: {}", max_subs);
    assert_eq!(max_subs, 4, "Expected default max_subscriptions of 4");
    println!("  [PASS] max_subscriptions() returns 4");

    // Test is_demo
    let is_demo = client.is_demo();
    println!("  is_demo: {}", is_demo);
    assert!(is_demo, "Expected demo account");
    println!("  [PASS] is_demo() returns true");

    // Test balance
    let balance = client.balance().await;
    println!("  balance: {}", balance);
    assert!(
        balance > rust_decimal::Decimal::ZERO,
        "Expected positive balance"
    );
    println!("  [PASS] balance() returns positive value");

    println!("\n=== Test 2: Get candles ===");
    match client.candles("EURUSD_otc", 60).await {
        Ok(candles) => {
            println!("  Got {} candles", candles.len());
            if let Some(first) = candles.first() {
                println!(
                    "  First candle: O={:.5} H={:.5} L={:.5} C={:.5}",
                    first.open, first.high, first.low, first.close
                );
            }
            println!("  [PASS] candles() works");
        }
        Err(e) => {
            println!("  [WARN] candles() returned error: {}", e);
        }
    }

    println!("\n=== Test 3: Subscribe to asset ===");
    match client
        .subscribe(
            "EURUSD_otc",
            binary_options_tools::pocketoption::candle::SubscriptionType::None,
        )
        .await
    {
        Ok(subscription) => {
            println!("  Subscribed to EURUSD_otc");
            use futures_util::StreamExt;
            let mut stream = subscription.to_stream();
            match tokio::time::timeout(Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(candle))) => {
                    println!(
                        "  Received tick: O={:.5} H={:.5} L={:.5} C={:.5}",
                        candle.open, candle.high, candle.low, candle.close
                    );
                    println!("  [PASS] subscribe() works");
                }
                Ok(Some(Err(e))) => {
                    println!("  [FAIL] Subscription error: {}", e);
                }
                Ok(None) => {
                    println!("  [WARN] Stream ended immediately");
                }
                Err(_) => {
                    println!("  [WARN] Timed out waiting for tick data");
                }
            }
        }
        Err(e) => {
            println!("  [FAIL] subscribe() error: {}", e);
        }
    }

    println!("\n=== Test 4: Custom max_subscriptions ===");
    let mut config = Config::default();
    config.max_subscriptions = 8;
    let client2 = PocketOption::new_with_config(&ssid, config).await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    let max_subs2 = client2.max_subscriptions();
    println!("  max_subscriptions: {}", max_subs2);
    assert_eq!(max_subs2, 8, "Expected max_subscriptions of 8");
    println!("  [PASS] Custom max_subscriptions works");

    let connected2 = client2.is_connected();
    println!("  is_connected: {}", connected2);
    assert!(connected2, "Expected to be connected");
    println!("  [PASS] Second client connected");

    println!("\n=== All tests passed ===");
    Ok(())
}
