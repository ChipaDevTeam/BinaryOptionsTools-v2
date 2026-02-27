# Rust Examples for BinaryOptionsTools

This directory contains example Rust programs demonstrating how to use the BinaryOptionsTools library.

## Prerequisites

1. Rust and Cargo installed ([Install Rust](https://rustup.rs/))
2. Add `binary_options_tools` to your `Cargo.toml`:

```toml
[dependencies]
binary_options_tools = "0.1"
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
```

## Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value and replace `"your-session-id"` in the examples

## Running the Examples

Each example can be run using:

```bash
cargo run --example <example_name>
```

For example:

```bash
cargo run --example balance
cargo run --example buy
cargo run --example subscribe_symbol
```

Or compile and run them directly:

```bash
rustc basic.rs && ./basic
```

## Available Examples

### `basic.rs`

Basic example showing:

- Client initialization
- Getting account balance
- Getting server time
- Checking if account is demo

**Run:**

```bash
cargo run --example basic
```

### `balance.rs`

Simple example showing how to get your account balance.

**Run:**

```bash
cargo run --example balance
```

### `buy.rs`

Example demonstrating:

- Placing a buy trade
- Checking balance before and after
- Calculating profit/loss

**Run:**

```bash
cargo run --example buy
```

### `sell.rs`

Example demonstrating:

- Placing a sell trade
- Checking balance before and after
- Calculating profit/loss

**Run:**

```bash
cargo run --example sell
```

### `check_win.rs`

Example showing:

- Placing trades
- Checking trade results manually
- Using automatic result checking with timeout

**Run:**

```bash
cargo run --example check_win
```

### `subscribe_symbol.rs`

Example demonstrating:

- Subscribing to real-time candle data
- Processing candle streams
- Displaying OHLC (Open, High, Low, Close) data

**Run:**

```bash
cargo run --example subscribe_symbol
```

## Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```rust
let client = PocketOption::new("your-session-id").await?;
tokio::time::sleep(Duration::from_secs(5)).await;  // Critical!
```

### Error Handling

All examples use proper error handling with `Result<(), Box<dyn std::error::Error>>`. Make sure to handle errors appropriately in production code.

### Async Runtime

All examples use the Tokio async runtime with the `#[tokio::main]` macro. Make sure your `Cargo.toml` includes:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

## Common Assets

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## Additional Resources

- **Crate Documentation**: [https://docs.rs/binary_options_tools](https://docs.rs/binary_options_tools)
- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. These examples are provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
