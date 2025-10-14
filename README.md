# BinaryOptionsTools V2

[![Discord](https://img.shields.io/discord/your-discord-id?color=7289da&label=Discord&logo=discord&logoColor=white)](https://discord.gg/p7YyFqSmAz)
[![Crates.io](https://img.shields.io/crates/v/binary_options_tools.svg)](https://crates.io/crates/binary_options_tools)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://pypi.org/project/binaryoptionstoolsv2/)

A powerful, multi-language library for automated binary options trading. Built with Rust for performance and safety, with bindings for Python, JavaScript, C#, Go, Kotlin, Ruby, and Swift.

## 🚀 Features

Currently we support **PocketOption** (quick trading) with the following features (for real and demo accounts):

- ✅ **Trading Operations**: Place buy/sell trades for any asset
- ✅ **Trade Management**: Check trade results with optional timeout
- ✅ **Account Information**: Get account balance and server time synchronization
- ✅ **Asset Data**: Get payout information for each asset
- ✅ **Real-time Data**: Subscribe to assets for real-time price data with different subscription types
- ✅ **Trade Monitoring**: Get list of opened trades with all trade data
- ✅ **Asset Validation**: Validate assets and retrieve detailed information
- ✅ **Connection Management**: Automatic reconnection and connection status monitoring

## 📋 TODO Features

- ⏳ Historical candle data retrieval
- ⏳ Closed trades management and history
- ⏳ Pending trades support
- ⏳ Additional trading platforms (Expert Options, etc.)

## 💬 Support & Community

If you are looking to build a bot, let us build it for you! Check [Chipa's shop](https://shop.chipatrade.com/collections/all)

**Support us and our contributors:**
- Join PocketOption with [Chipa's affiliate link](https://u3.shortink.io/smart/SDIaxbeamcYYqB)
- Join PocketOption with [Six's affiliate link](https://u3.shortink.io?utm_campaign=821725&utm_source=affiliate&utm_medium=sr&a=IqeAmBtFTrEWbh&ac=api)
- Donate at [PayPal](https://paypal.me/ChipaCL)
- Join us on [Patreon](https://patreon.com/VigoDEV?utm_medium=unknown&utm_source=join_link&utm_campaign=creatorshare_creator&utm_content=copyLink)
- Join our [Discord community](https://discord.gg/p7YyFqSmAz)

Don't know programming and you are looking for a bot to automate YOUR strategy? [Get our development services!](https://shop.chipatrade.com/)

# Features
Currently we only support **Pocket Option** (quick trading) with the following features (for real and demo):
* Place trades for any asset (buy/sell)
* Check trade results with optional timeout
* Get account balance
* Get server time synchronization
* Get the payout of each asset
* Get a list with the opened trades with all of the trades data
* Subscribe to an asset to get realtime data with different subscription types
* Asset validation and information retrieval
* Automatic reconnection and connection management

## TODO Features
* Get a list with the closed trades with all of the trades data
* Get candle data for a specific asset (historical data)
* Add support for pending trades
* Add support for other trading platforms like Expert Options

## 🌍 Supported Languages

We provide bindings for multiple programming languages:

- **[Python](./BinaryOptionsToolsV2/Readme.md)** - Full sync and async support (Python 3.8+)
- **[Rust](./crates/binary_options_tools/Readme.md)** - Native async implementation
- **[JavaScript/Node.js](./BinaryOptionsToolsUni/out/javascript/README.md)** - Async support via UniFFI bindings
- **[C#](./BinaryOptionsToolsUni/out/cs/README.md)** - .NET support via UniFFI bindings
- **[Go](./BinaryOptionsToolsUni/out/go/README.md)** - Go support via UniFFI bindings
- **[Kotlin](./BinaryOptionsToolsUni/out/kotlin/README.md)** - JVM support via UniFFI bindings
- **[Ruby](./BinaryOptionsToolsUni/out/ruby/README.md)** - Ruby support via UniFFI bindings
- **[Swift](./BinaryOptionsToolsUni/out/swift/README.md)** - iOS/macOS support via UniFFI bindings

## 📦 Quick Start

### Python

**Installation:**
```bash
# Windows
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/blob/master/wheels/BinaryOptionsToolsV2-0.1.8-cp38-abi3-win_amd64.whl?raw=true"

# Linux
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/blob/master/wheels/BinaryOptionsToolsV2-0.2.0-cp38-abi3-manylinux_2_34_x86_64.whl?raw=true"
```

**Quick Example (Synchronous):**
```python
from BinaryOptionsToolsV2.pocketoption import PocketOption
import time

# Initialize client
client = PocketOption(ssid="your-session-id")
time.sleep(5)  # Wait for connection to establish

# Get balance
balance = client.balance()
print(f"Account Balance: ${balance}")

# Place a buy trade
trade_id, deal = client.buy("EURUSD_otc", 60, 1.0)
print(f"Trade placed: {deal}")

# Check result
result = client.check_win(trade_id)
print(f"Trade result: {result}")
```

**Quick Example (Asynchronous):**
```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import asyncio

async def main():
    # Initialize client
    client = PocketOptionAsync(ssid="your-session-id")
    await asyncio.sleep(5)  # Wait for connection to establish
    
    # Get balance
    balance = await client.balance()
    print(f"Account Balance: ${balance}")
    
    # Place a buy trade
    trade_id, deal = await client.buy("EURUSD_otc", 60, 1.0)
    print(f"Trade placed: {deal}")
    
    # Check result
    result = await client.check_win(trade_id)
    print(f"Trade result: {result}")

asyncio.run(main())
```

### Rust

**Installation:**

Add to your `Cargo.toml`:
```toml
[dependencies]
binary_options_tools = "0.1"
tokio = { version = "1", features = ["full"] }
```

**Quick Example:**
```rust
use binary_options_tools::PocketOption;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client
    let client = PocketOption::new("your-session-id").await?;
    
    // Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Get balance
    let balance = client.balance().await;
    println!("Account Balance: ${}", balance);
    
    // Place a buy trade
    let (trade_id, deal) = client.buy("EURUSD_otc", 60, 1.0).await?;
    println!("Trade placed: {:?}", deal);
    
    // Check result
    let result = client.result(trade_id).await?;
    println!("Trade result: {:?}", result);
    
    Ok(())
}
```

## 📚 Documentation

- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Python API**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/python.html](https://chipadevteam.github.io/BinaryOptionsTools-v2/python.html)
- **Rust API**: [https://docs.rs/binary_options_tools](https://docs.rs/binary_options_tools)

## 💡 Examples

You can find comprehensive examples for all features in the [examples](./examples) directory:

- **[Python Examples](./examples/python)** - Sync and async implementations
- **[JavaScript Examples](./examples/javascript)** - Node.js examples
- More language examples coming soon!

## 🔧 Building from Source

**Prerequisites:**
- Rust and Cargo installed ([Install Rust](https://rustup.rs/))
- Python 3.8+ (for Python bindings)
- [Maturin](https://www.maturin.rs/installation) (for building Python wheels)

**Build Python Wheel:**
```bash
cd BinaryOptionsToolsV2
maturin build -r
```

**Build Rust Crate:**
```bash
cargo build --release
```

## 🤝 Contributing

We welcome contributions! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the terms specified in the [LICENSE](./LICENSE) file.

## ⚠️ Disclaimer

This software is for educational purposes only. Trading binary options involves substantial risk of loss and is not suitable for all investors. Use at your own risk.
