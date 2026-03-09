// Example showing how to get account balance
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your-session-id").await?;
    
    // IMPORTANT: Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get current balance
    let balance = client.balance().await;
    println!("Your current balance is: ${:.2}", balance);
    
    Ok(())
}
