---
sidebar_position: 1
---

# BinaryOptionsTools API Reference

Complete API reference for BinaryOptionsTools with examples in all supported languages.

## Installation

### Python

```bash
pip install binaryoptionstoolsv2
```

### JavaScript/TypeScript

```bash
npm install binaryoptionstoolsv2
```

### Rust

```toml
[dependencies]
binary_options_tools = "0.1"
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
```

### Kotlin

```gradle
dependencies {
    implementation 'com.chipadevteam:binaryoptionstools:0.1.0'
}
```

### Swift

Add to `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2", from: "0.1.0")
]
```

### Go

```bash
go get github.com/ChipaDevTeam/BinaryOptionsTools-v2/bindings/go
```

### Ruby

```bash
gem install binaryoptionstoolsv2
```

### C#

```bash
dotnet add package BinaryOptionsToolsV2
```

---

## Quick Start

### Initialize Client

#### Python (Async)

```python
import asyncio
from binaryoptionstoolsv2 import PocketOptionAsync

async def main():
    client = await PocketOptionAsync("your_ssid")
    await asyncio.sleep(2)  # Wait for API to initialize

    balance = await client.balance()
    print(f"Balance: ${balance}")

    await client.shutdown()

asyncio.run(main())
```

#### Python (Sync)

```python
from binaryoptionstoolsv2 import PocketOption

client = PocketOption("your_ssid")
import time
time.sleep(2)

balance = client.balance()
print(f"Balance: ${balance}")

client.shutdown()
```

#### JavaScript

```javascript
const { PocketOption } = require('binaryoptionstoolsv2');

async function main() {
    const client = new PocketOption("your_ssid");
    await new Promise(resolve => setTimeout(resolve, 2000));

    const balance = await client.balance();
    console.log(`Balance: $${balance}`);

    await client.shutdown();
}

main();
```

#### Rust

```rust
use binary_options_tools::pocketoption::PocketOption;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PocketOption::new("your_ssid").await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let balance = client.balance().await?;
    println!("Balance: ${}", balance);

    client.shutdown().await?;
    Ok(())
}
```

---

## Trading Operations

### Place a Call (Buy) Trade

#### Python

```python
# Place a $1 call trade on EURUSD_otc for 60 seconds
trade = await client.buy("EURUSD_otc", 60, 1.0)
print(f"Trade ID: {trade.id}")
print(f"Asset: {trade.asset}")
print(f"Amount: ${trade.amount}")
```

#### JavaScript

```javascript
// Place a $1 call trade on EURUSD_otc for 60 seconds
const trade = await client.buy("EURUSD_otc", 60, 1.0);
console.log(`Trade ID: ${trade.id}`);
console.log(`Asset: ${trade.asset}`);
console.log(`Amount: $${trade.amount}`);
```

#### Rust

```rust
// Place a $1 call trade on EURUSD_otc for 60 seconds
let trade = client.buy("EURUSD_otc", 60, 1.0).await?;
println!("Trade ID: {}", trade.id);
println!("Asset: {}", trade.asset);
println!("Amount: ${}", trade.amount);
```

### Place a Put (Sell) Trade

#### Python

```python
# Place a $1 put trade on EURUSD_otc for 60 seconds
trade = await client.sell("EURUSD_otc", 60, 1.0)
print(f"Trade ID: {trade.id}")
```

#### JavaScript

```javascript
// Place a $1 put trade on EURUSD_otc for 60 seconds
const trade = await client.sell("EURUSD_otc", 60, 1.0);
console.log(`Trade ID: ${trade.id}`);
```

#### Rust

```rust
// Place a $1 put trade on EURUSD_otc for 60 seconds
let trade = client.sell("EURUSD_otc", 60, 1.0).await?;
println!("Trade ID: {}", trade.id);
```

### Check Trade Result

#### Python

```python
# Check if a trade won or lost
result = await client.result(trade.id)
print(f"Result: {'WIN' if result.profit > 0 else 'LOSS'}")
print(f"Profit: ${result.profit}")
```

#### JavaScript

```javascript
// Check if a trade won or lost
const result = await client.result(trade.id);
console.log(`Result: ${result.profit > 0 ? 'WIN' : 'LOSS'}`);
console.log(`Profit: $${result.profit}`);
```

#### Rust

```rust
// Check if a trade won or lost
let result = client.result(trade.id).await?;
println!("Result: {}", if result.profit > 0 { "WIN" } else { "LOSS" });
println!("Profit: ${}", result.profit);
```

---

## Account Management

### Get Balance

#### Python

```python
balance = await client.balance()
print(f"Current balance: ${balance:.2f}")
```

#### JavaScript

```javascript
const balance = await client.balance();
console.log(`Current balance: $${balance.toFixed(2)}`);
```

#### Rust

```rust
let balance = client.balance().await?;
println!("Current balance: ${:.2}", balance);
```

### Check if Demo Account

