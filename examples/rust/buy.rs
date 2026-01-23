// Example showing how to place a buy trade
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your-session-id").await?;
    
    // IMPORTANT: Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get initial balance
    let balance_before = client.balance().await;
    println!("Balance before trade: ${:.2}", balance_before);
    
    // Place a buy trade on EURUSD for 60 seconds with $1
    let (trade_id, deal) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("\nTrade placed successfully!");
    println!("Trade ID: {}", trade_id);
    println!("Deal data: {:?}", deal);
    
    // Wait for trade to complete
    println!("\nWaiting for trade to complete (65 seconds)...");
    tokio::time::sleep(Duration::from_secs(65)).await;
    
    // Get final balance
    let balance_after = client.balance().await;
    println!("Balance after trade: ${:.2}", balance_after);
    println!("Profit/Loss: ${:.2}", balance_after - balance_before);
    
    Ok(())
}
