# BinaryOptionsToolsV2 Python API Reference

Complete reference guide for all features and methods available in the BinaryOptionsToolsV2 Python library.

## Table of Contents
- [Trading Operations](#trading-operations)
- [Account Management](#account-management)
- [Market Data](#market-data)
- [Real-time Subscriptions](#real-time-subscriptions)
- [Connection Management](#connection-management)
- [Advanced Features](#advanced-features)

---

## Trading Operations

| Feature | Async Code | Sync Code | Description |
|---------|-----------|-----------|-------------|
| **Buy/Call Order** | `await client.buy(asset, amount, time, check_win)` | `client.buy(asset, amount, time, check_win)` | Places a buy (call) order. Returns `(trade_id, trade_data)`. Set `check_win=True` to wait for result. |
| **Sell/Put Order** | `await client.sell(asset, amount, time, check_win)` | `client.sell(asset, amount, time, check_win)` | Places a sell (put) order. Returns `(trade_id, trade_data)`. Set `check_win=True` to wait for result. |
| **Check Trade Result** | `await client.check_win(trade_id)` | `client.check_win(trade_id)` | Checks if a trade won, lost, or drew. Returns dict with `result` ("win"/"loss"/"draw") and `profit`. |

### Trading Example
```python
# Async
import asyncio

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

trade_id, trade = await client.buy("EURUSD_otc", 1.0, 60, check_win=True)
print(f"Result: {trade['result']}, Profit: {trade['profit']}")

# Sync
import time

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

trade_id, trade = client.buy("EURUSD_otc", 1.0, 60, check_win=True)
print(f"Result: {trade['result']}, Profit: {trade['profit']}")
```

---

## Account Management

| Feature | Async Code | Sync Code | Description |
|---------|-----------|-----------|-------------|
| **Get Balance** | `await client.balance()` | `client.balance()` | Returns current account balance as float. |
| **Check Demo Account** | `client.is_demo()` | `client.is_demo()` | Returns `True` if using demo account, `False` for real account. |
| **Get Opened Deals** | `await client.opened_deals()` | `client.opened_deals()` | Returns list of all currently open trades with full details. |
| **Get Closed Deals** | `await client.closed_deals()` | `client.closed_deals()` | Returns list of all closed trades from memory. |
| **Clear Closed Deals** | `await client.clear_closed_deals()` | `client.clear_closed_deals()` | Removes all closed deals from memory. Returns nothing. |

### Account Management Example
```python
# Async
import asyncio

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

balance = await client.balance()
is_demo = client.is_demo()
open_trades = await client.opened_deals()
closed_trades = await client.closed_deals()

# Sync
import time

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

balance = client.balance()
is_demo = client.is_demo()
open_trades = client.opened_deals()
closed_trades = client.closed_deals()
```

---

## Market Data

| Feature | Async Code | Sync Code | Description |
|---------|-----------|-----------|-------------|
| **Get Historical Candles** | `await client.get_candles(asset, period, offset)` | `client.get_candles(asset, period, offset)` | Returns list of historical candles (OHLC) for the asset. Each candle has `time`, `open`, `high`, `low`, `close`. |
| **Get Candles (Advanced)** | `await client.get_candles_advanced(asset, period, offset, time)` | `client.get_candles_advanced(asset, period, offset, time)` | Returns historical candles starting from specific timestamp. More control over time range. |
| **Get Asset Payout** | `await client.payout(asset)` | `client.payout(asset)` | Returns payout percentage. Pass `None` for all assets dict, string for single asset int, or list for multiple assets list. |
| **Get History** | `await client.history(asset, period)` | `client.history(asset, period)` | Returns latest available historical data for asset starting from period. Same format as `get_candles`. |
| **Get Server Time** | `await client.get_server_time()` | `client.get_server_time()` | Returns current server time as UNIX timestamp (int). |

### Market Data Example
```python
# Async
import asyncio

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

candles = await client.get_candles("EURUSD_otc", 60, 100)
all_payouts = await client.payout()  # Dict of all assets
eurusd_payout = await client.payout("EURUSD_otc")  # Single int
multi_payouts = await client.payout(["EURUSD_otc", "GBPUSD"])  # List of ints
server_time = await client.get_server_time()

# Sync
import time

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

candles = client.get_candles("EURUSD_otc", 60, 100)
all_payouts = client.payout()
eurusd_payout = client.payout("EURUSD_otc")
server_time = client.get_server_time()
```

### Candle Data Structure
```python
{
    "time": "2025-01-01T12:00:00Z",  # ISO format timestamp
    "open": 1.0950,                   # Opening price
    "high": 1.0955,                   # Highest price
    "low": 1.0948,                    # Lowest price
    "close": 1.0952                   # Closing price
}
```

---

## Real-time Subscriptions

| Feature | Async Code | Sync Code | Description |
|---------|-----------|-----------|-------------|
| **Subscribe (Raw)** | `await client.subscribe_symbol(asset)` | `client.subscribe_symbol(asset)` | Returns iterator that yields real-time raw tick data as it arrives. Most granular data. |
| **Subscribe (Chunked)** | `await client.subscribe_symbol_chuncked(asset, chunk_size)` | `client.subscribe_symbol_chuncked(asset, chunk_size)` | Returns iterator that yields candles formed from specified number of raw ticks. Groups raw data into chunks. |
| **Subscribe (Timed)** | `await client.subscribe_symbol_timed(asset, timedelta)` | `client.subscribe_symbol_timed(asset, timedelta)` | Returns iterator that yields candles formed over specified time duration. Creates time-based candles. |
| **Subscribe (Time-Aligned)** | `await client.subscribe_symbol_time_aligned(asset, timedelta)` | `client.subscribe_symbol_time_aligned(asset, timedelta)` | Returns iterator that yields candles perfectly aligned to time intervals (e.g., exactly on minute boundaries). |
| **Unsubscribe** | `await client.unsubscribe(asset)` | `client.unsubscribe(asset)` | Unsubscribes from an asset's data stream by asset name. Cleans up resources. |

### Subscription Examples

#### Async Subscriptions
```python
import asyncio
from datetime import timedelta

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

# Raw tick data
subscription = await client.subscribe_symbol("EURUSD_otc")
async for tick in subscription:
    print(f"Raw tick: {tick}")

# Chunked candles (every 10 ticks)
subscription = await client.subscribe_symbol_chuncked("EURUSD_otc", 10)
async for candle in subscription:
    print(f"Chunk candle: {candle}")

# Timed candles (every 60 seconds)
subscription = await client.subscribe_symbol_timed("EURUSD_otc", timedelta(seconds=60))
async for candle in subscription:
    print(f"Timed candle: {candle}")

# Time-aligned candles (aligned to minute boundaries)
subscription = await client.subscribe_symbol_time_aligned("EURUSD_otc", timedelta(minutes=1))
async for candle in subscription:
    print(f"Aligned candle at {candle['time']}")

# Unsubscribe when done
await client.unsubscribe("EURUSD_otc")
```

#### Sync Subscriptions
```python
import time
from datetime import timedelta

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

# Raw tick data
subscription = client.subscribe_symbol("EURUSD_otc")
for tick in subscription:
    print(f"Raw tick: {tick}")
    if some_condition:
        break

# Time-aligned candles
subscription = client.subscribe_symbol_time_aligned("EURUSD_otc", timedelta(minutes=1))
for candle in subscription:
    print(f"Price: {candle['close']}")

# Unsubscribe
client.unsubscribe("EURUSD_otc")
```

---

## Connection Management

| Feature | Async Code | Sync Code | Description |
|---------|-----------|-----------|-------------|
| **Disconnect** | `await client.disconnect()` | `client.disconnect()` | Closes WebSocket connection while keeping configuration. Can reconnect later. |
| **Connect** | `await client.connect()` | `client.connect()` | Establishes connection after manual disconnect. Uses same config and credentials. |
| **Reconnect** | `await client.reconnect()` | `client.reconnect()` | Disconnects and immediately reconnects. Useful for resetting connection state. |

### Connection Management Example
```python
# Async
import asyncio

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

# ... use client ...
await client.disconnect()  # Close connection
# ... do other work ...
await client.connect()     # Reopen connection
await asyncio.sleep(2)     # Wait for reconnection to complete
await client.reconnect()   # Quick disconnect + connect
await asyncio.sleep(2)     # Wait for reconnection to complete

# Sync
import time

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

# ... use client ...
client.disconnect()
client.connect()
time.sleep(2)  # Wait for reconnection to complete
client.reconnect()
time.sleep(2)  # Wait for reconnection to complete
```

---

## Advanced Features

### Raw Handler API

The Raw Handler API provides low-level WebSocket access for custom protocol implementations and advanced use cases.

| Feature | Async Code | Sync Code | Description |
|---------|-----------|-----------|-------------|
| **Create Handler** | `await client.create_raw_handler(validator, keep_alive)` | `client.create_raw_handler(validator, keep_alive)` | Creates a raw handler with message validator. Returns `RawHandler` (async) or `RawHandlerSync`. |
| **Send Text** | `await handler.send_text(message)` | `handler.send_text(message)` | Sends a text WebSocket message without waiting for response. |
| **Send Binary** | `await handler.send_binary(data)` | `handler.send_binary(data)` | Sends binary WebSocket message without waiting for response. |
| **Send and Wait** | `await handler.send_and_wait(message)` | `handler.send_and_wait(message)` | Sends message and waits for first matching response based on validator. |
| **Wait Next** | `await handler.wait_next()` | `handler.wait_next()` | Waits for next message that matches handler's validator. |
| **Subscribe Stream** | `await handler.subscribe()` | `handler.subscribe()` | Returns iterator yielding all matching messages. For continuous monitoring. |
| **Get Handler ID** | `handler.id()` | `handler.id()` | Returns unique UUID string identifier for this handler. |
| **Close Handler** | `await handler.close()` | `handler.close()` | Closes handler and cleans up resources (automatic on scope exit). |

### Raw Handler Examples

#### Async Raw Handler
```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

# Create validator for filtering messages
validator = Validator.starts_with('42["signals"')
handler = await client.create_raw_handler(validator)

# Send custom message and wait for response
response = await handler.send_and_wait('42["signals/subscribe"]')
data = json.loads(response)
print(f"Signals data: {data}")

# Subscribe to continuous stream
async for message in await handler.subscribe():
    data = json.loads(message)
    print(f"Signal update: {data}")
    if data.get('stop'):
        break

# Get handler info
handler_id = handler.id()
print(f"Handler ID: {handler_id}")
```

#### Sync Raw Handler
```python
import time
from BinaryOptionsToolsV2 import PocketOption, Validator

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

# Create validator
validator = Validator.contains('"type":"price"')
handler = client.create_raw_handler(validator)

# Send and wait
response = handler.send_and_wait('42["price/subscribe"]')
data = json.loads(response)

# Stream messages
for message in handler.subscribe():
    data = json.loads(message)
    print(f"Price: {data['price']}")
    if data['price'] > threshold:
        break

handler.close()
```

### Validator API

Validators filter incoming WebSocket messages. Create complex filters using the Validator class.

| Validator Type | Code | Description |
|---------------|------|-------------|
| **Starts With** | `Validator.starts_with(prefix)` | Matches messages starting with prefix string. |
| **Contains** | `Validator.contains(substring)` | Matches messages containing substring anywhere. |
| **Regex** | `Validator.regex(pattern)` | Matches messages against regex pattern. |
| **All (AND)** | `Validator.all([validator1, validator2, ...])` | Matches only if ALL validators match (logical AND). |
| **Any (OR)** | `Validator.any([validator1, validator2, ...])` | Matches if ANY validator matches (logical OR). |

#### Validator Examples
```python
from BinaryOptionsToolsV2 import Validator

# Simple validators
validator1 = Validator.starts_with('42["signals"')
validator2 = Validator.contains('"type":"candle"')
validator3 = Validator.regex(r'\d+\.\d+')  # Match decimal numbers

# Complex validators (combining multiple)
# Match messages that start with signals AND contain price
complex1 = Validator.all([
    Validator.starts_with('42["signals"'),
    Validator.contains('"price"')
])

# Match messages about either candles OR ticks
complex2 = Validator.any([
    Validator.contains('"type":"candle"'),
    Validator.contains('"type":"tick"')
])

# Use with handler
handler = await client.create_raw_handler(complex1)
```

---

## Configuration

### Custom WebSocket URL

You can specify a custom WebSocket URL when initializing the client:

```python
# Async - with custom URL
client = PocketOptionAsync(
    ssid="your_ssid",
    url="wss://custom-server.com/websocket"
)

# Sync - with custom URL  
client = PocketOption(
    ssid="your_ssid",
    url="wss://custom-server.com/websocket"
)
```

**Use cases for custom URLs:**
- Testing with custom/mock servers
- Using proxy servers
- Connecting to alternative regional endpoints
- Development and debugging

### ⚠️ Config Class (Currently Not Functional)

**Important**: The `Config` class exists in the Python codebase but is **NOT currently used** by the library. The Rust backend does not accept configuration parameters, and all settings use internal defaults.

```python
# This code will NOT have any effect:
from BinaryOptionsToolsV2 import Config

config = Config.from_dict({"timeout_secs": 60})
client = PocketOptionAsync(ssid, config=config)  # config parameter is ignored
```

**Current Behavior:**
- ❌ Config parameter is accepted but ignored
- ❌ All configuration values use hardcoded defaults
- ✅ Only the `url` parameter actually works
- ✅ Built-in automatic reconnection with exponential backoff
- ✅ Built-in connection and operation timeouts
- ✅ Built-in WebSocket keepalive

**Planned Configuration Parameters** (not yet implemented):

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `max_allowed_loops` | int | 1000 | Maximum event loop iterations before timeout |
| `sleep_interval` | int | 100 | Sleep time between operations (milliseconds) |
| `reconnect_time` | int | 5 | Wait time before reconnection attempts (seconds) |
| `connection_initialization_timeout_secs` | int | 30 | Timeout for initial connection (seconds) |
| `timeout_secs` | int | 30 | General operation timeout (seconds) |
| `urls` | List[str] | Platform defaults | List of WebSocket URLs for fallback |

If you need custom configuration options, please:
- Open an issue on [GitHub](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- Contact us on [Discord](https://discord.gg/p7YyFqSmAz)

---

## Complete Usage Examples

### Basic Trading Bot (Async)
```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync

async def main():
    client = PocketOptionAsync(ssid="your_ssid")
    await asyncio.sleep(2)  # Wait for API to initialize
    
    # Check account
    balance = await client.balance()
    is_demo = client.is_demo()
    print(f"{'Demo' if is_demo else 'Real'} Account - Balance: ${balance}")
    
    # Get market data
    payout = await client.payout("EURUSD_otc")
    print(f"EURUSD_OTC Payout: {payout}%")
    
    # Place trade
    asset = "EURUSD_otc"
    amount = 1.0
    duration = 60
    
    trade_id, trade = await client.buy(asset, amount, duration, check_win=True)
    print(f"Trade Result: {trade['result']}")
    print(f"Profit: ${trade['profit']}")
    
    await client.disconnect()

asyncio.run(main())
```

### Basic Trading Bot (Sync)
```python
import time
from BinaryOptionsToolsV2 import PocketOption

client = PocketOption(ssid="your_ssid")
time.sleep(2)  # Wait for API to initialize

# Check account
balance = client.balance()
is_demo = client.is_demo()
print(f"Balance: ${balance}")

# Place trade
trade_id, trade = client.buy("EURUSD_otc", 1.0, 60, check_win=True)
print(f"Result: {trade['result']}, Profit: ${trade['profit']}")

client.disconnect()
```

### Real-time Data Monitoring (Async)
```python
import asyncio
from datetime import timedelta
from BinaryOptionsToolsV2 import PocketOptionAsync

async def monitor_price():
    client = PocketOptionAsync(ssid="your_ssid")
    await asyncio.sleep(2)  # Wait for API to initialize
    
    # Subscribe to 1-minute aligned candles
    subscription = await client.subscribe_symbol_time_aligned(
        "EURUSD_otc", 
        timedelta(minutes=1)
    )
    
    candle_count = 0
    async for candle in subscription:
        print(f"Time: {candle['time']}")
        print(f"OHLC: {candle['open']}/{candle['high']}/{candle['low']}/{candle['close']}")
        
        candle_count += 1
        if candle_count >= 10:
            break
    
    await client.unsubscribe("EURUSD_otc")
    await client.disconnect()

asyncio.run(monitor_price())
```

### Advanced Strategy with Raw Handler (Async)
```python
import asyncio
import json
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

async def advanced_strategy():
    client = PocketOptionAsync(ssid="your_ssid")
    await asyncio.sleep(2)  # Wait for API to initialize
    
    # Create custom message handler
    validator = Validator.all([
        Validator.contains('"type":"price"'),
        Validator.contains('"asset":"EURUSD_otc"')
    ])
    
    handler = await client.create_raw_handler(validator)
    
    # Monitor specific price updates
    count = 0
    async for message in await handler.subscribe():
        data = json.loads(message)
        price = data.get('price', 0)
        
        print(f"Current price: {price}")
        
        # Custom trading logic
        if price > 1.0950:
            trade_id, _ = await client.sell("EURUSD_otc", 1.0, 60)
            print(f"Sold at {price}, Trade ID: {trade_id}")
            break
        
        count += 1
        if count >= 100:
            break
    
    await client.disconnect()

asyncio.run(advanced_strategy())
```

### Multi-Asset Monitoring (Sync)
```python
import time
from datetime import timedelta
from BinaryOptionsToolsV2 import PocketOption

client = PocketOption(ssid="your_ssid")
time.sleep(2)  # Wait for API to initialize

assets = ["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"]

# Get payouts for all assets
payouts = client.payout(assets)
for asset, payout in zip(assets, payouts):
    print(f"{asset}: {payout}%")

# Monitor one asset
subscription = client.subscribe_symbol_time_aligned(
    "EURUSD_otc",
    timedelta(minutes=1)
)

for candle in subscription:
    print(f"EURUSD Close: {candle['close']}")
    # Your trading logic here
    if candle['close'] > threshold:
        break

client.unsubscribe("EURUSD_otc")
client.disconnect()
```

---

## Error Handling

```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync

async def safe_trading():
    try:
        client = PocketOptionAsync(ssid="your_ssid")
        await asyncio.sleep(2)  # Wait for API to initialize
        
        # Check connection
        balance = await client.balance()
        
        # Place trade with error handling
        trade_id, trade = await client.buy("EURUSD_otc", 1.0, 60, check_win=True)
        print(f"Trade completed: {trade['result']}")
        
    except ConnectionError as e:
        print(f"Connection error: {e}")
    except ValueError as e:
        print(f"Invalid parameters: {e}")
    except TimeoutError as e:
        print(f"Operation timed out: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")
    finally:
        try:
            await client.disconnect()
        except:
            pass

asyncio.run(safe_trading())
```

---

## Best Practices

### 1. Always Close Connections
```python
# Async - use try/finally
import asyncio

try:
    client = PocketOptionAsync(ssid)
    await asyncio.sleep(2)  # Wait for API to initialize
    # ... your code ...
finally:
    await client.disconnect()

# Sync - use try/finally
import time

try:
    client = PocketOption(ssid)
    time.sleep(2)  # Wait for API to initialize
    # ... your code ...
finally:
    client.disconnect()
```

### 2. Use Demo Account for Testing
```python
import time

client = PocketOption(ssid)
time.sleep(2)  # Wait for API to initialize

if not client.is_demo():
    print("WARNING: Using real account!")
    # Switch to demo or abort
```

### 3. Handle Connection Issues
```python
import asyncio

# The library has built-in automatic reconnection
# Just handle exceptions gracefully

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

try:
    balance = await client.balance()
except ConnectionError:
    print("Connection lost, library will auto-reconnect")
    await asyncio.sleep(5)  # Wait before retry
except TimeoutError:
    print("Operation timed out")
```

### 4. Unsubscribe When Done
```python
import asyncio

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

# Subscribe
subscription = await client.subscribe_symbol("EURUSD_otc")

# Use subscription...

# Always unsubscribe
await client.unsubscribe("EURUSD_otc")
```

### 5. Validate Assets Before Trading
```python
import asyncio

client = PocketOptionAsync(ssid)
await asyncio.sleep(2)  # Wait for API to initialize

# Check if asset has good payout
payout = await client.payout("EURUSD_otc")
if payout < 80:
    print(f"Low payout: {payout}%")
    # Skip or choose different asset
```

---

## Platform Support

| Platform | Status | Features |
|----------|--------|----------|
| **PocketOption** | ✅ Fully Supported | Quick Trading, Real/Demo accounts, All features |
| **Expert Options** | ❌ Not Yet | Planned for future release |
| **IQ Option** | ❌ Not Yet | Planned for future release |

---

## Getting Help

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report bugs](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- **Documentation**: [Full docs](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Examples**: [Code examples](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/tree/master/examples/python)

---

## Version Information

This documentation is for **BinaryOptionsToolsV2 v0.2.1+**

For changelog and version history, see the [GitHub Releases](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases).
