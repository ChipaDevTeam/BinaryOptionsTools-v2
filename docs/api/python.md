---
sidebar_position: 2
---

# BinaryOptionsToolsV2 Python API Reference

Complete reference guide for all features and methods available in the BinaryOptionsToolsV2 Python library.

## Async Client

The `PocketOptionAsync` class provides asynchronous access to PocketOption's API.

```python
from binaryoptionstoolsv2 import PocketOptionAsync

client = await PocketOptionAsync("your_ssid")
await asyncio.sleep(2)  # Wait for initialization
```

### Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `__init__(ssid)` | Initialize with session ID | Self |
| `balance()` | Get account balance | `float` |
| `is_demo()` | Check if demo account | `bool` |
| `buy(asset, time, amount)` | Place call trade | `Deal` |
| `sell(asset, time, amount)` | Place put trade | `Deal` |
| `trade(asset, action, time, amount)` | Place trade with action | `Deal` |
| `result(id)` | Check trade result | `Deal` |
| `result_with_timeout(id, timeout)` | Check result with timeout | `Deal` |
| `get_opened_deals()` | Get open trades | `List[Deal]` |
| `get_closed_deals()` | Get closed trades | `List[Deal]` |
| `clear_closed_deals()` | Clear closed trades | `None` |
| `get_candles(asset, period, offset)` | Get historical candles | `List[Candle]` |
| `server_time()` | Get server timestamp | `int` |
| `subscribe(asset, duration)` | Subscribe to real-time data | `AsyncIterator[Candle]` |
| `unsubscribe(asset)` | Unsubscribe from asset | `None` |
| `reconnect()` | Reconnect to server | `None` |
| `shutdown()` | Shutdown client | `None` |

## Sync Client

The `PocketOption` class provides synchronous access to PocketOption's API.

```python
from binaryoptionstoolsv2 import PocketOption

client = PocketOption("your_ssid")
import time
time.sleep(2)  # Wait for initialization
```

### Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `__init__(ssid)` | Initialize with session ID | Self |
| `balance()` | Get account balance | `float` |
| `is_demo()` | Check if demo account | `bool` |
| `buy(asset, time, amount)` | Place call trade | `Deal` |
| `sell(asset, time, amount)` | Place put trade | `Deal` |
| `result(id)` | Check trade result | `Deal` |
| `get_opened_deals()` | Get open trades | `List[Deal]` |
| `get_closed_deals()` | Get closed trades | `List[Deal]` |
| `get_candles(asset, period, offset)` | Get historical candles | `List[Candle]` |
| `subscribe(asset, duration)` | Subscribe to real-time data | `Iterator[Candle]` |
| `unsubscribe(asset)` | Unsubscribe from asset | `None` |
| `reconnect()` | Reconnect to server | `None` |
| `shutdown()` | Shutdown client | `None` |

## Raw Handler

The `RawHandler` class provides low-level access to PocketOption's WebSocket messages.

```python
from binaryoptionstoolsv2 import RawHandler

handler = RawHandler("your_ssid")
```

### Methods

| Method | Description |
|--------|-------------|
| `connect()` | Connect to WebSocket |
| `send(message)` | Send raw message |
| `receive()` | Receive raw message |
| `close()` | Close connection |

## Validator

The `Validator` class validates session data and SSID tokens.

```python
from binaryoptionstoolsv2 import Validator

validator = Validator()
result = validator.validate_ssid("your_ssid")
```

### Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `validate_ssid(ssid)` | Validate SSID format | `bool` |
| `validate_credentials(email, password)` | Validate login credentials | `bool` |

## Configuration

The `Config` class manages library configuration.

```python
from binaryoptionstoolsv2 import Config

config = Config()
config.set("timeout", 30)
```

### Methods

| Method | Description |
|--------|-------------|
| `set(key, value)` | Set configuration value |
| `get(key)` | Get configuration value |
| `load_from_file(path)` | Load config from file |
| `save_to_file(path)` | Save config to file |

## Trading Examples

### Basic Trade

```python
import asyncio
from binaryoptionstoolsv2 import PocketOptionAsync

async def basic_trade():
    client = await PocketOptionAsync("your_ssid")
    await asyncio.sleep(2)
    
    # Check account type
    if not client.is_demo():
        print("⚠️ WARNING: Using REAL account!")
        return
    
    # Place a call trade
    trade = await client.buy("EURUSD_otc", 60, 1.0)
    print(f"Trade ID: {trade.id}")
    
    # Wait for result
    await asyncio.sleep(65)
    
    # Check result
    result = await client.result(trade.id)
    if result.profit > 0:
        print(f"✅ WIN! Profit: ${result.profit:.2f}")
    else:
        print(f"❌ LOSS! Loss: ${abs(result.profit):.2f}")
    
    await client.shutdown()

asyncio.run(basic_trade())
```

### Multiple Trades

```python
async def multiple_trades():
    client = await PocketOptionAsync("your_ssid")
    await asyncio.sleep(2)
    
    assets = ["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"]
    trades = []
    
    for asset in assets:
        trade = await client.buy(asset, 60, 1.0)
        trades.append(trade)
    
    await asyncio.sleep(65)
    
    total_profit = 0
    for trade in trades:
        result = await client.result(trade.id)
        total_profit += result.profit
        status = "WIN" if result.profit > 0 else "LOSS"
        print(f"{trade.asset}: {status} ${result.profit:.2f}")
    
    print(f"Total Profit: ${total_profit:.2f}")
    await client.shutdown()
```

### Candle Data

```python
async def get_candles():
    client = await PocketOptionAsync("your_ssid")
    await asyncio.sleep(2)
    
    # Get last 100 candles
    candles = await client.get_candles("EURUSD_otc", 60, 100)
    print(f"Retrieved {len(candles)} candles")
    
    for candle in candles[:5]:
        print(f"  Time: {candle.time}, O: {candle.open}, H: {candle.high}, L: {candle.low}, C: {candle.close}")
    
    await client.shutdown()
```

## Error Handling

```python
from binaryoptionstoolsv2 import PocketOptionAsync, PocketError

async def safe_trade():
    try:
        client = await PocketOptionAsync("your_ssid")
        await asyncio.sleep(2)
        
        balance = await client.balance()
        print(f"Balance: ${balance:.2f}")
        
    except PocketError as e:
        print(f"PocketOption Error: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")
    finally:
        if 'client' in locals():
            await client.shutdown()
```

## Risk Management

```python
class SafeTrader:
    def __init__(self, client, max_daily_loss=10.0, risk_per_trade=0.02):
        self.client = client
        self.max_daily_loss = max_daily_loss
        self.risk_per_trade = risk_per_trade
        self.daily_pnl = 0.0
    
    async def can_trade(self):
        return abs(self.daily_pnl) < self.max_daily_loss
    
    async def safe_amount(self):
        balance = await self.client.balance()
        return balance * self.risk_per_trade
    
    async def trade(self, asset, action, time, amount=None):
        if not await self.can_trade():
            raise Exception("Daily loss limit reached")
        
        if amount is None:
            amount = await self.safe_amount()
        
        if action == "call":
            trade = await self.client.buy(asset, time, amount)
        else:
            trade = await self.client.sell(asset, time, amount)
        
        await asyncio.sleep(time + 5)
        result = await self.client.result(trade.id)
        
        self.daily_pnl += result.profit
        return result
```