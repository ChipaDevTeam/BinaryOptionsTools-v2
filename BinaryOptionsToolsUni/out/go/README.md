# BinaryOptionsTools - Go Bindings

Go bindings for BinaryOptionsTools, providing access to PocketOption trading platform via UniFFI.

## 🚀 Features

- ✅ **Trading Operations**: Place buy/sell trades
- ✅ **Account Management**: Get balance and account information
- ✅ **Real-time Data**: Subscribe to asset price feeds
- ✅ **Trade Monitoring**: Check trade results and opened deals

## 📦 Installation

```bash
# Import the generated Go package
# Make sure the native library is accessible
```

## 🔧 Quick Start

### Basic Example

```go
package main

import (
    "fmt"
    "time"
    "binary_options_tools_uni"
)

func main() {
    // Initialize client with your session ID
    client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
    if err != nil {
        panic(err)
    }

    // IMPORTANT: Wait for connection to establish
    time.Sleep(5 * time.Second)

    // Get account balance
    balance, err := client.Balance()
    if err != nil {
        panic(err)
    }
    fmt.Printf("Account Balance: $%.2f\n", balance)

    // Place a buy trade
    deal, err := client.Buy("EURUSD_otc", 60, 1.0)
    if err != nil {
        panic(err)
    }
    fmt.Printf("Trade placed: %+v\n", deal)
}
```

## 📖 Detailed Examples

### Buy Trade Example

```go
package main

import (
    "fmt"
    "time"
    "binary_options_tools_uni"
)

func buyTradeExample() {
    // Initialize client
    client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
    if err != nil {
        panic(err)
    }

    // Wait for connection
    time.Sleep(5 * time.Second)

    // Place a buy trade on EURUSD for 60 seconds with $1
    deal, err := client.Buy("EURUSD_otc", 60, 1.0)
    if err != nil {
        panic(err)
    }

    fmt.Println("Trade placed successfully!")
    fmt.Printf("Deal data: %+v\n", deal)
}
```

### Sell Trade Example

```go
package main

import (
    "fmt"
    "time"
    "binary_options_tools_uni"
)

func sellTradeExample() {
    // Initialize client
    client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
    if err != nil {
        panic(err)
    }

    // Wait for connection
    time.Sleep(5 * time.Second)

    // Place a sell trade on EURUSD for 60 seconds with $1
    deal, err := client.Sell("EURUSD_otc", 60, 1.0)
    if err != nil {
        panic(err)
    }

    fmt.Println("Trade placed successfully!")
    fmt.Printf("Deal data: %+v\n", deal)
}
```

### Check Balance Example

```go
package main

import (
    "fmt"
    "time"
    "binary_options_tools_uni"
)

func balanceExample() {
    // Initialize client
    client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
    if err != nil {
        panic(err)
    }

    // Wait for connection
    time.Sleep(5 * time.Second)

    // Get current balance
    balance, err := client.Balance()
    if err != nil {
        panic(err)
    }

    fmt.Printf("Your current balance is: $%.2f\n", balance)
}
```

### Check Trade Result Example

```go
package main

import (
    "fmt"
    "time"
    "binary_options_tools_uni"
)

func checkWinExample() {
    // Initialize client
    client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
    if err != nil {
        panic(err)
    }

    // Wait for connection
    time.Sleep(5 * time.Second)

    // Place a trade
    deal, err := client.Buy("EURUSD_otc", 60, 1.0)
    if err != nil {
        panic(err)
    }
    tradeID := deal.ID // Extract trade ID from deal

    // Wait for trade to complete
    time.Sleep(65 * time.Second)

    // Check the result
    result, err := client.CheckWin(tradeID)
    if err != nil {
        panic(err)
    }

    fmt.Printf("Trade result: %+v\n", result)
}
```

### Subscribe to Real-time Data

```go
package main

import (
    "fmt"
    "time"
    "binary_options_tools_uni"
)

func subscribeExample() {
    // Initialize client
    client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
    if err != nil {
        panic(err)
    }

    // Wait for connection
    time.Sleep(5 * time.Second)

    // Subscribe to real-time candle data for EURUSD
    // Duration in seconds for each candle
    subscription, err := client.Subscribe("EURUSD_otc", 60)
    if err != nil {
        panic(err)
    }

    fmt.Println("Listening for real-time candles...")
    // Process subscription stream
    _ = subscription
}
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```go
client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
if err != nil {
    panic(err)
}
time.Sleep(5 * time.Second)  // Critical!
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
