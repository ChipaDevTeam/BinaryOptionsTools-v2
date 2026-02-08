// Basic example showing how to initialize the client and get balance
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client with your session ID
    let client = PocketOption::new("your-session-id").await?;

    // IMPORTANT: Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Get account balance
    let balance = client.balance().await;
    println!("Current Balance: ${}", balance);

    // Get server time
    let server_time = client.server_time().await;
    println!("Server Time: {}", server_time);

    // Check if account is demo
    let is_demo = client.is_demo().await;
    println!("Is Demo Account: {}", is_demo);

    Ok(())
}
