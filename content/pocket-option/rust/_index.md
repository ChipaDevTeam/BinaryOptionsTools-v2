+++
title = "Rust Example"
description = "Example of how to use the PocketOption client in Rust."
weight = 1
+++

# Rust Example

Here is a basic example of how to use the `PocketOption` client in a Rust application.

```rust
use binary_options_tools_pocketoption::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Replace with your actual session ID
    let ssid = "YOUR_SSID_HERE";

    // Create a new PocketOption client
    let client = PocketOption::new(ssid).await?;

    // Wait for the client to connect
    client.wait_connected().await;

    // Get the current balance
    let balance = client.balance().await;
    println!("Current balance: {}", balance);

    // Get the list of available assets
    if let Some(assets) = client.assets().await {
        println!("Available assets: {:?}", assets);
    }

    // Subscribe to price updates for an asset
    let mut subscription = client.subscribe("EURUSD_otc", Default::default()).await?;

    // Print the first 5 price updates
    for _ in 0..5 {
        if let Some(price) = subscription.to_stream().next().await {
            println!("Price update: {:?}", price);
        }
    }

    // Shutdown the client
    client.shutdown().await?;

    Ok(())
}
```
