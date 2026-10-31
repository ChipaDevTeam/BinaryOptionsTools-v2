---
sidebar_position: 2
---

# BinaryOptionsToolsV2 Python API Reference

:::tip Prototype before you implement
Before writing a few hundred lines against this API, sketch the strategy in
**[ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=api-python&utm_campaign=BinaryOptionsToolsV2)** — AI drafts it in CHTL, the built-in backtester tells
you whether the edge is real, and only then is it worth building here properly.
[Try it free →](https://chipaeditor.com/?utm_source=docs&utm_medium=api-python&utm_campaign=BinaryOptionsToolsV2&utm_content=cta)
:::

Complete reference guide for all features and methods available in the BinaryOptionsToolsV2 Python library.

## Async Client

The `PocketOptionAsync` class provides asynchronous access to PocketOption's API.

```python
from BinaryOptionsToolsV2 import PocketOptionAsync

client = await PocketOptionAsync("your_ssid")
await asyncio.sleep(2)  # Wait for initialization
```

### Methods

| Method                                    | Description                                                          | Returns                   |
| ----------------------------------------- | -------------------------------------------------------------------- | ------------------------- |
| `__init__(ssid)`                          | Initialize with session ID                                           | Self                      |
| `balance()`                               | Get account balance                                                  | `float`                   |
| `is_demo()`                               | Check if demo account                                                | `bool`                    |
| `buy(asset, time, amount)`                | Place call trade                                                     | `Deal`                    |
| `sell(asset, time, amount)`               | Place put trade                                                      | `Deal`                    |
| `trade(asset, action, time, amount)`      | Place trade with action                                              | `Deal`                    |
| `result(id)`                              | Check trade result                                                   | `Deal`                    |
| `result_with_timeout(id, timeout)`        | Check result with timeout                                            | `Deal`                    |
| `get_opened_deals()`                      | Get open trades                                                      | `List[Deal]`              |
| `get_closed_deals()`                      | Get closed trades                                                    | `List[Deal]`              |
| `clear_closed_deals()`                    | Clear closed trades                                                  | `None`                    |
| `candles(asset, period, count=100)`       | Last `count` closed candles                                          | `List[Candle]`            |
| `candles_many(assets, period, count=100)` | Candles for many symbols in one call (not stream-limited)            | `Dict[str, List[Candle]]` |
| `get_candles(asset, period, offset)`      | Get historical candles (`offset` is a window in **seconds**)         | `List[Candle]`            |
| `server_time()`                           | Get server timestamp                                                 | `int`                     |
| `subscribe(asset, duration)`              | Subscribe to real-time data (the broker streams ~2 symbols at once)  | `AsyncIterator[Candle]`   |
| `unsubscribe(asset)`                      | Release a subscription locally (the broker has no unsubscribe event) | `None`                    |
| `reconnect()`                             | Reconnect to server                                                  | `None`                    |
| `shutdown()`                              | Shutdown client                                                      | `None`                    |
| `try_demo(token=None)`                    | Connect to a free guest demo session                                 | `PocketOptionAsync`       |

## Sync Client

The `PocketOption` class provides synchronous access to PocketOption's API.

```python
from BinaryOptionsToolsV2 import PocketOption

client = PocketOption("your_ssid")
import time
time.sleep(2)  # Wait for initialization
```

### Methods

| Method                                    | Description                                                          | Returns                   |
| ----------------------------------------- | -------------------------------------------------------------------- | ------------------------- |
| `__init__(ssid)`                          | Initialize with session ID                                           | Self                      |
| `balance()`                               | Get account balance                                                  | `float`                   |
| `is_demo()`                               | Check if demo account                                                | `bool`                    |
| `buy(asset, time, amount)`                | Place call trade                                                     | `Deal`                    |
| `sell(asset, time, amount)`               | Place put trade                                                      | `Deal`                    |
| `result(id)`                              | Check trade result                                                   | `Deal`                    |
| `get_opened_deals()`                      | Get open trades                                                      | `List[Deal]`              |
| `get_closed_deals()`                      | Get closed trades                                                    | `List[Deal]`              |
| `candles(asset, period, count=100)`       | Last `count` closed candles                                          | `List[Candle]`            |
| `candles_many(assets, period, count=100)` | Candles for many symbols in one call (not stream-limited)            | `Dict[str, List[Candle]]` |
| `get_candles(asset, period, offset)`      | Get historical candles (`offset` is a window in **seconds**)         | `List[Candle]`            |
| `subscribe(asset, duration)`              | Subscribe to real-time data (the broker streams ~2 symbols at once)  | `Iterator[Candle]`        |
| `unsubscribe(asset)`                      | Release a subscription locally (the broker has no unsubscribe event) | `None`                    |
| `reconnect()`                             | Reconnect to server                                                  | `None`                    |
| `shutdown()`                              | Shutdown client                                                      | `None`                    |
| `try_demo(token=None)`                    | Connect to a free guest demo session                                 | `PocketOption`            |

## Try Demo (guest) sessions

PocketOption's free "try demo" hands out a short-lived guest token instead of an
account session, so no credentials are needed:

```python
from BinaryOptionsToolsV2 import PocketOptionAsync

client = await PocketOptionAsync.try_demo()
assets = await client.active_assets()  # 100+ tradable symbols
```

`try_demo(token=None)` mints the guest token locally - the Python equivalent of
PocketOption's own "Try demo" client, which generates a 10-character id and caches
it in `localStorage['po-td-token']` - then connects to the demo server. Pass
`token=` to replay a token you already captured from the browser. The auth frame
looks like `42["auth",{"token":"...","balance":50000,"isFastHistory":true}]`.

```python
client = await PocketOptionAsync.try_demo("9PRxPzq6mQ")
```

Guest sessions are demo-only. The asset list is pushed to every socket, so receiving
it does not prove the token was accepted: the client waits for the server to confirm
the session (`successauth` or a balance frame) and raises `NotAuthorized` when that
never happens, instead of returning a client whose every request blocks until it
times out.

> **Guest tokens are only honoured when the broker knows them.** As of October 2026
> the demo servers ignore tokens generated locally, so `try_demo()` raises
> `NotAuthorized` unless the token came from the website. Use a session SSID with
> `PocketOption(...)` / `PocketOptionAsync(...)` in that case.

Guest sessions receive the same balance frame as account sessions. When the server
does not send one, the client reports the balance the session was opened with
(`50000`). A replayed `token` does not carry a balance - sending one would reset the
guest balance - so the auth frame omits it and the balance is reported as unknown if
the server never sends one.

## Raw Handler

The `RawHandler` class provides low-level access to PocketOption's WebSocket messages.

```python
from BinaryOptionsToolsV2 import RawHandler

handler = RawHandler("your_ssid")
```

### Methods

| Method          | Description          |
| --------------- | -------------------- |
| `connect()`     | Connect to WebSocket |
| `send(message)` | Send raw message     |
| `receive()`     | Receive raw message  |
| `close()`       | Close connection     |

## Validator

The `Validator` class validates session data and SSID tokens.

```python
from BinaryOptionsToolsV2 import Validator

validator = Validator()
result = validator.validate_ssid("your_ssid")
```

### Methods

| Method                                  | Description                | Returns |
| --------------------------------------- | -------------------------- | ------- |
| `validate_ssid(ssid)`                   | Validate SSID format       | `bool`  |
| `validate_credentials(email, password)` | Validate login credentials | `bool`  |

## Configuration

The `Config` class manages library configuration.

```python
from BinaryOptionsToolsV2 import Config

config = Config()
config.set("timeout", 30)
```

### Methods

| Method                 | Description             |
| ---------------------- | ----------------------- |
| `set(key, value)`      | Set configuration value |
| `get(key)`             | Get configuration value |
| `load_from_file(path)` | Load config from file   |
| `save_to_file(path)`   | Save config to file     |

## Trading Examples

### Basic Trade

```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync

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
    # The constructor waits for the auth handshake; no sleep needed.
    client = PocketOptionAsync("your_ssid")

    # Last 100 closed M1 candles
    candles = await client.candles("EURUSD_otc", 60, count=100)
    print(f"Retrieved {len(candles)} candles")

    for candle in candles[:5]:
        print(f"  Time: {candle['time']}, O: {candle['open']}, H: {candle['high']}, "
              f"L: {candle['low']}, C: {candle['close']}")

    client.shutdown()
```

Many symbols at once - polling snapshots is not limited by how many symbols the
broker will stream:

```python
feed = await client.candles_many(["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"], 60, count=2)
for symbol, candles in feed.items():
    print(symbol, candles[-1]["close"])
```

## Error Handling

```python
from BinaryOptionsToolsV2 import PocketOptionAsync, PocketError

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
