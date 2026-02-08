// Example showing how to check trade results
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your-session-id").await?;

    // IMPORTANT: Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Place a buy trade
    let (trade_id, deal) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("Trade placed with ID: {}", trade_id);
    println!("Deal data: {:?}", deal);

    // Wait for trade to complete
    println!("\nWaiting for trade to complete (65 seconds)...");
    tokio::time::sleep(Duration::from_secs(65)).await;

    // Check the result
    let result = client.result(trade_id).await?;
    println!("\n=== Trade Result ===");
    println!("{:#?}", result);

    // You can also use result_with_timeout to wait for the result automatically
    println!("\n--- Placing another trade with automatic result checking ---");
    let (trade_id2, _) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("Trade placed with ID: {}", trade_id2);

    // This will wait for the trade to complete (with 70 second timeout)
    println!("Waiting for trade result...");
    let result2 = client.result_with_timeout(trade_id2, 70).await?;
    println!("\n=== Trade Result (with timeout) ===");
    println!("{:#?}", result2);

    Ok(())
}
