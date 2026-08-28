# CloseOption Examples

This directory contains examples for using the CloseOption trading platform integration.

## Prerequisites

To run these examples, you need:
1. A valid CloseOption session ID (SSID) in the format: `token|sid|demo|public_code|hidden_code`
2. Python 3.9+ with the BinaryOptionsToolsV2 package installed

## Setting Up

Export your SSID as an environment variable:

```bash
# Linux/Mac
export CLOSEOPTION_SSID="your_token|your_sid|true|your_public_code|your_hidden_code"

# Windows (Command Prompt)
set CLOSEOPTION_SSID=your_token|your_sid|true|your_public_code|your_hidden_code

# Windows (PowerShell)
$env:CLOSEOPTION_SSID="your_token|your_sid|true|your_public_code|your_hidden_code"
```

## Examples

### Async Examples

- **`closeoption_basic.py`** - Basic usage demonstrating connection and simple operations
- **`closeoption_advanced.py`** - Advanced features including asset management, subscriptions, and error handling
- **`closeoption_trading.py`** - Trading operations (buy/sell orders, trade history)

### Sync Examples

- **`closeoption_basic.py`** - Synchronous basic usage
- **`closeoption_advanced.py`** - Synchronous advanced features
- **`closeoption_trading.py`** - Synchronous trading operations

## Running Examples

```bash
# Run async basic example
python examples/async/closeoption_basic.py

# Run sync advanced example
python examples/sync/closeoption_advanced.py

# Run trading example (requires valid credentials)
python examples/async/closeoption_trading.py
```

## API Methods

The CloseOption client supports the following methods:

### Account Management
- `balance()` - Get current account balance
- `get_server_time()` - Get server time
- `reconnect()` - Reconnect to the server

### Market Data
- `active_assets()` - Get list of active assets
- `payout(asset)` - Get payout percentage for an asset
- `get_candles(asset, period, count)` - Get historical candles
- `get_candles_live(asset, period)` - Get live candle updates

### Trading
- `buy(asset, amount, duration)` - Place a BUY (CALL) order
- `sell(asset, amount, duration)` - Place a SELL (PUT) order
- `check_win(order_id)` - Check trade result
- `history(limit)` - Get trade history
- `opened_deals()` - Get opened deals
- `closed_deals()` - Get closed deals

### Subscriptions
- `subscribe_symbol(symbol)` - Subscribe to price updates for a symbol
- `subscribe_raw()` - Subscribe to all raw messages
- `send_raw(message)` - Send a raw message
- `raw_handler()` - Get raw handler for advanced operations

## SSID Format

The session ID (SSID) format is:
```
token|sid|demo|public_code|hidden_code
```

Where:
- `token` - Your authentication token
- `sid` - Session ID from Socket.IO handshake
- `demo` - `true` for demo account, `false` for real account
- `public_code` - Public asset code from your session
- `hidden_code` - Hidden asset code from your session

These values can be extracted from your browser's network requests when logged into CloseOption.

## Notes

- All examples support both demo and real accounts
- Trading operations require valid credentials and sufficient balance
- Connection timeout is 30 seconds for all operations
- The client automatically handles keep-alive pings
