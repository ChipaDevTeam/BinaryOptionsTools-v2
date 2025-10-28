# Binary Options Tools (Rust)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ChipaDevTeam/BinaryOptionsTools-v2)
[![Crates.io](https://img.shields.io/crates/v/binary_options_tools.svg)](https://crates.io/crates/binary_options_tools)
[![Docs.rs](https://docs.rs/binary_options_tools/badge.svg)](https://docs.rs/binary_options_tools)
<!-- Add other badges as appropriate, e.g., license, build status -->

A Rust crate providing tools to interact programmatically with various binary options trading platforms.

## Overview

This crate aims to provide a unified and robust interface for developers looking to connect to and automate interactions with binary options trading platforms using Rust. Whether you're building trading bots, analysis tools, or integrating trading capabilities into larger applications, `binary_options_tools` strives to offer the necessary building blocks.

The core library is written in Rust for performance and safety, and it serves as the foundation for potential bindings or wrappers in other programming languages.

## Currently Supported Features

### PocketOption Platform
- **Authentication**: Secure connection using session IDs (SSID)
- **Account Management**: 
  - Get current account balance
  - Check if account is demo or real
  - Server time synchronization
- **Trading Operations**:
  - Place buy/sell trades on any supported asset
  - Trade validation (amount limits, asset availability, time validation)
  - Get trade results with optional timeout
  - Get list of currently opened trades
- **Asset Management**:
  - Get asset information including payouts and available trade times
  - Asset validation for trading
- **Real-time Data**:
  - Subscribe to asset price feeds with different subscription types
  - Time-aligned subscriptions
  - Chunked data subscriptions
- **Connection Management**:
  - Automatic reconnection handling
  - Connection status monitoring
  - Manual reconnection support

## TODO Features
- Historical candle data retrieval
- Closed deals management and history
- Pending trades support
- Additional trading platforms (Expert Options, etc.)

## Installation

Add the crate to your `Cargo.toml` dependencies:

```toml
[dependencies]
binary_options_tools = "0.1.7" 
```

## Quick Start

```rust
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client with session ID
    let client = PocketOption::new("your_session_id").await?;
    
    // IMPORTANT: Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get account balance
    let balance = client.balance().await;
    println!("Current balance: ${}", balance);
    
    // Place a buy trade on EURUSD for 60 seconds with $1
    let (trade_id, deal) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("Trade placed with ID: {}", trade_id);
    println!("Deal data: {:?}", deal);
    
    // Wait for trade to complete
    tokio::time::sleep(Duration::from_secs(65)).await;
    
    // Check trade result
    let result = client.result(trade_id).await?;
    println!("Trade result: {:?}", result);
    
    Ok(())
}
```

## Detailed Examples

### Basic Trading Operations

```rust
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your_session_id").await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get account balance
    let balance = client.balance().await;
    println!("Current Balance: ${}", balance);
    
    // Place a buy trade
    let (buy_id, buy_deal) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("Buy Trade ID: {}", buy_id);
    
    // Place a sell trade
    let (sell_id, sell_deal) = client.sell("EURUSD_otc", 60, 1.0).await?;
    println!("Sell Trade ID: {}", sell_id);
    
    // Wait for trades to complete
    tokio::time::sleep(Duration::from_secs(65)).await;
    
    // Check results
    let buy_result = client.result(buy_id).await?;
    let sell_result = client.result(sell_id).await?;
    
    println!("Buy result: {:?}", buy_result);
    println!("Sell result: {:?}", sell_result);
    
    Ok(())
}
```

### Real-Time Data Subscription

```rust
use binary_options_tools::{PocketOption, SubscriptionType};
use std::time::Duration;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your_session_id").await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Subscribe to real-time candle data
    let mut subscription = client.subscribe("EURUSD_otc", SubscriptionType::None).await?;
    
    println!("Listening for real-time candles...");
    while let Some(candle) = subscription.next().await {
        match candle {
            Ok(candle_data) => {
                println!("New Candle:");
                println!("  Time: {}", candle_data.time);
                println!("  Open: {}", candle_data.open);
                println!("  High: {}", candle_data.high);
                println!("  Low: {}", candle_data.low);
                println!("  Close: {}", candle_data.close);
                println!("---");
            }
            Err(e) => eprintln!("Error receiving candle: {:?}", e),
        }
    }
    
    Ok(())
}
```

### Checking Opened Deals

```rust
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your_session_id").await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get all opened deals
    let opened_deals = client.opened().await?;
    
    if opened_deals.is_empty() {
        println!("No opened deals");
    } else {
        println!("You have {} opened deals:", opened_deals.len());
        for deal in opened_deals {
            println!("  - Trade ID: {}", deal.id);
            println!("    Asset: {}", deal.asset);
            println!("    Amount: ${}", deal.amount);
            println!("    Direction: {:?}", deal.action);
        }
    }
    
    Ok(())
}
```

### Advanced: Multiple Concurrent Operations

```rust
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your_session_id").await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Execute multiple operations concurrently
    let (balance, opened_deals, server_time) = tokio::try_join!(
        async { Ok::<_, Box<dyn std::error::Error>>(client.balance().await) },
        client.opened(),
        async { Ok::<_, Box<dyn std::error::Error>>(client.server_time().await) },
    )?;
    
    println!("Balance: ${}", balance);
    println!("Opened Deals: {}", opened_deals.len());
    println!("Server Time: {}", server_time);
    
    Ok(())
}
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish properly:

```rust
let client = PocketOption::new("your_session_id").await?;
tokio::time::sleep(Duration::from_secs(5)).await;  // Critical!
```

### Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value

### Supported Assets

Common assets include:
- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## 📚 Additional Resources

- **Full Documentation**: [https://docs.rs/binary_options_tools](https://docs.rs/binary_options_tools)
- **Examples**: [examples directory](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/tree/master/examples)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.