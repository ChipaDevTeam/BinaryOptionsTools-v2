# BinaryOptionsTools - Swift Bindings

Swift bindings for BinaryOptionsTools, providing access to PocketOption trading platform via UniFFI. Perfect for iOS and macOS applications.

## 🚀 Features

- ✅ **Trading Operations**: Place buy/sell trades
- ✅ **Account Management**: Get balance and account information
- ✅ **Real-time Data**: Subscribe to asset price feeds
- ✅ **Trade Monitoring**: Check trade results and opened deals

## 📦 Installation

```swift
// Add the binary_options_tools_uni.swift file to your Xcode project
// Make sure to include the native library in your build
```

## 🔧 Quick Start

### Basic Example

```swift
import BinaryOptionsToolsUni

Task {
    // Initialize client with your session ID
    let client = try await PocketOption(ssid: "your-session-id")

    // IMPORTANT: Wait for connection to establish
    try await Task.sleep(nanoseconds: 5_000_000_000)

    // Get account balance
    let balance = try await client.balance()
    print("Account Balance: $\(balance)")

    // Place a buy trade
    let deal = try await client.buy(asset: "EURUSD_otc", time: 60, amount: 1.0)
    print("Trade placed: \(deal)")
}
```

## 📖 Detailed Examples

### Buy Trade Example

```swift
import BinaryOptionsToolsUni

func buyTradeExample() async throws {
    // Initialize client
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)  // Wait for connection

    // Place a buy trade on EURUSD for 60 seconds with $1
    let deal = try await client.buy(
        asset: "EURUSD_otc",
        time: 60,
        amount: 1.0
    )

    print("Trade placed successfully!")
    print("Deal data: \(deal)")
}
```

### Sell Trade Example

```swift
import BinaryOptionsToolsUni

func sellTradeExample() async throws {
    // Initialize client
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)  // Wait for connection

    // Place a sell trade on EURUSD for 60 seconds with $1
    let deal = try await client.sell(
        asset: "EURUSD_otc",
        time: 60,
        amount: 1.0
    )

    print("Trade placed successfully!")
    print("Deal data: \(deal)")
}
```

### Check Balance Example

```swift
import BinaryOptionsToolsUni

func balanceExample() async throws {
    // Initialize client
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)  // Wait for connection

    // Get current balance
    let balance = try await client.balance()
    print("Your current balance is: $\(balance)")
}
```

### Check Trade Result Example

```swift
import BinaryOptionsToolsUni

func checkWinExample() async throws {
    // Initialize client
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)  // Wait for connection

    // Place a trade
    let deal = try await client.buy(asset: "EURUSD_otc", time: 60, amount: 1.0)
    let tradeId = deal.id  // Extract trade ID from deal

    // Wait for trade to complete
    try await Task.sleep(nanoseconds: 65_000_000_000)

    // Check the result
    let result = try await client.checkWin(tradeId: tradeId)
    print("Trade result: \(result)")
}
```

### Subscribe to Real-time Data

```swift
import BinaryOptionsToolsUni

func subscribeExample() async throws {
    // Initialize client
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)  // Wait for connection

    // Subscribe to real-time candle data for EURUSD
    // Duration in seconds for each candle
    let subscription = try await client.subscribe(asset: "EURUSD_otc", durationSecs: 60)

    print("Listening for real-time candles...")
    // Process subscription stream
}
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```swift
let client = try await PocketOption(ssid: "your-session-id")
try await Task.sleep(nanoseconds: 5_000_000_000)  // Critical!
```

### Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12 or Inspect Element)
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

## 🍎 iOS/macOS Integration

This library works great in SwiftUI apps:

```swift
import SwiftUI
import BinaryOptionsToolsUni

struct ContentView: View {
    @State private var balance: Double = 0
    @State private var isLoading = false

    var body: some View {
        VStack {
            if isLoading {
                ProgressView()
            } else {
                Text("Balance: $\(balance, specifier: "%.2f")")
            }

            Button("Get Balance") {
                Task {
                    isLoading = true
                    defer { isLoading = false }

                    let client = try await PocketOption(ssid: "your-session-id")
                    try await Task.sleep(nanoseconds: 5_000_000_000)
                    balance = try await client.balance()
                }
            }
        }
    }
}
```

## 📚 Additional Resources

- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
