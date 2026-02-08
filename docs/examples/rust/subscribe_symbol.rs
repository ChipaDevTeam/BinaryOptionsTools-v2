// Example showing how to subscribe to real-time candle data
use binary_options_tools::{PocketOption, SubscriptionType};
use std::time::Duration;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your-session-id").await?;

    // IMPORTANT: Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Subscribe to real-time candle data for EURUSD
    let mut subscription = client
        .subscribe("EURUSD_otc", SubscriptionType::None)
        .await?;

    println!("Listening for real-time candles...");
    println!("Press Ctrl+C to stop\n");

    // Process incoming candles
    let mut count = 0;
    while let Some(candle_result) = subscription.next().await {
        match candle_result {
            Ok(candle) => {
                count += 1;
                println!("=== Candle #{} ===", count);
                println!("Time:  {}", candle.time);
                println!("Open:  {:.5}", candle.open);
                println!("High:  {:.5}", candle.high);
                println!("Low:   {:.5}", candle.low);
                println!("Close: {:.5}", candle.close);
                println!();

                // Stop after 10 candles for demo purposes
                if count >= 10 {
                    println!("Received 10 candles, stopping...");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error receiving candle: {:?}", e);
            }
        }
    }

    Ok(())
}
