# BinaryOptionsToolsUni

Cross-platform library for binary options trading automation using UniFFI. Provides native bindings for multiple programming languages from a single Rust codebase.

## 🌍 Supported Languages

- **C#** (.NET/Mono)
- **Go**
- **Kotlin** (JVM/Android)
- **Python** (3.8+)
- **Ruby** (2.7+)
- **Swift** (iOS/macOS)

## 📦 Installation

### C# (.NET)

```bash
# Coming soon - NuGet package
dotnet add package BinaryOptionsToolsUni
```

### Go

```bash
go get gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/bindings/go
```

### Kotlin

```gradle
dependencies {
    implementation 'com.chipadevteam:binaryoptionstoolsuni:0.1.0'
}
```

### Python

```bash
pip install binaryoptionstoolsuni
```

### Ruby

```bash
gem install binaryoptionstoolsuni
```

### Swift

```swift
// Add to Package.swift
dependencies: {
    .package(url: "https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2", from: "0.1.0")
}
```

## 🚀 Quick Start

All languages follow the same API structure. Here's a quick example in each supported language:

### Python

```python
import asyncio
from binaryoptionstoolsuni import PocketOption

async def main():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)  # Wait for initialization

    balance = await client.balance()
    print(f"Balance: ${balance}")

    # Place a trade
    trade = await client.buy("EURUSD_otc", 60, 1.0)
    print(f"Trade ID: {trade.id}")

asyncio.run(main())
```

### Kotlin

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*
import kotlinx.coroutines.*

suspend fun main() = coroutineScope {
    val client = PocketOption.init("your_ssid")
    delay(2000) // Wait for initialization

    val balance = client.balance()
    println("Balance: $$balance")

    // Place a trade
    val trade = client.buy("EURUSD_otc", 60u, 1.0)
    println("Trade ID: ${trade.id}")
}
```

### Swift

```swift
import BinaryOptionsToolsUni

Task {
    let client = try await PocketOption.init(ssid: "your_ssid")
    try await Task.sleep(nanoseconds: 2_000_000_000) // Wait for initialization

    let balance = await client.balance()
    print("Balance: $\(balance)")

    // Place a trade
    let trade = try await client.buy(asset: "EURUSD_otc", time: 60, amount: 1.0)
    print("Trade ID: \(trade.id)")
}
```

### Go

```go
package main

import (
    "fmt"
    "time"
    bot "binaryoptionstoolsuni"
)

func main() {
    client, err := bot.PocketOptionInit("your_ssid")
    if err != nil {
        panic(err)
    }
    time.Sleep(2 * time.Second) // Wait for initialization

    balance := client.Balance()
    fmt.Printf("Balance: $%.2f\n", balance)

    // Place a trade
    trade, err := client.Buy("EURUSD_otc", 60, 1.0)
    if err != nil {
        panic(err)
    }
    fmt.Printf("Trade ID: %s\n", trade.Id)
}
```

### Ruby

```ruby
require 'binaryoptionstoolsuni'
require 'async'

Async do
  client = BinaryOptionsToolsUni::PocketOption.init('your_ssid')
  sleep 2 # Wait for initialization

  balance = client.balance
  puts "Balance: $#{balance}"

  # Place a trade
  trade = client.buy('EURUSD_otc', 60, 1.0)
  puts "Trade ID: #{trade.id}"
end
```

### C

```csharp
using BinaryOptionsToolsUni;

var client = await PocketOption.InitAsync("your_ssid");
await Task.Delay(2000); // Wait for initialization

var balance = await client.BalanceAsync();
Console.WriteLine($"Balance: ${balance}");

// Place a trade
var trade = await client.BuyAsync("EURUSD_otc", 60, 1.0);
Console.WriteLine($"Trade ID: {trade.Id}");
```

## 📚 Documentation

Comprehensive API documentation with examples in all supported languages:

- **[Full API Reference](docs/API_REFERENCE.md)** - Complete API documentation with multi-language examples
- **[Trading Guide](docs/TRADING_GUIDE.md)** - Learn how to place trades and manage orders
- **[Market Data Guide](docs/MARKET_DATA_GUIDE.md)** - Access real-time and historical data
- **[Examples](docs/examples/)** - Working code examples for each language

## ✨ Features

### Trading Operations

- ✅ Place Call/Put trades
- ✅ Check trade results
- ✅ Get open and closed deals
- ✅ Support for both demo and real accounts

### Account Management

- ✅ Get account balance
- ✅ Check demo/real account status
- ✅ Manage trade history

### Market Data

- ✅ Get historical candles (OHLC data)
- ✅ Subscribe to real-time price updates
- ✅ Get asset information and payouts
- ✅ Server time synchronization

### Connection Management

- ✅ Automatic reconnection
- ✅ Connection state management
- ✅ Custom WebSocket URLs
- ✅ Graceful shutdown

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│         Application Code                │
│  (C#, Go, Kotlin, Python, Ruby, Swift)  │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│          UniFFI Bindings                │
│     (Generated Language Bindings)        │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│      Rust Core (BinaryOptionsToolsUni)  │
│         binary_options_tools            │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│      PocketOption WebSocket API         │
└─────────────────────────────────────────┘
```

## 🔧 Building from Source

### Prerequisites

- Rust 1.70+ with `cargo`
- UniFFI CLI: `cargo install uniffi_bindgen`
- Target language toolchains (as needed)

### Build Steps

```bash
# Clone the repository
git clone https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2.git
cd BinaryOptionsTools-v2/BinaryOptionsToolsUni

# Build the Rust library
cargo build --release

# Generate bindings for your target language
cargo run --bin uniffi-bindgen generate src/binary_options_tools_uni.udl \
    --language <python|kotlin|swift|ruby> \
    --out-dir out/<language>
```

## 🤝 Contributing

Contributions are welcome! Please ensure:

1. Code follows language-specific best practices
2. All tests pass
3. New features include examples for all supported languages
4. Documentation is updated

## 📄 License

See [LICENSE](../LICENSE) file for details.

**Personal Use** - Free for personal, educational, and non-commercial use.
**Commercial Use** - Requires explicit written permission. Contact us on [Discord](https://discord.gg/p7YyFqSmAz).

## ⚠️ Disclaimer

This software is provided "AS IS" without warranty. The authors and ChipaDevTeam are NOT responsible for:

- Any financial losses incurred from using this software
- Any trading decisions made using this software
- Any bugs, errors, or issues in the software

Binary options trading carries significant risk. Use at your own risk.

- **GitLab Issues**: [Report bugs](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/issues)
- **Documentation**: [Full docs](https://chipatrade.gitlab.io/chipadevorg/BinaryOptionsTools-v2/)

## 🔗 Related Projects

- **[BinaryOptionsToolsV2](../BinaryOptionsToolsV2/)** - Python-specific bindings with PyO3
- **[binary_options_tools](../crates/binary_options_tools/)** - Core Rust library

---

**Platform Support**: Currently supporting **PocketOption** (Quick Trading Mode) with both real and demo accounts.
