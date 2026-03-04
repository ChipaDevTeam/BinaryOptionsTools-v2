# BinaryOptionsTools V2

[![Discord](https://img.shields.io/discord/1261483112991555665?label=Discord&logo=discord&color=7289da)](https://discord.com/invite/p7YyFqSmAz)
[![Python Version](https://img.shields.io/badge/python-3.8%20|%203.9%20|%203.10%20|%203.11%20|%203.12%20|%203.13%20|%203.14%20|%203.15-blue)](https://www.python.org/)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Personal-green)](LICENSE)

**A high-performance, cross-platform package for automating binary options trading.**
Built with **Rust** for speed and memory safety, featuring **Python** bindings for ease of use.

---

## Support the Development

This project is maintained by the **ChipaDevTeam**. Your support helps keep the updates coming.

| Support Channel          | Link                                                                           |
| :----------------------- | :----------------------------------------------------------------------------- |
| **PayPal**               | [Support ChipaDevTeam](https://www.paypal.me/ChipaCL)                          |
| **PocketOption (Six)**   | [Join via Six's Affiliate Link](https://poaffiliate.onelink.me/t5P7/9y34jkp3)  |
| **PocketOption (Chipa)** | [Join via Chipa's Affiliate Link](https://u3.shortink.io/smart/SDIaxbeamcYYqB) |

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
  - [Async API](#async-api-recommended)
  - [Sync API](#sync-api)
  - [Data Streaming](#real-time-data-streaming)
- [Advanced Usage](#advanced-usage)
- [Roadmap](#roadmap)
- [Legal & Disclaimer](#legal-and-disclaimer)

---

## Overview

**BinaryOptionsTools v2** is a complete rewrite of the original library. It bridges the gap between low-level performance and high-level usability.

### Key Highlights

- **Rust Core**: Maximum performance, concurrency, and memory safety.
- **Python Bindings**: Seamless integration with the Python ecosystem via PyO3.
- **WebSocket Native**: Real-time market data streaming and instant trade execution.
- **Robust Connectivity**: Automatic reconnection, keep-alive monitoring, and robust error handling.
- **Type Safety**: Strong typing across both Rust and Python interfaces.

### Supported Platforms

- **PocketOption** (Quick Trading Mode & Pending Orders BETA)
  - _Real & Demo Accounts Supported_

---

## Features

### Trading and Account

- **Execution**: Place Buy/Sell orders instantly.
- **Monitoring**: Check trade results (Win/Loss) with configurable timeouts.
- **Balances**: Real-time account balance retrieval.
- **Portfolio**: Access active positions and closed deal history.

### Market Data

- **Live Stream**: Subscribe to real-time candles (tick, 5s, 15s, 30s, 60s, 300s).
- **Historical**: Fetch OHLC data (`get_candles`) for backtesting.
- **Payouts**: Retrieve current payout percentages for assets.
- **Sync**: Server time synchronization for precision timing.

### Framework Utilities

- **Raw Handler API**: Low-level WebSocket access for custom protocols.
- **Validators**: Built-in message filtering system.
- **Asset Logic**: Automatic verification of trading pairs and OTC availability.

---

## Architecture

The system uses a layered architecture to ensure stability and speed.

```mermaid
graph TD
    User[User Application <br/> Python/Rust/JS] --> Bindings[Language Bindings <br/> PyO3 Async/Sync Wrappers]
    Bindings --> Core[Rust Core Library]

    subgraph Rust Core
    Core --> WS[WebSocket Client <br/> Tungstenite]
    Core --> Mgr[Connection Manager]
    Core --> Router[Message Router & Validators]
    end

    WS <--> API[PocketOption WebSocket API]
```

---

## Installation

### Python

#### Option A: PyPI (Recommended)

The easiest way to install the library is via PyPI:

```bash
pip install binaryoptionstoolsv2
```

#### Option B: Prebuilt Wheels

You can also install directly from our GitHub releases. Supports **Python 3.8 - 3.15**.

**Windows**

```bash
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/v0.2.8/binaryoptionstoolsv2-0.2.8-cp39-abi3-win_amd64.whl"
```

**Linux**

```bash
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/v0.2.8/binaryoptionstoolsv2-0.2.8-cp39-abi3-manylinux_2_28_x86_64.whl"
```

**macOS (Apple Silicon)**

```bash
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/v0.2.8/binaryoptionstoolsv2-0.2.8-cp39-abi3-macosx_10_12_x86_64.macosx_11_0_arm64.macosx_10_12_universal2.whl"
```

#### Option C: Build from Source

Requires `rustc`, `cargo`, and `maturin`.

```bash
git clone https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git
cd BinaryOptionsTools-v2/BinaryOptionsToolsV2
pip install maturin
maturin develop --release
# bleeding edge release(s)
# pip install git+https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git#subdirectory=BinaryOptionsToolsV2
```

### Rust

Add this to your `Cargo.toml`:

```toml
[dependencies]
binary_options_tools = { path = "crates/binary_options_tools" }
```

---

## Quick Start

### Async API (Recommended)

Best for building trading bots that need to handle streams and trades simultaneously.

```python
import asyncio
import os
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync

async def main():
    # 1. Get SSID (Session ID)
    ssid = os.getenv("POCKET_OPTION_SSID")

    # 2. Initialize with Context Manager
    async with PocketOptionAsync(ssid=ssid) as client:
        # Get Balance
        balance = await client.balance()
        print(f"Current Balance: ${balance}")

        # Place Trade: Asset, Amount, Duration (seconds)
        trade_id, deal = await client.buy("EURUSD_otc", 1.0, 60)
        print(f"Trade Placed: {deal}")

        # Wait for Result (blocks until trade is closed)
        result = await client.check_win(trade_id)
        print(f"Outcome: {result['result']} | Profit: {result['profit']}")

if __name__ == "__main__":
    asyncio.run(main())
```

### Sync API

Best for simple scripts or data fetching.

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption
import os

# Initialize with Context Manager
with PocketOption(ssid=os.getenv("POCKET_OPTION_SSID")) as client:
    print(f"Balance: ${client.balance()}")
    
    # Place Trade
    trade_id, _ = client.buy("EURUSD_otc", 1.0, 60)
    
    # Check Result
    print(f"Result: {client.check_win(trade_id)['result']}")
```

### Real-time Data Streaming

```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import asyncio

async def main():
    async with PocketOptionAsync(ssid="...") as client:
        # Subscribe to real-time price updates
        subscription = await client.subscribe_symbol("EURUSD_otc")

        print("Streaming data...")
        async for candle in subscription:
            print(f"Time: {candle['time']} | Close: {candle['close']}")

if __name__ == "__main__":
    asyncio.run(main())
```

---

## Examples & Tutorials

For more detailed usage and advanced patterns, explore our examples:

- **Python Examples**:
  - [Asynchronous Examples](docs/examples/python/async/) - Advanced bots, chunked subscriptions, and more.
  - [Synchronous Examples](docs/examples/python/sync/) - Simple scripts and data gathering.
- **Rust Examples**:
  - [Core Library Examples](docs/examples/rust/) - High-performance implementations.
  - [Advanced Module Usage](crates/binary_options_tools/examples/) - Pending orders and complex logic.
- **SSID Tutorial**: See the [Tutorials Directory](tutorials/) for instructions on how to extract your session ID (SSID) from the browser.

---

## Advanced Usage

### Logging and Tracing

Enable detailed logging for debugging:

```python
from BinaryOptionsToolsV2.tracing import start_logs

# Start logging to terminal and file
start_logs(path="logs/", level="INFO", terminal=True)
```

### Raw Handler API

For complex implementations, you can access the **Raw Handler API**. This allows you to construct custom WebSocket messages and filter responses.

```python
from BinaryOptionsToolsV2.validator import Validator

# Create a validator to filter messages containing "balance"
validator = Validator.contains("balance")
handler = await client.create_raw_handler(validator)

# Send raw JSON request
await handler.send_text('42["getBalance"]')

# Listen to the filtered stream
async for message in await handler.subscribe():
    print(f"Raw Update: {message}")
```

> **Note on Authentication**: Authentication is handled via the `SSID` cookie. See our [Tutorials Directory](tutorials/) for instructions on how to extract this from your browser.

---

## Roadmap

- [x] **PocketOption**: Quick Trading
- [x] **PocketOption**: Pending Orders (BETA)
- [ ] **Platform**: Expert Options Integration
- [ ] **Platform**: IQ Option Integration
- [ ] **Core**: JavaScript/TypeScript Bindings
- [ ] **Core**: WebAssembly (WASM) Support
- [ ] **Tools**: Historical Data Export & Backtesting Framework

---

## Contributing

We welcome contributions!

1. Fork the repo.
2. Ensure tests pass (`cargo test` & `pytest`).
3. Submit a Pull Request with clear descriptions.

---

## Legal and Disclaimer

### License

- **Personal Use**: Free for personal, educational, and non-commercial use.
- **Commercial Use**: Requires explicit written permission. Contact us on Discord.
- See [LICENSE](LICENSE) for details.

### Risk Warning

**This software is provided "AS IS" without warranty of any kind.**

- Binary options trading involves high risk and may result in the loss of capital.
- The authors and ChipaDevTeam are **NOT** responsible for any financial losses, trading errors, or software bugs.
- Use this software entirely at your own risk.

---

[Documentation](https://chipadevteam.github.io/BinaryOptionsTools-v2/) | [API Reference](https://chipadevteam.github.io/BinaryOptionsTools-v2/api/reference.md) | [Discord Community](https://discord.com/invite/p7YyFqSmAz)
