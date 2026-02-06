# BinaryOptionsTools - Kotlin Bindings

Kotlin bindings for BinaryOptionsTools, providing access to PocketOption trading platform via UniFFI.

## 🚀 Features

- ✅ **Trading Operations**: Place buy/sell trades
- ✅ **Account Management**: Get balance and account information
- ✅ **Real-time Data**: Subscribe to asset price feeds
- ✅ **Trade Monitoring**: Check trade results and opened deals

## 📦 Installation

```gradle
// Add the generated Kotlin files to your project
// Make sure to include the native library in your build
```

## 🔧 Quick Start

### Basic Example

```kotlin
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun main() {
    // Initialize client with your session ID
    val client = PocketOption.new("your-session-id")

    // IMPORTANT: Wait for connection to establish
    delay(5000)

    // Get account balance
    val balance = client.balance()
    println("Account Balance: $$balance")

    // Place a buy trade
    val deal = client.buy("EURUSD_otc", 60u, 1.0)
    println("Trade placed: $deal")
}
```

## 📖 Detailed Examples

### Buy Trade Example

```kotlin
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun buyTradeExample() {
    // Initialize client
    val client = PocketOption.new("your-session-id")
    delay(5000)  // Wait for connection

    // Place a buy trade on EURUSD for 60 seconds with $1
    val deal = client.buy(
        asset = "EURUSD_otc",
        time = 60u,
        amount = 1.0
    )

    println("Trade placed successfully!")
    println("Deal data: $deal")
}
```

### Sell Trade Example

```kotlin
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun sellTradeExample() {
    // Initialize client
    val client = PocketOption.new("your-session-id")
    delay(5000)  // Wait for connection

    // Place a sell trade on EURUSD for 60 seconds with $1
    val deal = client.sell(
        asset = "EURUSD_otc",
        time = 60u,
        amount = 1.0
    )

    println("Trade placed successfully!")
    println("Deal data: $deal")
}
```

### Check Balance Example

```kotlin
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun balanceExample() {
    // Initialize client
    val client = PocketOption.new("your-session-id")
    delay(5000)  // Wait for connection

    // Get current balance
    val balance = client.balance()
    println("Your current balance is: $$balance")
}
```

### Check Trade Result Example

```kotlin
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun checkWinExample() {
    // Initialize client
    val client = PocketOption.new("your-session-id")
    delay(5000)  // Wait for connection

    // Place a trade
    val deal = client.buy("EURUSD_otc", 60u, 1.0)
    val tradeId = deal.id  // Extract trade ID from deal

    // Wait for trade to complete
    delay(65000)

    // Check the result
    val result = client.checkWin(tradeId)
    println("Trade result: $result")
}
```

### Subscribe to Real-time Data

```kotlin
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun subscribeExample() {
    // Initialize client
    val client = PocketOption.new("your-session-id")
    delay(5000)  // Wait for connection

    // Subscribe to real-time candle data for EURUSD
    // Duration in seconds for each candle
    val subscription = client.subscribe("EURUSD_otc", 60u)

    println("Listening for real-time candles...")
    // Process subscription stream
}
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```kotlin
val client = PocketOption.new("your-session-id")
delay(5000)  // Critical!
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

- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
