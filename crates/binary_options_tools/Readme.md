# Binary Options Tools (Rust)

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

## Basic Usage

```rust
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client with session ID
    let client = PocketOption::new("your_session_id").await?;
    
    // Wait for connection to be established
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get account balance
    let balance = client.balance().await;
    println!("Current balance: {}", balance);
    
    // Place a buy trade
    let (trade_id, deal) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("Trade placed: {:?}", deal);
    
    // Check trade result
    let result = client.result(trade_id).await?;
    println!("Trade result: {:?}", result);
    
    // Subscribe to real-time data
    let subscription = client.subscribe("EURUSD_otc", SubscriptionType::None).await?;
    
    // Shutdown client
    client.shutdown().await?;
    
    Ok(())
}
```