#### Python

```python
is_demo = client.is_demo()
account_type = "Demo" if is_demo else "Real"
print(f"Account type: {account_type}")
```

#### JavaScript

```javascript
const isDemo = client.isDemo();
const accountType = isDemo ? "Demo" : "Real";
console.log(`Account type: ${accountType}`);
```

#### Rust

```rust
let is_demo = client.is_demo().await?;
let account_type = if is_demo { "Demo" } else { "Real" };
println!("Account type: {}", account_type);
```

### Get Open Deals

#### Python

```python
open_deals = await client.get_opened_deals()
print(f"Open trades: {len(open_deals)}")
for deal in open_deals:
    print(f"  {deal.asset}: ${deal.amount} ({deal.action})")
```

#### JavaScript

```javascript
const openDeals = await client.getOpenedDeals();
console.log(`Open trades: ${openDeals.length}`);
openDeals.forEach(deal => {
    console.log(`  ${deal.asset}: $${deal.amount} (${deal.action})`);
});
```

#### Rust

```rust
let open_deals = client.get_opened_deals().await?;
println!("Open trades: {}", open_deals.len());
for deal in open_deals {
    println!("  {}: ${} ({})", deal.asset, deal.amount, deal.action);
}
```

### Get Closed Deals

#### Python

```python
closed_deals = await client.get_closed_deals()
print(f"Closed trades: {len(closed_deals)}")
for deal in closed_deals:
    result = "WIN" if deal.profit > 0 else "LOSS"
    print(f"  {deal.asset}: {result} (${deal.profit:.2f})")
```

#### JavaScript

```javascript
const closedDeals = await client.getClosedDeals();
console.log(`Closed trades: ${closedDeals.length}`);
closedDeals.forEach(deal => {
    const result = deal.profit > 0 ? "WIN" : "LOSS";
    console.log(`  ${deal.asset}: ${result} ($${deal.profit.toFixed(2)})`);
});
```

#### Rust

```rust
let closed_deals = client.get_closed_deals().await?;
println!("Closed trades: {}", closed_deals.len());
for deal in closed_deals {
    let result = if deal.profit > 0 { "WIN" } else { "LOSS" };
    println!("  {}: {} (${})", deal.asset, result, deal.profit);
}
```

---

## Market Data

### Get Historical Candles

> **Note**: Historical candles are fetched and manually compiled locally on the client from 1-second raw ticks. Timestamps are grouped strictly according to UTC calendar boundaries (`timestamp / period * period`), avoiding server-side candle time-alignment mismatches, gaps, or overlaps ("merges"). This applies to both `.candles()` (default 1000 periods lookback) and `.compile_candles()` (custom lookback period).

#### Python

```python
# Get last 100 candles with 60-second period
candles = await client.get_candles("EURUSD_otc", 60, 100)
print(f"Retrieved {len(candles)} candles")
for candle in candles[:5]:  # Show first 5
    print(f"  Time: {candle.time}, Close: {candle.close}")
```

#### JavaScript

```javascript
// Get last 100 candles with 60-second period
const candles = await client.getCandles("EURUSD_otc", 60, 100);
console.log(`Retrieved ${candles.length} candles`);
candles.slice(0, 5).forEach(candle => {
    console.log(`  Time: ${candle.time}, Close: ${candle.close}`);
});
```

#### Rust

```rust
// Get last 100 candles with 60-second period
let candles = client.get_candles("EURUSD_otc", 60, 100).await?;
println!("Retrieved {} candles", candles.len());
for candle in candles.iter().take(5) {
    println!("  Time: {}, Close: {}", candle.time, candle.close);
}
```

### Get Server Time

#### Python

```python
server_time = await client.server_time()
print(f"Server timestamp: {server_time}")
```

#### JavaScript

```javascript
const serverTime = await client.serverTime();
console.log(`Server timestamp: ${serverTime}`);
```

#### Rust

```rust
let server_time = client.server_time().await?;
println!("Server timestamp: {}", server_time);
```

---

## Real-time Subscriptions

### Subscribe to Asset

#### Python

```python
# Subscribe to 60-second candles
subscription = await client.subscribe("EURUSD_otc", 60)
print("Subscribed to EURUSD_otc")

# Iterate over candles
async for candle in subscription:
    print(f"Candle: {candle}")
```

#### JavaScript

```javascript
// Subscribe to 60-second candles
const subscription = await client.subscribe("EURUSD_otc", 60);
console.log("Subscribed to EURUSD_otc");

// Receive candles (async iterator)
for await (const candle of subscription) {
    console.log(`Candle: ${JSON.stringify(candle)}`);
}
```

#### Rust

```rust
// Subscribe to 60-second candles
let subscription = client.subscribe("EURUSD_otc", 60).await?;
println!("Subscribed to EURUSD_otc");

// Receive candles
while let Some(candle) = subscription.next().await {
    println!("Candle: {:?}", candle);
}
```

