# BinaryOptionsTools V2
> **✨ Build with [Chipa Editor](https://chipaeditor.com/?utm_source=github&utm_medium=readme&utm_campaign=BinaryOptionsToolsV2&utm_term=support&utm_content=header) — the AI-powered strategy editor for Traders. Try it free!**

[![Discord](https://img.shields.io/discord/1261483112991555665?label=Discord&logo=discord&color=7289da)](https://discord.com/invite/p7YyFqSmAz)
[![Python Version](https://img.shields.io/badge/python-3.9%20|%203.10%20|%203.11%20|%203.12-blue)](https://www.python.org/)
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
| **PocketOption (Six)**   | [Join via Six's Affiliate Link](https://u3.shortink.io/smart/IqeAmBtFTrEWbh)   |
| **PocketOption (Chipa)** | [Join via Chipa's Affiliate Link](https://u3.shortink.io/smart/SDIaxbeamcYYqB) |

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
  - [Async API](#async-api-recommended)
  - [Bot Framework](#bot-framework--strategy-high-level)
  - [Data Streaming](#real-time-data-streaming)
- [Advanced Usage](#advanced-usage)
- [Examples](#examples)
- [Roadmap](#roadmap)
- [Legal & Disclaimer](#legal-and-disclaimer)
- [Known Bugs](#known-bugs)

---

## Known Bugs

**Automatic email & password SSID fetching:**
- 2FA may not be supported

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

- **PocketOption** (Full Support: Quick Trading, Pending Orders, Assets, History)
- **ExpertOption** (Alpha/Beta: Account Info, Keep-Alive, WebSocket Core)
- **IQ Option** (On Roadmap)

---

## Features

### Trading and Account

- **Execution**: Place Buy/Sell orders instantly.
- **Monitoring**: Check trade results (Win/Loss) with configurable timeouts.
- **Balances**: Real-time account balance retrieval.
- **Portfolio**: Access active positions and closed deal history.

### Market Data & Backtesting

- **Live Stream**: Subscribe to real-time candles and price ticks.
- **Historical / UTC Candles**: Fetch and compile custom or standard candles directly from 1-second ticks aligned strictly to UTC boundaries, ensuring no server-side gaps or overlaps (merges).
- **Virtual Market**: Built-in simulator for backtesting strategies without financial risk.
- **Server Sync**: Precision timing via NTP-like synchronization.

### Bot Framework (New)

- **Event-Driven**: Hooks for `on_start` and `on_candle` with JSON candle data.
- **Contextual API**: Write once, run on any platform (PocketOption, ExpertOption, or Virtual).
- **Strategy Trait**: Easily implement and swap trading algorithms.
- **Virtual Market**: Built-in simulator for backtesting strategies without financial risk.

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

#### Option A: Prebuilt Wheels (Recommended)

Install directly from our GitHub releases. Supports **Python 3.9 - 3.12**.

**Windows**

```bash
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/v0.2.12/binaryoptionstoolsv2-0.2.12-cp39-abi3-win_amd64.whl"
```

**Linux**

```bash
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/v0.2.12/binaryoptionstoolsv2-0.2.12-cp39-abi3-manylinux_2_28_x86_64.whl"
```

**macOS (Apple Silicon)**

```bash
pip install "https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/download/v0.2.12/binaryoptionstoolsv2-0.2.12-cp39-abi3-macosx_10_12_x86_64.macosx_11_0_arm64.macosx_10_12_universal2.whl"
```

#### Option B: Build from Source

Requires `rustc`, `cargo`, and `maturin`.

```bash
git clone https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git
cd BinaryOptionsTools-v2/python
pip install maturin
maturin develop --release
```

#### Option C: Build from Source Automatically

Requires `rustc`, `cargo`, and `maturin`.

```bash
pip install git+https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git#subdirectory=python
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

```python
import asyncio
import os
from BinaryOptionsToolsV2 import PocketOptionAsync

async def main():
    ssid = os.getenv("POCKET_OPTION_SSID")
    async with PocketOptionAsync(ssid=ssid) as client:
        balance = await client.balance()
        print(f"Balance: ${balance}")

        trade_id, deal = await client.buy("EURUSD_otc", 1.0, 60)
        print(f"Outcome: {await client.check_win(trade_id)}")

if __name__ == "__main__":
    asyncio.run(main())
```

### Bot Framework & Strategy (High-Level)

Implement the `Strategy` trait (Rust) or inherit from `PyStrategy` (Python) for structured bot development.

```python
import asyncio
import json
import os

from BinaryOptionsToolsV2 import PyBot, PyStrategy, RawPocketOption


class MyStrategy(PyStrategy):
    def on_start(self, ctx):
        print("Strategy started!")

    def on_candle(self, ctx, asset, candle_json):
        candle = json.loads(candle_json)
        if candle["close"] > candle["open"]:
            asyncio.create_task(ctx.buy(asset, 1.0, 60))


async def main():
    ssid = os.getenv("POCKET_OPTION_SSID")
    client = await RawPocketOption.create(ssid)

    strategy = MyStrategy()
    bot = PyBot(client, strategy)
    bot.add_asset("EURUSD_otc", 60)  # Monitor 60s candles

    await bot.run()

if __name__ == "__main__":
    asyncio.run(main())
```

### Real-time Data Streaming

```python
async with PocketOptionAsync(ssid="...") as client:
    async for candle in await client.subscribe_symbol("EURUSD_otc"):
        print(f"Price: {candle['close']}")
```

---

## Advanced Usage

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

> **Note on Authentication**: Authentication is handled via the `SSID` cookie. See our [Tutorials Directory](docs/tutorials/) for instructions on how to extract this from your browser.

---

## Examples

The [`examples/`](examples/) directory contains ready-to-run scripts for both async and sync APIs.

### Python Async

| Example                                                                | Description                     |
| ---------------------------------------------------------------------- | ------------------------------- |
| [`trade.py`](examples/python/async/trade.py)                           | Basic buy/sell with `check_win` |
| [`get_balance.py`](examples/python/async/get_balance.py)               | Account balance retrieval       |
| [`get_candles.py`](examples/python/async/get_candles.py)               | Historical candle data          |
| [`subscribe_symbol.py`](examples/python/async/subscribe_symbol.py)     | Real-time candle subscription   |
| [`strategy_example.py`](examples/python/async/strategy_example.py)     | PyBot/PyStrategy framework      |
| [`comprehensive_demo.py`](examples/python/async/comprehensive_demo.py) | Full API walkthrough            |
| [`raw_send.py`](examples/python/async/raw_send.py)                     | Raw WebSocket messages          |
| [`create_raw_order.py`](examples/python/async/create_raw_order.py)     | Raw order with validator        |
| [`validator.py`](examples/python/async/validator.py)                   | Validator usage examples        |

### Python Sync

A parallel set of examples using the synchronous `PocketOption` client is available in [`examples/python/sync/`](examples/python/sync/).

### Other Languages

UniFFI-generated examples for Go, Kotlin, Swift, Ruby, C#, and Rust are available in their respective subdirectories under [`examples/`](examples/).

---

## Roadmap

- [x] **PocketOption**: Quick Trading & Pending Orders
- [x] **ExpertOption**: Core Implementation (Alpha/Beta)
- [x] **Framework**: Bot & Strategy System
- [x] **Backtesting**: Virtual Market Simulator
- [ ] **Platform**: IQ Option Integration
- [x] **Core**: Multi-language support via UniFFI (Kotlin, Swift, Go, C#)
- [ ] **Core**: JavaScript/TypeScript Bindings
- [ ] **Core**: WebAssembly (WASM) Support
- [ ] **Tools**: Advanced Strategy Optimizer

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

[Documentation](https://chipadevteam.github.io/BinaryOptionsTools-v2/) | [API Reference](https://chipadevteam.github.io/BinaryOptionsTools-v2/api/reference.md) | [Discord Community](https://discord.com/invite/p7YyFqSmAz) | [Agents & AI](agents/AGENTS.md)
