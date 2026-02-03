# BinaryOptionsTools v2

A high-performance, cross-platform library for binary options trading automation. Built with Rust for speed and reliability, with Python bindings for ease of use.

**Need help?** Join us on [Discord](https://discord.gg/p7YyFqSmAz) for support and discussions.

## Overview

BinaryOptionsTools v2 is a complete rewrite of the original library, featuring:

- **Rust Core**: Built with Rust for maximum performance and memory safety
- **Python Bindings**: Easy-to-use Python API via PyO3
- **WebSocket Support**: Real-time market data streaming and trade execution
- **Type Safety**: Strong typing across both Rust and Python interfaces
- **Connection Management**: Automatic reconnection and error handling
- **Raw API Access**: Low-level WebSocket control for advanced use cases

## Supported Platforms

Currently supporting **PocketOption** (Quick Trading Mode) with both real and demo accounts.

## Current Status

**Available Features**:

- Authentication and secure connection
- Buy/Sell trading operations
- Balance retrieval
- Server time synchronization
- Symbol subscriptions with different types (real-time, time-aligned, chunked)
- Trade result checking
- Opened deals management
- Asset information and validation
- Automatic reconnection handling
- Historical candle data (`get_candles`, `get_candles_advanced`)
- Advanced validators

We're working to restore all functionality with improved stability and performance.

## Features

### Trading Operations

- **Trade Execution**: Place buy/sell orders on any available asset
- **Trade Monitoring**: Check trade results with configurable timeouts
- **Balance Management**: Real-time account balance retrieval
- **Open/Closed Deals**: Access active positions and closed deals

### Market Data

- **Real-time Candle Streaming**: Subscribe to live price data with multiple timeframes (1s, 5s, 15s, 30s, 60s, 300s)
- **Historical Candles**: Fetch historical OHLC data for backtesting and analysis
- **Time-Aligned Subscriptions**: Get perfectly aligned candle data for strategy execution
- **Payout Information**: Retrieve current payout percentages for all assets

### Connection Management

- **Automatic Reconnection**: Built-in connection recovery with exponential backoff
- **Connection Control**: Manual connect/disconnect/reconnect methods
- **Subscription Management**: Unsubscribe from specific assets or handlers
- **WebSocket Health Monitoring**: Automatic ping/pong keepalive

### Advanced Features

- **Raw Handler API**: Low-level WebSocket access for custom protocol implementations
- **Message Validation**: Built-in validator system for response filtering
- **Async/Sync Support**: Both asynchronous and synchronous Python APIs
- **Asset Validation**: Automatic verification of trading pairs and OTC availability
- **Server Time Sync**: Accurate server timestamp synchronization

## Architecture

```
┌─────────────────────────────────────────┐
│         User Application                │
│      (Python/Rust/JavaScript)           │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│      Language Bindings (PyO3)           │
│    Python Async/Sync API Wrappers       │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│         Rust Core Library               │
│  binary_options_tools / core-pre        │
│  • WebSocket Client (tungstenite)       │
│  • Connection Manager                   │
│  • Message Router & Validators          │
│  • Raw Handler System                   │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│      PocketOption WebSocket API         │
└─────────────────────────────────────────┘
```

## Installation

### Python

#### Using pip (Prebuilt Wheels)

```bash
# Windows
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/BinaryOptionsToolsV2-0.2.3/BinaryOptionsToolsV2-0.2.3-cp38-abi3-win_amd64.whl"

# Linux
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/BinaryOptionsToolsV2-0.2.3/BinaryOptionsToolsV2-0.2.3-cp38-abi3-manylinux_2_34_x86_64.whl"

# Mac
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/BinaryOptionsToolsV2-0.2.3/BinaryOptionsToolsV2-0.2.3-cp38-abi3-macosx_11_0_arm64.whl"
```

**Requirements**:

- **OS**: Windows, Linux, macOS
- **Python**: 3.8 - 3.12

#### Building from Source

```bash
# Clone the repository
git clone https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git
cd BinaryOptionsTools-v2/BinaryOptionsToolsV2

# Install maturin (if not already installed)
pip install maturin

# Build and install
maturin develop --release
```

### Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
binary_options_tools = { path = "crates/binary_options_tools" }
```

## Quick Start

### Python - Async API

```python
from BinaryOptionsToolsV2 import PocketOptionAsync
import asyncio
import os

async def main():
    # Initialize client with SSID from environment variable
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        raise ValueError("Please set POCKET_OPTION_SSID environment variable")
    client = PocketOptionAsync(ssid=ssid)
    # Connection is established automatically during initialization
    
    # Get account balance
    balance = await client.balance()
    print(f"Balance: ${balance}")
    
    # Place a trade
    asset = "EURUSD_otc"
    amount = 1.0  # $1
    duration = 60  # 60 seconds
    
    # The `buy` function is used for "call" trades. For "put" trades, use the `sell` method.
    trade_id, deal = await client.buy(asset, amount, duration)
    # trade_id, deal = await client.sell(asset, amount, duration)
    print(f"Trade placed: {deal}")
    
    # Check if trade won
    result = await client.check_win(trade_id)
    print(f"Trade result: {result}")
    
    # Disconnect
    await client.disconnect()

asyncio.run(main())
```

### Python - Sync API

```python
from BinaryOptionsToolsV2 import PocketOption
import time

# Initialize client
client = PocketOption(ssid="your-session-id")
# Connection is established automatically during initialization

# Place trade (blocking)
# The `buy` function is used for "call" trades. For "put" trades, use the `sell` method.
trade_id, deal = client.buy("EURUSD_otc", 1.0, 60)
# trade_id, deal = client.sell("EURUSD_otc", 1.0, 60)
print(f"Trade placed: {deal}")
result = client.check_win(trade_id)
print(f"Trade result: {result}")

# Disconnect
client.disconnect()
```

### Real-time Data Streaming

```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync

async def main():
    client = PocketOptionAsync(ssid="your_ssid_here")
    
    # Subscribe to 60-second candles
    subscription = await client.subscribe_symbol("EURUSD_otc", 60)
    
    # Process candles
    async for candle in subscription:
        print(f"Time: {candle['time']}, Close: {candle['close']}")
        
        # Break after 10 candles
        if candle['index'] >= 10:
            break
    
    await client.disconnect()

asyncio.run(main())
```

### Raw Handler API (Advanced)

```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

async def main():
    client = PocketOptionAsync(ssid="your_ssid_here")
    
    # Create raw handler with validator
    validator = Validator.contains("price")
    handler = await client.create_raw_handler(validator)
    
    # Send custom WebSocket message
    await handler.send_text('42["custom/request"]')
    
    # Wait for validated response
    response = await handler.wait_next()
    print(f"Received: {response}")
    
    await client.disconnect()

asyncio.run(main())
```

## Documentation

- **Python API**: [Full Python Documentation](https://chipadevteam.github.io/BinaryOptionsTools-v2/python.html)
- **Examples**: Browse [examples directory](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/tree/master/examples) for comprehensive code samples
- **Architecture**: See [DOCUMENTATION_SUMMARY.md](docs/DOCUMENTATION_SUMMARY.md) for technical details

## Development

### Project Structure

```
BinaryOptionsTools-v2/
├── crates/
│   ├── binary_options_tools/    # Main Rust library
│   ├── core/                    # Core WebSocket client
│   ├── core-pre/                # Low-level protocol handlers
│   └── macros/                  # Procedural macros
├── BinaryOptionsToolsV2/
│   ├── src/                     # Rust PyO3 bindings
│   └── BinaryOptionsToolsV2/    # Python wrapper layer
├── examples/
│   ├── python/                  # Python examples
│   └── javascript/              # Node.js examples (experimental)
└── docs/                        # Documentation
```

### Building the Rust Library

```bash
cd crates/binary_options_tools
cargo build --release
cargo test
```

### Building Python Bindings

```bash
cd BinaryOptionsToolsV2
maturin build --release
```

### Running Tests

```bash
# Rust tests
cargo test

# Python tests
cd BinaryOptionsToolsV2
pytest tests/
```

## Roadmap

### Planned Features

- [ ] Expert Options platform integration
- [ ] JavaScript/TypeScript native bindings
- [ ] WebAssembly support for browser usage
- [ ] Advanced order types (stop-loss, take-profit) - Only available for Forex accounts, not Quick Trading (QT) accounts
- [ ] Historical data export tools
- [ ] Strategy backtesting framework

### Platform Support

- [x] PocketOption Quick Trading
- [x] PocketOption Pending Orders (BETA)
- [ ] Expert Options
- [ ] IQ Option (planned)

## Contributing

Contributions are welcome! Please ensure:

1. Code follows Rust and Python best practices
2. All tests pass (`cargo test` and `pytest`)
3. New features include documentation and examples
4. Commit messages are clear and descriptive

## License

**Personal Use License** - Free for personal, educational, and non-commercial use.

**Commercial Use** - Requires explicit written permission from ChipaDevTeam. Contact us on [Discord](https://discord.gg/p7YyFqSmAz) for commercial licensing.

See the full [LICENSE](LICENSE) file for complete terms and conditions.

### Key Points

- ✅ **Free** for personal use, learning, and private trading
- ✅ **Open source** - modify and distribute for personal use
- ⚠️ **Commercial use requires permission** - Contact us first
- ⚠️ **No warranty** - Software provided "as is"
- ⚠️ **No liability** - Use at your own risk

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz) for help, discussions, and updates
- **Issues**: Report bugs or request features via [GitHub Issues](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)

## Disclaimer

**IMPORTANT**: This software is provided "AS IS" without any warranty. The authors and ChipaDevTeam are NOT responsible for:

- Any financial losses incurred from using this software
- Any trading decisions made using this software  
- Any bugs, errors, or issues in the software
- Any consequences of using this software for trading

**Risk Warning**: Binary options trading carries significant financial risk. This software is for educational and personal use only. You should:

- Never risk more than you can afford to lose
- Understand the risks involved in binary options trading
- Comply with all applicable laws and regulations in your jurisdiction
- Use this software at your own risk

By using this software, you acknowledge and accept these terms.
