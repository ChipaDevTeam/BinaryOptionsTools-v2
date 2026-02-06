# BinaryOptionsToolsUni API Reference

Complete API reference for BinaryOptionsToolsUni with examples in all supported languages.

> 📝 **Note**: This is a Markdown version. For the interactive version with language switchers, see [API_REFERENCE.html](API_REFERENCE.html)

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Trading Operations](#trading-operations)
- [Account Management](#account-management)
- [Market Data](#market-data)
- [Real-time Subscriptions](#real-time-subscriptions)
- [Connection Management](#connection-management)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)

---

## Installation

### Python

```bash
pip install binaryoptionstoolsuni
```

### Kotlin

```gradle
dependencies {
    implementation 'com.chipadevteam:binaryoptionstoolsuni:0.1.0'
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
gem install binaryoptionstoolsuni
```

### C #

```bash
dotnet add package BinaryOptionsToolsUni
```

---

## Quick Start

### Initialize Client

#### Python

```python
import asyncio
from binaryoptionstoolsuni import PocketOption

async def main():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)  # Wait for API to initialize

    balance = await client.balance()
    print(f"Balance: ${balance}")

    await client.shutdown()

asyncio.run(main())
```

#### Kotlin

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*
import kotlinx.coroutines.*

suspend fun main() = coroutineScope {
    val client = PocketOption.init("your_ssid")
    delay(2000) // Wait for API to initialize

    val balance = client.balance()
    println("Balance: $$balance")

    client.shutdown()
}
```

#### Swift

```swift
import BinaryOptionsToolsUni

Task {
    let client = try await PocketOption.init(ssid: "your_ssid")
    try await Task.sleep(nanoseconds: 2_000_000_000)

    let balance = await client.balance()
    print("Balance: $\(balance)")

    try await client.shutdown()
}
```

#### Go

```go
package main

import (
    "fmt"
    "time"
    bot "binaryoptionstoolsuni"
)

func main() {
    client, _ := bot.PocketOptionInit("your_ssid")
    time.Sleep(2 * time.Second)

    balance := client.Balance()
    fmt.Printf("Balance: $%.2f\n", balance)

    client.Shutdown()
}
```

#### Ruby

```ruby
require 'binaryoptionstoolsuni'
require 'async'

Async do
  client = BinaryOptionsToolsUni::PocketOption.init('your_ssid')
  sleep 2

  balance = client.balance
  puts "Balance: $#{balance}"

  client.shutdown
end
```

#### C #

```csharp
using BinaryOptionsToolsUni;

var client = await PocketOption.InitAsync("your_ssid");
await Task.Delay(2000);

var balance = await client.BalanceAsync();
Console.WriteLine($"Balance: ${balance}");

await client.ShutdownAsync();
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

#### Kotlin

```kotlin
// Place a $1 call trade on EURUSD_otc for 60 seconds
val trade = client.buy("EURUSD_otc", 60u, 1.0)
println("Trade ID: ${trade.id}")
println("Asset: ${trade.asset}")
println("Amount: $${trade.amount}")
```

#### Swift

```swift
// Place a $1 call trade on EURUSD_otc for 60 seconds
let trade = try await client.buy(asset: "EURUSD_otc", time: 60, amount: 1.0)
print("Trade ID: \(trade.id)")
print("Asset: \(trade.asset)")
print("Amount: $\(trade.amount)")
```

#### Go

```go
// Place a $1 call trade on EURUSD_otc for 60 seconds
trade, _ := client.Buy("EURUSD_otc", 60, 1.0)
fmt.Printf("Trade ID: %s\n", trade.Id)
fmt.Printf("Asset: %s\n", trade.Asset)
fmt.Printf("Amount: $%.2f\n", trade.Amount)
```

#### Ruby

```ruby
# Place a $1 call trade on EURUSD_otc for 60 seconds
trade = client.buy('EURUSD_otc', 60, 1.0)
puts "Trade ID: #{trade.id}"
puts "Asset: #{trade.asset}"
puts "Amount: $#{trade.amount}"
```

#### C #

```csharp
// Place a $1 call trade on EURUSD_otc for 60 seconds
var trade = await client.BuyAsync("EURUSD_otc", 60, 1.0);
Console.WriteLine($"Trade ID: {trade.Id}");
Console.WriteLine($"Asset: {trade.Asset}");
Console.WriteLine($"Amount: ${trade.Amount}");
```

### Place a Put (Sell) Trade

#### Python

```python
# Place a $1 put trade on EURUSD_otc for 60 seconds
trade = await client.sell("EURUSD_otc", 60, 1.0)
print(f"Trade ID: {trade.id}")
```

#### Kotlin

```kotlin
// Place a $1 put trade on EURUSD_otc for 60 seconds
val trade = client.sell("EURUSD_otc", 60u, 1.0)
println("Trade ID: ${trade.id}")
```

#### Swift

```swift
// Place a $1 put trade on EURUSD_otc for 60 seconds
let trade = try await client.sell(asset: "EURUSD_otc", time: 60, amount: 1.0)
print("Trade ID: \(trade.id)")
```

#### Go

```go
// Place a $1 put trade on EURUSD_otc for 60 seconds
trade, _ := client.Sell("EURUSD_otc", 60, 1.0)
fmt.Printf("Trade ID: %s\n", trade.Id)
```

#### Ruby

```ruby
# Place a $1 put trade on EURUSD_otc for 60 seconds
trade = client.sell('EURUSD_otc', 60, 1.0)
puts "Trade ID: #{trade.id}"
```

#### C #

```csharp
// Place a $1 put trade on EURUSD_otc for 60 seconds
var trade = await client.SellAsync("EURUSD_otc", 60, 1.0);
Console.WriteLine($"Trade ID: {trade.Id}");
```

### Check Trade Result

#### Python

```python
# Check if a trade won or lost
result = await client.result(trade.id)
print(f"Result: {result.profit > 0 and 'WIN' or 'LOSS'}")
print(f"Profit: ${result.profit}")
```

#### Kotlin

```kotlin
// Check if a trade won or lost
val result = client.result(trade.id)
println("Result: ${if (result.profit > 0) "WIN" else "LOSS"}")
println("Profit: $${result.profit}")
```

#### Swift

```swift
// Check if a trade won or lost
let result = try await client.result(id: trade.id)
print("Result: \(result.profit > 0 ? "WIN" : "LOSS")")
print("Profit: $\(result.profit)")
```

#### Go

```go
// Check if a trade won or lost
result, _ := client.Result(trade.Id)
status := "LOSS"
if result.Profit > 0 {
    status = "WIN"
}
fmt.Printf("Result: %s\n", status)
fmt.Printf("Profit: $%.2f\n", result.Profit)
```

#### Ruby

```ruby
# Check if a trade won or lost
result = client.result(trade.id)
puts "Result: #{result.profit > 0 ? 'WIN' : 'LOSS'}"
puts "Profit: $#{result.profit}"
```

#### C #

```csharp
// Check if a trade won or lost
var result = await client.ResultAsync(trade.Id);
Console.WriteLine($"Result: {(result.Profit > 0 ? "WIN" : "LOSS")}");
Console.WriteLine($"Profit: ${result.Profit}");
```

---

## Account Management

### Get Balance

#### Python

```python
balance = await client.balance()
print(f"Current balance: ${balance:.2f}")
```

#### Kotlin

```kotlin
val balance = client.balance()
println("Current balance: $${"%.2f".format(balance)}")
```

#### Swift

```swift
let balance = await client.balance()
print("Current balance: $\(String(format: "%.2f", balance))")
```

#### Go

```go
balance := client.Balance()
fmt.Printf("Current balance: $%.2f\n", balance)
```

#### Ruby

```ruby
balance = client.balance
puts "Current balance: $#{'%.2f' % balance}"
```

#### C #

```csharp
var balance = await client.BalanceAsync();
Console.WriteLine($"Current balance: ${balance:F2}");
```

### Check if Demo Account

#### Python

```python
is_demo = client.is_demo()
account_type = "Demo" if is_demo else "Real"
print(f"Account type: {account_type}")
```

#### Kotlin

```kotlin
val isDemo = client.isDemo()
val accountType = if (isDemo) "Demo" else "Real"
println("Account type: $accountType")
```

#### Swift

```swift
let isDemo = client.isDemo()
let accountType = isDemo ? "Demo" : "Real"
print("Account type: \(accountType)")
```

#### Go

```go
isDemo := client.IsDemo()
accountType := "Real"
if isDemo {
    accountType = "Demo"
}
fmt.Printf("Account type: %s\n", accountType)
```

#### Ruby

```ruby
is_demo = client.is_demo?
account_type = is_demo ? "Demo" : "Real"
puts "Account type: #{account_type}"
```

#### C #

```csharp
var isDemo = client.IsDemo();
var accountType = isDemo ? "Demo" : "Real";
Console.WriteLine($"Account type: {accountType}");
```

### Get Open Deals

#### Python

```python
open_deals = await client.get_opened_deals()
print(f"Open trades: {len(open_deals)}")
for deal in open_deals:
    print(f"  {deal.asset}: ${deal.amount} ({deal.action})")
```

#### Kotlin

```kotlin
val openDeals = client.getOpenedDeals()
println("Open trades: ${openDeals.size}")
openDeals.forEach { deal ->
    println("  ${deal.asset}: $${deal.amount} (${deal.action})")
}
```

#### Swift

```swift
let openDeals = await client.getOpenedDeals()
print("Open trades: \(openDeals.count)")
for deal in openDeals {
    print("  \(deal.asset): $\(deal.amount) (\(deal.action))")
}
```

#### Go

```go
openDeals := client.GetOpenedDeals()
fmt.Printf("Open trades: %d\n", len(openDeals))
for _, deal := range openDeals {
    fmt.Printf("  %s: $%.2f (%s)\n", deal.Asset, deal.Amount, deal.Action)
}
```

#### Ruby

```ruby
open_deals = client.get_opened_deals
puts "Open trades: #{open_deals.length}"
open_deals.each do |deal|
  puts "  #{deal.asset}: $#{deal.amount} (#{deal.action})"
end
```

#### C #

```csharp
var openDeals = await client.GetOpenedDealsAsync();
Console.WriteLine($"Open trades: {openDeals.Count}");
foreach (var deal in openDeals)
{
    Console.WriteLine($"  {deal.Asset}: ${deal.Amount} ({deal.Action})");
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

#### Kotlin

```kotlin
val closedDeals = client.getClosedDeals()
println("Closed trades: ${closedDeals.size}")
closedDeals.forEach { deal ->
    val result = if (deal.profit > 0) "WIN" else "LOSS"
    println("  ${deal.asset}: $result ($${deal.profit})")
}
```

#### Swift

```swift
let closedDeals = await client.getClosedDeals()
print("Closed trades: \(closedDeals.count)")
for deal in closedDeals {
    let result = deal.profit > 0 ? "WIN" : "LOSS"
    print("  \(deal.asset): \(result) ($\(deal.profit))")
}
```

#### Go

```go
closedDeals := client.GetClosedDeals()
fmt.Printf("Closed trades: %d\n", len(closedDeals))
for _, deal := range closedDeals {
    result := "LOSS"
    if deal.Profit > 0 {
        result = "WIN"
    }
    fmt.Printf("  %s: %s ($%.2f)\n", deal.Asset, result, deal.Profit)
}
```

#### Ruby

```ruby
closed_deals = client.get_closed_deals
puts "Closed trades: #{closed_deals.length}"
closed_deals.each do |deal|
  result = deal.profit > 0 ? "WIN" : "LOSS"
  puts "  #{deal.asset}: #{result} ($#{deal.profit})"
end
```

#### C #

```csharp
var closedDeals = await client.GetClosedDealsAsync();
Console.WriteLine($"Closed trades: {closedDeals.Count}");
foreach (var deal in closedDeals)
{
    var result = deal.Profit > 0 ? "WIN" : "LOSS";
    Console.WriteLine($"  {deal.Asset}: {result} (${deal.Profit:F2})");
}
```

---

## Market Data

### Get Historical Candles

#### Python

```python
# Get last 100 candles with 60-second period
candles = await client.get_candles("EURUSD_otc", 60, 100)
print(f"Retrieved {len(candles)} candles")
for candle in candles[:5]:  # Show first 5
    print(f"  Time: {candle.time}, Close: {candle.close}")
```

#### Kotlin

```kotlin
// Get last 100 candles with 60-second period
val candles = client.getCandles("EURUSD_otc", 60, 100)
println("Retrieved ${candles.size} candles")
candles.take(5).forEach { candle ->
    println("  Time: ${candle.time}, Close: ${candle.close}")
}
```

#### Swift

```swift
// Get last 100 candles with 60-second period
let candles = try await client.getCandles(asset: "EURUSD_otc", period: 60, offset: 100)
print("Retrieved \(candles.count) candles")
for candle in candles.prefix(5) {
    print("  Time: \(candle.time), Close: \(candle.close)")
}
```

#### Go

```go
// Get last 100 candles with 60-second period
candles, _ := client.GetCandles("EURUSD_otc", 60, 100)
fmt.Printf("Retrieved %d candles\n", len(candles))
for i, candle := range candles {
    if i >= 5 { break }
    fmt.Printf("  Time: %d, Close: %.5f\n", candle.Time, candle.Close)
}
```

#### Ruby

```ruby
# Get last 100 candles with 60-second period
candles = client.get_candles('EURUSD_otc', 60, 100)
puts "Retrieved #{candles.length} candles"
candles.first(5).each do |candle|
  puts "  Time: #{candle.time}, Close: #{candle.close}"
end
```

#### C #

```csharp
// Get last 100 candles with 60-second period
var candles = await client.GetCandlesAsync("EURUSD_otc", 60, 100);
Console.WriteLine($"Retrieved {candles.Count} candles");
foreach (var candle in candles.Take(5))
{
    Console.WriteLine($"  Time: {candle.Time}, Close: {candle.Close}");
}
```

### Get Server Time

#### Python

```python
server_time = await client.server_time()
print(f"Server timestamp: {server_time}")
```

#### Kotlin

```kotlin
val serverTime = client.serverTime()
println("Server timestamp: $serverTime")
```

#### Swift

```swift
let serverTime = await client.serverTime()
print("Server timestamp: \(serverTime)")
```

#### Go

```go
serverTime := client.ServerTime()
fmt.Printf("Server timestamp: %d\n", serverTime)
```

#### Ruby

```ruby
server_time = client.server_time
puts "Server timestamp: #{server_time}"
```

#### C #

```csharp
var serverTime = await client.ServerTimeAsync();
Console.WriteLine($"Server timestamp: {serverTime}");
```

---

## Real-time Subscriptions

### Subscribe to Asset

#### Python

```python
# Subscribe to 60-second candles
subscription = await client.subscribe("EURUSD_otc", 60)

# Receive candles (this is an async iterator in the actual implementation)
# Note: Actual iteration depends on the generated bindings
print("Subscribed to EURUSD_otc")
```

#### Kotlin

```kotlin
// Subscribe to 60-second candles
val subscription = client.subscribe("EURUSD_otc", 60u)
println("Subscribed to EURUSD_otc")

// Receive candles (implementation depends on generated bindings)
```

#### Swift

```swift
// Subscribe to 60-second candles
let subscription = try await client.subscribe(asset: "EURUSD_otc", durationSecs: 60)
print("Subscribed to EURUSD_otc")

// Receive candles (implementation depends on generated bindings)
```

#### Go

```go
// Subscribe to 60-second candles
subscription, _ := client.Subscribe("EURUSD_otc", 60)
fmt.Println("Subscribed to EURUSD_otc")

// Receive candles (implementation depends on generated bindings)
```

#### Ruby

```ruby
# Subscribe to 60-second candles
subscription = client.subscribe('EURUSD_otc', 60)
puts "Subscribed to EURUSD_otc"

# Receive candles (implementation depends on generated bindings)
```

#### C #

```csharp
// Subscribe to 60-second candles
var subscription = await client.SubscribeAsync("EURUSD_otc", 60);
Console.WriteLine("Subscribed to EURUSD_otc");

// Receive candles (implementation depends on generated bindings)
```

### Unsubscribe from Asset

#### Python

```python
await client.unsubscribe("EURUSD_otc")
print("Unsubscribed from EURUSD_otc")
```

#### Kotlin

```kotlin
client.unsubscribe("EURUSD_otc")
println("Unsubscribed from EURUSD_otc")
```

#### Swift

```swift
try await client.unsubscribe(asset: "EURUSD_otc")
print("Unsubscribed from EURUSD_otc")
```

#### Go

```go
client.Unsubscribe("EURUSD_otc")
fmt.Println("Unsubscribed from EURUSD_otc")
```

#### Ruby

```ruby
client.unsubscribe('EURUSD_otc')
puts "Unsubscribed from EURUSD_otc"
```

#### C #

```csharp
await client.UnsubscribeAsync("EURUSD_otc");
Console.WriteLine("Unsubscribed from EURUSD_otc");
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

#### Kotlin

```kotlin
client.reconnect()
delay(2000)
println("Reconnected to server")
```

#### Swift

```swift
try await client.reconnect()
try await Task.sleep(nanoseconds: 2_000_000_000)
print("Reconnected to server")
```

#### Go

```go
client.Reconnect()
time.Sleep(2 * time.Second)
fmt.Println("Reconnected to server")
```

#### Ruby

```ruby
client.reconnect
sleep 2
puts "Reconnected to server"
```

#### C #

```csharp
await client.ReconnectAsync();
await Task.Delay(2000);
Console.WriteLine("Reconnected to server");
```

### Shutdown

#### Python

```python
await client.shutdown()
print("Client shut down gracefully")
```

#### Kotlin

```kotlin
client.shutdown()
println("Client shut down gracefully")
```

#### Swift

```swift
try await client.shutdown()
print("Client shut down gracefully")
```

#### Go

```go
client.Shutdown()
fmt.Println("Client shut down gracefully")
```

#### Ruby

```ruby
client.shutdown
puts "Client shut down gracefully"
```

#### C #

```csharp
await client.ShutdownAsync();
Console.WriteLine("Client shut down gracefully");
```

---

## Error Handling

### Python

```python
from binaryoptionstoolsuni import PocketOption, UniError

try:
    client = await PocketOption.init("invalid_ssid")
    balance = await client.balance()
except UniError as e:
    print(f"Error: {e}")
except Exception as e:
    print(f"Unexpected error: {e}")
```

### Kotlin

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*

try {
    val client = PocketOption.init("invalid_ssid")
    val balance = client.balance()
} catch (e: UniErrorException) {
    println("Error: ${e.message}")
} catch (e: Exception) {
    println("Unexpected error: ${e.message}")
}
```

### Swift

```swift
import BinaryOptionsToolsUni

do {
    let client = try await PocketOption.init(ssid: "invalid_ssid")
    let balance = await client.balance()
} catch let error as UniError {
    print("Error: \(error)")
} catch {
    print("Unexpected error: \(error)")
}
```

### Go

```go
client, err := bot.PocketOptionInit("invalid_ssid")
if err != nil {
    fmt.Printf("Error: %v\n", err)
    return
}

balance := client.Balance()
```

### Ruby

```ruby
begin
  client = BinaryOptionsToolsUni::PocketOption.init('invalid_ssid')
  balance = client.balance
rescue BinaryOptionsToolsUni::UniError => e
  puts "Error: #{e.message}"
rescue => e
  puts "Unexpected error: #{e.message}"
end
```

### C #

```csharp
using BinaryOptionsToolsUni;

try
{
    var client = await PocketOption.InitAsync("invalid_ssid");
    var balance = await client.BalanceAsync();
}
catch (UniErrorException ex)
{
    Console.WriteLine($"Error: {ex.Message}");
}
catch (Exception ex)
{
    Console.WriteLine($"Unexpected error: {ex.Message}");
}
```

---

## Best Practices

### 1. Always Wait for Initialization

All languages should wait 2 seconds after creating the client:

- **Python**: `await asyncio.sleep(2)`
- **Kotlin**: `delay(2000)`
- **Swift**: `try await Task.sleep(nanoseconds: 2_000_000_000)`
- **Go**: `time.Sleep(2 * time.Second)`
- **Ruby**: `sleep 2`
- **C#**: `await Task.Delay(2000)`

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

### Trading Bot Example

See the [examples directory](../examples/) for complete working examples in each language:

- [Python Example](../examples/python/)
- [Kotlin Example](../examples/kotlin/)
- [Swift Example](../examples/swift/)
- [Go Example](../examples/go/)
- [Ruby Example](../examples/ruby/)
- [C# Example](../examples/csharp/)

---

## API Method Reference

| Method                                              | Description                          | Returns                   |
| --------------------------------------------------- | ------------------------------------ | ------------------------- |
| `init(ssid)` / `new(ssid)`                          | Initialize client with session ID    | Client instance           |
| `new_with_url(ssid, url)`                           | Initialize with custom WebSocket URL | Client instance           |
| `balance()`                                         | Get current account balance          | Float                     |
| `is_demo()`                                         | Check if demo account                | Boolean                   |
| `buy(asset, time, amount)`                          | Place call trade                     | Deal object               |
| `sell(asset, time, amount)`                         | Place put trade                      | Deal object               |
| `trade(asset, action, time, amount)`                | Place trade with action              | Deal object               |
| `result(id)`                                        | Check trade result                   | Deal object               |
| `result_with_timeout(id, timeout)`                  | Check trade result with timeout      | Deal object               |
| `get_opened_deals()`                                | Get list of open trades              | List of Deals             |
| `get_closed_deals()`                                | Get list of closed trades            | List of Deals             |
| `clear_closed_deals()`                              | Clear closed trades from memory      | Void                      |
| `get_candles(asset, period, offset)`                | Get historical candles               | List of Candles           |
| `get_candles_advanced(asset, period, time, offset)` | Get historical candles (advanced)    | List of Candles           |
| `history(asset, period)`                            | Get historical data                  | List of Candles           |
| `subscribe(asset, duration)`                        | Subscribe to real-time data          | Subscription              |
| `unsubscribe(asset)`                                | Unsubscribe from asset               | Void                      |
| `server_time()`                                     | Get server timestamp                 | Integer (Unix timestamp)  |
| `assets()`                                          | Get available assets                 | List of Assets (optional) |
| `reconnect()`                                       | Reconnect to server                  | Void                      |
| `shutdown()`                                        | Shutdown client                      | Void                      |

---

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report bugs](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- **Documentation**: [Full docs](https://chipadevteam.github.io/BinaryOptionsTools-v2/)

---

**Version**: 0.1.0  
**Last Updated**: November 2025  
**Platform Support**: PocketOption (Quick Trading)
