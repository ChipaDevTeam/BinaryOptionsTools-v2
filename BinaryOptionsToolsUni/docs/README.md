# BinaryOptionsToolsUni Documentation

Welcome to the comprehensive documentation for **BinaryOptionsToolsUni** - Multi-language bindings for binary options trading.

## 🌍 Supported Languages

BinaryOptionsToolsUni provides native bindings for:

- 🐍 **Python** - Async/await support
- 🟣 **Kotlin** - Coroutines support  
- 🍎 **Swift** - Modern Swift with async/await
- 🔷 **Go** - Goroutines and channels
- 💎 **Ruby** - Async Fiber support
- 🔵 **C#** - Task-based async

## 📚 Documentation

### Getting Started

- **[README](../README.md)** - Project overview and quick start
- **[API Reference](API_REFERENCE.html)** ⭐ - Interactive multi-language API reference
- **[API Reference (Markdown)](API_REFERENCE.md)** - Markdown version for offline use

### Guides

- **[Trading Guide](TRADING_GUIDE.md)** - Complete trading guide with strategies
- **[Installation Guide](#installation)** - Language-specific installation
- **[Examples](#examples)** - Code examples for each language

---

## 🚀 Quick Start

### Python

```bash
pip install binaryoptionstoolsuni
```

```python
import asyncio
from binaryoptionstoolsuni import PocketOption

async def main():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)
    
    balance = await client.balance()
    print(f"Balance: ${balance}")
    
    await client.shutdown()

asyncio.run(main())
```

### Kotlin

```gradle
dependencies {
    implementation 'com.chipadevteam:binaryoptionstoolsuni:0.1.0'
}
```

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*
import kotlinx.coroutines.*

suspend fun main() = coroutineScope {
    val client = PocketOption.init("your_ssid")
    delay(2000)
    
    val balance = client.balance()
    println("Balance: $$balance")
    
    client.shutdown()
}
```

### Swift

Add to `Package.swift`:
```swift
dependencies: [
    .package(url: "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2", from: "0.1.0")
]
```

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

### Go

```bash
go get github.com/ChipaDevTeam/BinaryOptionsTools-v2/bindings/go
```

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

### Ruby

```bash
gem install binaryoptionstoolsuni
```

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

### C#

```bash
dotnet add package BinaryOptionsToolsUni
```

```csharp
using BinaryOptionsToolsUni;

var client = await PocketOption.InitAsync("your_ssid");
await Task.Delay(2000);

var balance = await client.BalanceAsync();
Console.WriteLine($"Balance: ${balance}");

await client.ShutdownAsync();
```

---

## 📖 Core Concepts

### Client Initialization

All languages follow the same pattern:

1. **Initialize** client with SSID
2. **Wait 2 seconds** for connection to establish
3. **Use** the client for trading operations
4. **Shutdown** gracefully when done

### Async/Await Pattern

All operations are asynchronous to provide optimal performance:

- **Python**: `async`/`await` with `asyncio`
- **Kotlin**: `suspend` functions with coroutines
- **Swift**: `async`/`await` with Tasks
- **Go**: Synchronous API (internally async)
- **Ruby**: Async with Fibers
- **C#**: `async`/`await` with Tasks

### Error Handling

All languages provide exception/error handling:

```python
# Python
try:
    client = await PocketOption.init("ssid")
except UniError as e:
    print(f"Error: {e}")
```

```kotlin
// Kotlin
try {
    val client = PocketOption.init("ssid")
} catch (e: UniErrorException) {
    println("Error: ${e.message}")
}
```

---

## 🎯 Features

### Trading Operations

| Feature | Description | Status |
|---------|-------------|--------|
| **Place Call Trade** | Buy/Long position | ✅ Available |
| **Place Put Trade** | Sell/Short position | ✅ Available |
| **Check Trade Result** | Get win/loss outcome | ✅ Available |
| **Result with Timeout** | Wait for result with timeout | ✅ Available |

### Account Management

| Feature | Description | Status |
|---------|-------------|--------|
| **Get Balance** | Current account balance | ✅ Available |
| **Check Demo/Real** | Verify account type | ✅ Available |
| **Get Open Deals** | List active trades | ✅ Available |
| **Get Closed Deals** | List completed trades | ✅ Available |

### Market Data

| Feature | Description | Status |
|---------|-------------|--------|
| **Historical Candles** | Get OHLC candle data | ✅ Available |
| **Advanced Candles** | Candles with timestamp | ✅ Available |
| **Server Time** | Get server timestamp | ✅ Available |
| **Real-time Subscribe** | Subscribe to live data | ✅ Available |

### Connection Management

| Feature | Description | Status |
|---------|-------------|--------|
| **Reconnect** | Reconnect to server | ✅ Available |
| **Shutdown** | Graceful disconnect | ✅ Available |
| **Custom URL** | Use custom WebSocket URL | ✅ Available |

---

## 📂 Project Structure

```
BinaryOptionsToolsUni/
├── docs/                      # Documentation
│   ├── README.md             # This file
│   ├── API_REFERENCE.md      # API reference (Markdown)
│   ├── API_REFERENCE.html    # Interactive API reference
│   └── TRADING_GUIDE.md      # Trading strategies guide
├── src/                       # Rust source code
│   ├── lib.rs                # Library entry point
│   ├── error.rs              # Error types
│   ├── tracing.rs            # Logging support
│   └── platforms/
│       └── pocketoption/
│           ├── mod.rs        # Module definition
│           └── client.rs     # PocketOption client
├── out/                       # Generated bindings
│   ├── python/               # Python bindings
│   ├── kotlin/               # Kotlin bindings
│   ├── swift/                # Swift bindings
│   ├── go/                   # Go bindings
│   ├── ruby/                 # Ruby bindings
│   └── cs/                   # C# bindings
├── Cargo.toml                # Rust dependencies
└── README.md                 # Project README
```

---

## 🔨 Building from Source

### Prerequisites

- **Rust** 1.70+ (`rustup` recommended)
- **cargo** (comes with Rust)
- **uniffi-bindgen** for generating bindings

### Build Steps

1. **Clone the repository**:
```bash
git clone https://github.com/ChipaDevTeam/BinaryOptionsTools-v2
cd BinaryOptionsTools-v2/BinaryOptionsToolsUni
```

2. **Build the Rust library**:
```bash
cargo build --release
```

3. **Generate bindings** (if using custom build):
```bash
cargo run --bin uniffi-bindgen
```

### Language-Specific Setup

<details>
<summary><b>Python Setup</b></summary>

```bash
# Install maturin for building Python wheels
pip install maturin

# Build and install
maturin develop --release
```
</details>

<details>
<summary><b>Kotlin Setup</b></summary>

```bash
# Copy generated Kotlin files to your project
cp -r out/kotlin/* your-project/src/main/kotlin/

# Add JAR to your build.gradle
```
</details>

<details>
<summary><b>Swift Setup</b></summary>

```bash
# Copy generated Swift files
cp -r out/swift/* your-project/Sources/

# Link against the Rust library
```
</details>

<details>
<summary><b>Go Setup</b></summary>

```bash
# Copy generated Go files
cp -r out/go/* your-project/

# Import in your code
```
</details>

<details>
<summary><b>Ruby Setup</b></summary>

```bash
# Build Ruby gem
gem build binaryoptionstoolsuni.gemspec
gem install binaryoptionstoolsuni-0.1.0.gem
```
</details>

<details>
<summary><b>C# Setup</b></summary>

```bash
# Copy generated C# files
cp -r out/cs/* your-project/

# Add reference in your .csproj
```
</details>

---

## 🧪 Testing

### Running Tests

```bash
# Run Rust tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_client_init
```

### Example Test

```rust
#[tokio::test]
async fn test_balance() {
    let client = PocketOption::init("test_ssid").await.unwrap();
    let balance = client.balance().await;
    assert!(balance >= 0.0);
}
```

---

## 🎓 Examples

See the [examples directory](../examples/) for complete working examples:

### Basic Examples

- **Get Balance**: Check your account balance
- **Place Trade**: Execute a buy/sell trade
- **Check Result**: Get trade outcome
- **Historical Data**: Fetch candle data

### Advanced Examples

- **Trading Bot**: Automated trading strategy
- **Multi-Asset**: Trade multiple pairs
- **Risk Management**: Position sizing and limits
- **Real-time Monitoring**: Subscribe to live data

---

## 🐛 Troubleshooting

### Common Issues

#### Build Errors

**Problem**: Compilation fails

**Solution**:
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

#### Import Errors

**Problem**: Can't import the library

**Solution**:
- Verify installation path
- Check language-specific setup
- Ensure bindings were generated
- Check library is in system path

#### Connection Errors

**Problem**: Can't connect to PocketOption

**Solution**:
- Verify SSID is valid and not expired
- Check internet connection
- Try in browser first
- Wait 2 seconds after initialization

---

## 📊 Performance

### Benchmarks

| Operation | Time | Notes |
|-----------|------|-------|
| Initialize Client | ~2s | Includes WebSocket connection |
| Place Trade | ~100-500ms | Network dependent |
| Get Balance | ~50-200ms | Fast operation |
| Get Candles (100) | ~200-500ms | Depends on data size |
| Subscribe | ~100-300ms | One-time setup |

### Optimization Tips

1. **Reuse clients**: Don't create new clients for each operation
2. **Batch operations**: Group related calls together
3. **Use subscriptions**: For real-time data instead of polling
4. **Connection pooling**: Maintain persistent connections
5. **Error handling**: Implement retries for transient failures

---

## 🔒 Security

### Best Practices

1. **Never hardcode SSID**: Use environment variables
2. **Secure storage**: Store credentials securely
3. **HTTPS only**: Always use secure connections
4. **Validate inputs**: Check all user inputs
5. **Rate limiting**: Avoid excessive API calls

### Environment Variables

```bash
# .env file
POCKETOPTION_SSID=your_ssid_here
DEMO_MODE=true
MAX_TRADE_SIZE=10.0
```

```python
import os
from dotenv import load_dotenv

load_dotenv()
ssid = os.getenv("POCKETOPTION_SSID")
```

---

## 📜 License

**Free for Personal Use**

Commercial use requires written permission from ChipaDev Team.

See [LICENSE](../../LICENSE) for full terms.

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

---

## 📞 Support

### Community

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report bugs](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- **Discussions**: [Ask questions](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/discussions)

### Resources

- **API Reference**: See [API_REFERENCE.html](API_REFERENCE.html)
- **Trading Guide**: See [TRADING_GUIDE.md](TRADING_GUIDE.md)
- **Main Docs**: [Full documentation](https://chipadevteam.github.io/BinaryOptionsTools-v2/)

---

## 🗺️ Roadmap

### Current (v0.1.0)

- ✅ Multi-language bindings (6 languages)
- ✅ Basic trading operations
- ✅ Account management
- ✅ Historical data
- ✅ Real-time subscriptions

### Planned (v0.2.0)

- ⏳ Payout calculation
- ⏳ Raw message handler
- ⏳ Validator system
- ⏳ Multiple subscription types (chunked, timed, aligned)
- ⏳ Connection lifecycle events

### Future

- 📋 More platforms (Expert Option, Quotex, IQ Option)
- 📋 WebSocket reconnection strategies
- 📋 Built-in trading strategies
- 📋 Backtesting framework
- 📋 Performance monitoring

---

## 🏆 Acknowledgments

Built with:

- **[UniFFI](https://mozilla.github.io/uniffi-rs/)** - Multi-language bindings
- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Tungstenite](https://github.com/snapview/tungstenite-rs)** - WebSocket client
- **[Serde](https://serde.rs/)** - Serialization

Special thanks to all contributors and the community!

---

**Version**: 0.1.0  
**Last Updated**: November 2025  
**Platform**: PocketOption (Quick Trading)
