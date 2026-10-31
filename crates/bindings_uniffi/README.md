# BinaryOptionsToolsUni

> **✨ [ChipaEditor](https://chipaeditor.com/?utm_source=github&utm_medium=readme&utm_campaign=BinaryOptionsToolsV2&utm_content=uniffi) — AI-powered algorithmic _trading strategy_ builder: describe your edge, get working CHTL code, backtest it, deploy it. Free to start.**
>
> **📈 Trade crypto perps, spot & margin on [ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS) — demo mode available.**

Cross-platform library for binary options trading automation using UniFFI. Provides native bindings for multiple programming languages from a single Rust codebase.

## 🌍 Supported Languages

- **C#** (.NET/Mono)
- **Go**
- **Kotlin** (JVM/Android)
- **Python** (3.10+)
- **Ruby** (2.7+)
- **Swift** (iOS/macOS)

## 📦 Installation

None of these bindings are published to a package registry — there is no NuGet,
Maven, RubyGems, Go module or Swift package to install. Every language is
generated from this crate with `uniffi-bindgen` (proc-macro based, no `.udl`):

```bash
# generate the bindings for one language
cargo run -p binary_options_tools_uni --bin uniffi-bindgen generate \
    --library --language kotlin --out-dir out/kotlin

# available: kotlin, swift, python, ruby
# then build the shared library and add it to your project
cargo build --release -p binary_options_tools_uni
```

The in-repo generator emits `kotlin`, `swift`, `python` and `ruby` only. Go and C#
need their community bindgens (`uniffi-bindgen-go`, `uniffi-bindgen-cs`) run against
the same compiled library.

If you only need Python, use the published wheel instead
(`pip install binaryoptionstoolsv2`, Python 3.10+) — it is not built from this
crate.

## 🚀 Quick Start

> **Known limitation (verified 2026-10-31):** the **Kotlin and Python** generators
> currently fail with `Async primary constructors not supported but PocketOption
has one`, because `PocketOption::new` is an `async` `#[uniffi::constructor]`.
> Swift and Ruby generate successfully. Use `new_with_url` / `new_with_config`
> (below) as the entry point, or a language whose generator succeeds.

The constructor is asynchronous — it returns only after the auth handshake, so a
fixed `sleep` is not required. Generated names differ per language; these are the
actual emitted signatures:

| Language | Construct                                                                      |
| -------- | ------------------------------------------------------------------------------ |
| Swift    | `try await PocketOption(ssid: "...")`                                          |
| Ruby     | `PocketOption.new_with_url(ssid, url)` or `.new_with_config(ssid, urls, secs)` |
| Kotlin   | not generated yet (see limitation above)                                       |
| Python   | not generated yet (see limitation above)                                       |

### Swift

```swift
import binary_options_tools_uni

let client = try await PocketOption(ssid: "your_ssid")
let balance = try await client.balance()
print("Balance: \(balance)")

let trade = try await client.buy(asset: "EURUSD_otc", time: 60, amount: 1.0)
print("Trade ID: \(trade.id)")
try await client.shutdown()
```

Note the argument order for `buy`: `(asset, time, amount)` — `amount` last, unlike
the Python and Node bindings.

### Ruby

```ruby
require 'binary_options_tools_uni'
require 'async'

Async do
  client = PocketOption.new_with_config('your_ssid', ['wss://api-eu.po.market/socket.io/?EIO=4&transport=websocket'], 30)
  puts "Balance: $#{client.balance}"

  trade = client.buy('EURUSD_otc', 60, 1.0)
  puts "Trade ID: #{trade.id}"
end
```

### Go and C#

Both need an external bindgen (`uniffi-bindgen-go`, `uniffi-bindgen-cs`) run
against the compiled `binary_options_tools_uni` library; nothing is published.
Their generated naming follows the same pattern (`NewPocketOption`, `Balance`,
`Buy`).

## 📚 Documentation

Comprehensive API documentation with examples in all supported languages:

- **[API Reference](../../docs/api/reference.md)** - Complete API documentation with multi-language examples
- **[Trading Guide](../../docs/guides/trading.md)** - Learn how to place trades and manage orders
- **[Candle history limits](../../docs/guides/candle-history-limits.md)** - What the broker's history endpoint returns
- **[Examples](../../examples/)** - Working code examples for each language

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
cargo run -p binary_options_tools_uni --bin uniffi-bindgen generate --library --language kotlin --out-dir out/kotlin
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

- **GitHub Issues**: [Report bugs](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/issues)
- **Documentation**: [Full docs](docs/api/reference.md)

## 🔗 Related Projects

- **[BinaryOptionsToolsV2](../BinaryOptionsToolsV2/)** - Python-specific bindings with PyO3
- **[binary_options_tools](../crates/binary_options_tools/)** - Core Rust library

---

**Platform Support**: Currently supporting **PocketOption** (Quick Trading Mode) with both real and demo accounts.