### Unsubscribe from Asset

#### Python

```python
await client.unsubscribe("EURUSD_otc")
print("Unsubscribed from EURUSD_otc")
```

#### JavaScript

```javascript
await client.unsubscribe("EURUSD_otc");
console.log("Unsubscribed from EURUSD_otc");
```

#### Rust

```rust
client.unsubscribe("EURUSD_otc").await?;
println!("Unsubscribed from EURUSD_otc");
```

---

## Connection Management

### Reconnect

#### Python

```python
await client.reconnect()
await asyncio.sleep(2)  # Wait for reconnection
print("Reconnected to server")
```

#### JavaScript

```javascript
await client.reconnect();
await new Promise(resolve => setTimeout(resolve, 2000));
console.log("Reconnected to server");
```

#### Rust

```rust
client.reconnect().await?;
tokio::time::sleep(std::time::Duration::from_secs(2)).await;
println!("Reconnected to server");
```

### Shutdown

#### Python

```python
await client.shutdown()
print("Client shut down gracefully")
```

#### JavaScript

```javascript
await client.shutdown();
console.log("Client shut down gracefully");
```

#### Rust

```rust
client.shutdown().await?;
println!("Client shut down gracefully");
```

---

## Error Handling

### Python

```python
from binaryoptionstoolsv2 import PocketOptionAsync, PocketError

try:
    client = await PocketOptionAsync("invalid_ssid")
    balance = await client.balance()
except PocketError as e:
    print(f"Error: {e}")
except Exception as e:
    print(f"Unexpected error: {e}")
```

### JavaScript

```javascript
const { PocketOption, PocketError } = require('binaryoptionstoolsv2');

try {
    const client = new PocketOption("invalid_ssid");
    const balance = await client.balance();
} catch (e) {
    if (e instanceof PocketError) {
        console.log(`Error: ${e.message}`);
    } else {
        console.log(`Unexpected error: ${e.message}`);
    }
}
```

### Rust

```rust
use binary_options_tools::error::PocketError;

match PocketOption::new("invalid_ssid").await {
    Ok(client) => {
        match client.balance().await {
            Ok(balance) => println!("Balance: {}", balance),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Best Practices

### 1. Always Wait for Initialization

All languages should wait 2 seconds after creating the client:

- **Python**: `await asyncio.sleep(2)`
- **JavaScript**: `await new Promise(resolve => setTimeout(resolve, 2000))`
- **Rust**: `tokio::time::sleep(Duration::from_secs(2)).await`

### 2. Always Shutdown Gracefully

Call `shutdown()` when done to clean up resources.

### 3. Check Demo vs Real Account

Always verify account type before trading with real money:

```python
if not client.is_demo():
    print("WARNING: Using REAL account!")
```

### 4. Handle Errors Appropriately

Use try-catch blocks to handle connection errors and invalid operations.

### 5. Use Appropriate Timeouts

For time-sensitive operations, use `result_with_timeout()`:

```python
result = await client.result_with_timeout(trade.id, 120)  # 120 seconds
```

---

## Complete Examples

See the [examples directory](/examples) for complete working examples in each language:

- [Python Async Examples](/examples/python/async)
- [Python Sync Examples](/examples/python/sync)
- [JavaScript Examples](/examples/javascript)
- [Rust Examples](/examples/rust)
---

## API Method Reference

| Method | Description | Returns |
|--------|-------------|---------|
| `PocketOption(ssid)` / `PocketOptionAsync(ssid)` | Initialize client with session ID | Client instance |
| `new_with_url(ssid, url)` | Initialize with custom WebSocket URL | Client instance |
| `balance()` | Get current account balance | Float |
| `is_demo()` | Check if demo account | Boolean |
| `buy(asset, time, amount)` | Place call trade | Deal object |
| `sell(asset, time, amount)` | Place put trade | Deal object |
| `result(id)` | Check trade result | Deal object |
| `result_with_timeout(id, timeout)` | Check trade result with timeout | Deal object |
| `get_opened_deals()` | Get list of open trades | List of Deals |
| `get_closed_deals()` | Get list of closed trades | List of Deals |
| `clear_closed_deals()` | Clear closed trades from memory | Void |
| `get_candles(asset, period, offset)` | Get historical candles | List of Candles |
| `server_time()` | Get server timestamp | Integer (Unix timestamp) |
| `subscribe(asset, duration)` | Subscribe to real-time data | Subscription |
| `unsubscribe(asset)` | Unsubscribe from asset | Void |
| `reconnect()` | Reconnect to server | Void |
| `shutdown()` | Shutdown client | Void |

---

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report bugs](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- **Documentation**: [Full docs](https://chipadevteam.github.io/BinaryOptionsTools-v2/)

---

**Version**: 2.0.0  
**Last Updated**: June 2026  
**Platform Support**: PocketOption (Quick Trading)