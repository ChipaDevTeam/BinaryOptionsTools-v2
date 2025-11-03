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

## Features

### Trading Operations
- **Trade Execution**: Place buy/sell orders on any available asset
- **Trade Monitoring**: Check trade results with configurable timeouts
- **Balance Management**: Real-time account balance retrieval
- **Open/Closed Deals**: Access detailed trade history and active positions

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

#### Using pip (Prebuilt Wheels):
```bash
# Windows
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/blob/master/wheels/BinaryOptionsToolsV2-0.1.8-cp38-abi3-win_amd64.whl?raw=true"

# Linux
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/blob/master/wheels/BinaryOptionsToolsV2-0.2.0-cp38-abi3-manylinux_2_34_x86_64.whl?raw=true"
```

**Requirements**: Python 3.8 or higher

#### Building from Source:
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

async def main():
    # Initialize client with SSID
    client = PocketOptionAsync(ssid="your_ssid_here")
    
    # Get account balance
    balance = await client.balance()
    print(f"Balance: ${balance}")
    
    # Place a trade
    asset = "EURUSD_otc"
    amount = 1.0  # $1
    action = "call"  # or "put"
    duration = 60  # 60 seconds
    
    order_id = await client.trade(asset, action, amount, duration)
    print(f"Order placed: {order_id}")
    
    # Check if trade won
    result = await client.check_win(order_id)
    print(f"Trade result: {result}")
    
    # Disconnect
    await client.disconnect()

asyncio.run(main())
```

### Python - Sync API

```python
from BinaryOptionsToolsV2 import PocketOption

# Initialize client
client = PocketOption(ssid="your_ssid_here")

# Place trade (blocking)
order_id = client.trade("EURUSD_otc", "call", 1.0, 60)
result = client.check_win(order_id)
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
- [ ] Pending order support
- [ ] Expert Options platform integration
- [ ] JavaScript/TypeScript native bindings
- [ ] WebAssembly support for browser usage
- [ ] Advanced order types (stop-loss, take-profit)
- [ ] Historical data export tools
- [ ] Strategy backtesting framework

### Platform Support
- [x] PocketOption (Quick Trading)
- [ ] PocketOption (Pending Orders)
- [ ] Expert Options
- [ ] IQ Option (planned)

## Contributing

Contributions are welcome! Please ensure:
1. Code follows Rust and Python best practices
2. All tests pass (`cargo test` and `pytest`)
3. New features include documentation and examples
4. Commit messages are clear and descriptive

## License

See [LICENSE](LICENSE) file for details.

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz) for help, discussions, and updates
- **Issues**: Report bugs or request features via [GitHub Issues](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)

## Disclaimer

This library is for educational purposes. Binary options trading carries significant risk. Always trade responsibly and never risk more than you can afford to lose.
