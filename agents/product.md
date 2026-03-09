# Product Context: BinaryOptionsTools-v2

## Description

A high-performance, cross-platform package for automating binary options trading. It is built with a Rust core for maximum speed and memory safety, providing high-level bindings for Python and other languages to ensure ease of use.

## Primary Users

- **Trading Bot Developers**: Individuals building automated trading systems.
- **Quantitative Traders**: Users requiring high-performance data streaming and execution for strategies.
- **Retail Traders**: Users looking for reliable tools to interface with binary options platforms programmatically.

## Main Goal

To bridge the gap between low-level performance and high-level usability, providing a robust, type-safe, and scalable framework for real-time market data streaming and instant trade execution on binary options platforms (starting with PocketOption).

## Key Features

- **High-Performance Rust Core**: Leveraging Rust for concurrency and memory safety.
- **Cross-Platform Bindings**: Seamless integration with Python (PyO3) and multiple other languages via UniFFI (Kotlin, Swift, Go, Ruby, C#).
- **Real-Time Data Streaming**: Native WebSocket support for live OHLC candles and market updates.
- **Instant Trade Execution**: Fast placement and monitoring of trades with configurable timeouts.
- **Historical Data Support**: Fetching OHLC data for backtesting and analysis.
- **Robust Connectivity**: Automatic reconnection, keep-alive monitoring, and server time synchronization.
- **Extensible Architecture**: Raw Handler API for custom protocols and built-in message validators.
