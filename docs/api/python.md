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

Reference for the Python client. Every signature below is taken from
`python/BinaryOptionsToolsV2/pocketoption/`; if a method is missing here it is not
part of the public surface.

## Two clients, one surface

| Class               | Module                 | Blocking |
| ------------------- | ---------------------- | -------- |
| `PocketOptionAsync` | `BinaryOptionsToolsV2` | No       |
| `PocketOption`      | `BinaryOptionsToolsV2` | Yes      |

Both expose the same families of methods; the sync client returns iterators where
the async client returns async generators or coroutines. Everything below is shown
against `PocketOptionAsync` unless noted.

## Constructing a client

The constructor is **not** awaitable — it starts the connection in the background.
Use the client as an async context manager so the session is closed for you:

```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync

async def main():
    async with PocketOptionAsync("your_ssid") as client:
        print(await client.balance())

asyncio.run(main())
```

`PocketOptionAsync(ssid, url=None, config=None)`; `config` accepts a `Config`, a
dict or a JSON string. Without a context manager, call `await client.shutdown()`
when done. Prefer `client.wait_for_assets()` over a fixed `sleep` when you need the
asset list to be populated.

## Async methods

### Account and session

| Method                          | Description                        | Returns      |
| ------------------------------- | ---------------------------------- | ------------ |
| `balance()`                     | Account balance                    | `float`      |
| `is_demo()` †                   | Whether this is a demo account     | `bool`       |
| `is_connected()` †              | Socket is currently connected      | `bool`       |
| `is_ssid_valid()` †             | SSID was accepted                  | `bool`       |
| `get_server_time()`             | Broker clock, in seconds           | `int`        |
| `wait_for_assets(timeout=60.0)` | Block until the asset list arrives | `None`       |
| `active_assets()`               | Assets currently tradable          | `List[Dict]` |
| `connect()` / `disconnect()`    | Open or drop the socket            | `None`       |
| `reconnect()`                   | Reconnect and re-authenticate      | `None`       |
| `shutdown()`                    | Close the client and its tasks     | `None`       |

† These three are **synchronous** even on `PocketOptionAsync` — they read local
state, so do not `await` them: `if client.is_demo(): ...` works, while `await
client.is_demo()` raises `TypeError: 'bool' object can't be awaited`. Every other
method in these tables is a coroutine.

### Trading

| Method                                       | Description                        | Returns           |
| -------------------------------------------- | ---------------------------------- | ----------------- |
| `buy(asset, amount, time, check_win=False)`  | Call trade; `time` in seconds      | `TradeResult`     |
| `sell(asset, amount, time, check_win=False)` | Put trade; `time` in seconds       | `TradeResult`     |
| `check_win(id, timeout_seconds=None)`        | Wait for a trade to settle         | `Trade`           |
| `opened_deals()`                             | Open trades                        | `List[Dict]`      |
| `closed_deals()`                             | Settled trades                     | `List[Dict]`      |
| `get_opened_deal(trade_id)`                  | One open trade, or `None`          | `Optional[Trade]` |
| `get_closed_deal(trade_id)`                  | One settled trade, or `None`       | `Optional[Trade]` |
| `get_pending_deals()`                        | Queued pending orders              | `List[Dict]`      |
| `clear_closed_deals()`                       | Empty the local closed-trade cache | `None`            |
| `get_deal_end_time(trade_id)`                | Scheduled settlement timestamp     | `Optional[int]`   |

`TradeResult` is the tuple `(trade_id, trade)` — note the order of `amount` and
`time` in `buy`/`sell`, and that `trade` may not be settled yet:

```python
trade_id, trade = await client.buy("EURUSD_otc", 1.0, 60)
settled = await client.check_win(trade_id)
print(settled.result, settled.profit, settled.is_win)
```

`Trade` is a `dict` subclass: `trade["profit"]` and `trade.profit` both work, and it
adds `is_win` / `is_loss` / `is_draw` / `is_settled`.

### Pending orders and payout

| Method                                                                                                             | Returns          |
| ------------------------------------------------------------------------------------------------------------------ | ---------------- |
| `open_pending_order(*, open_type, amount, asset, open_time, open_price, timeframe, min_payout=None, command=None)` | `Dict`           |
| `cancel_pending_order(ticket)`                                                                                     | `Dict`           |
| `cancel_pending_orders(tickets)`                                                                                   | `Dict`           |
| `payout(asset)`                                                                                                    | `Optional[int]`  |
| `payouts()`                                                                                                        | `Dict[str, int]` |

### Market data

| Method                                                     | Description                                           | Returns                                                   |
| ---------------------------------------------------------- | ----------------------------------------------------- | --------------------------------------------------------- |
| `candles(asset, period, count=100, end_time=None)`         | Last `count` closed candles                           | `List[Dict]`                                              |
| `candles_many(assets, period, *, count=100)`               | Candles for many symbols in one call                  | `Dict[str, List[Dict]]`                                   |
| `get_candles(asset, period, offset)` (deprecated)          | `offset` is a **count of periods**, not seconds       | `List[Dict]`                                              |
| `get_candles_advanced(asset, period, offset, time)`        | One `loadHistoryPeriod` page; `offset` is **seconds** | `List[Dict]`                                              |
| `get_candles_live(asset, period, hours=2.0, max_rows=100)` | Backfill + gap-free live candles                      | `AsyncGenerator[Tuple[List[Dict], Optional[Dict]], None]` |
| `stream_candles(asset, period, *, history, max_rows)`      | Live candle stream with configurable backfill         | `AsyncGenerator[Tuple[List[Dict], Optional[Dict]], None]` |
| `ticks(asset, seconds)`                                    | Raw ticks over a window                               | `List[Tuple[int, float]]`                                 |
| `get_ticks(asset, lookback_seconds)`                       | Same, explicit lookback name                          | `List[Tuple[int, float]]`                                 |
| `history(asset, period)`                                   | Alias of `candles()`                                  | `List[Dict]`                                              |
| `compile_candles(asset, custom_period, lookback_period)`   | Aggregate ticks into a non-standard timeframe         | `List[Dict]`                                              |

Candles are dictionaries with `time`, `open`, `high`, `low`, `close`. The two
`offset` meanings above are the most common source of "I asked for 200 candles and
got nothing" — see [candle history limits](../guides/candle-history-limits.md).

`candles_many` polls snapshots rather than holding one stream per symbol, so it is
the right shape for a dozen or more instruments:

```python
feed = await client.candles_many(["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"], 60, count=2)
for symbol, candles in feed.items():
    print(symbol, candles[-1]["close"])
```

Symbols that cannot be fetched are absent from the result rather than failing the
whole call.

### Subscriptions

| Method                                                     | Yields                                |
| ---------------------------------------------------------- | ------------------------------------- |
| `subscribe_symbol(asset)`                                  | `AsyncSubscription` of closed candles |
| `subscribe_symbol_chunked(asset, chunk_size)`              | `AsyncSubscription`                   |
| `subscribe_symbol_timed(asset, time)`                      | `AsyncSubscription`                   |
| `subscribe_symbol_time_aligned(asset, time)`               | `AsyncSubscription`                   |
| `subscribe_ticks(asset, *, chunk_size, interval, aligned)` | `AsyncSubscription`                   |
| `unsubscribe(asset)`                                       | `None`                                |

```python
sub = await client.subscribe_symbol("EURUSD_otc")
async for candle in sub:
    print(candle)
    if done():
        break
await client.unsubscribe("EURUSD_otc")  # release the stream
```

`AsyncSubscription` is an iterator only — it has no `close()`. Release the stream
with `unsubscribe(asset)` (or by dropping the client).

The broker streams only about two symbols per session (see
[`OBSERVED_CONCURRENT_STREAM_LIMIT`](../../crates/binary_options_tools/src/pocketoption/modules/subscriptions.rs)),
and has no working unsubscribe event — `unsubscribe()` releases the stream locally
so another symbol can use it. For many symbols, poll `candles_many()` instead.

### Raw access

| Method                                                                           | Returns                |
| -------------------------------------------------------------------------------- | ---------------------- |
| `send_raw(message)` / `send_raw_message(message)`                                | `None`                 |
| `subscribe_raw()`                                                                | `AsyncRawSubscription` |
| `raw_handler(validator=None, keep_alive=None)`                                   | `RawHandler`           |
| `create_raw_handler(validator=None, keep_alive=None)`                            | `RawHandler`           |
| `raw_request(message, validator=None, timeout=None, retry=None)`                 | `str`                  |
| `create_raw_order(message, validator=None)`                                      | `str`                  |
| `create_raw_order_with_timeout(message, validator=None, timeout=None)`           | `str`                  |
| `create_raw_order_with_timeout_and_retry(message, validator=None, timeout=None)` | `str`                  |
| `raw_stream(...)` / `create_raw_iterator(...)`                                   | raw stream             |

See the [Raw Handler guide](../guides/raw-handler.md).

## Sync client

`PocketOption` mirrors the async surface without `await`; generators become plain
iterators (`SyncCandleLiveIterator`, `SyncSubscription`).

```python
from BinaryOptionsToolsV2 import PocketOption

with PocketOption("your_ssid") as client:
    balance = client.balance()
    trade_id, trade = client.buy("EURUSD_otc", 1.0, 60)
    settled = client.check_win(trade_id)
    print(settled.result, settled.profit)
```

The sync methods are `candles`, `candles_many`, `get_candles`,
`get_candles_advanced`, `get_candles_live`, `stream_candles`, `ticks`, `get_ticks`,
`history`, `compile_candles`, `balance`, `opened_deals`, `closed_deals`,
`get_opened_deal`, `get_closed_deal`, `get_pending_deals`, `clear_closed_deals`,
`open_pending_order`, `cancel_pending_order`, `cancel_pending_orders`, `payout`,
`payouts`, `active_assets`, `get_server_time`, `wait_for_assets`, `is_demo`,
`is_connected`, `is_ssid_valid`, `connect`, `disconnect`, `reconnect`, `shutdown`,
and the `subscribe_symbol*` / `subscribe_ticks` / `subscribe_raw` family.

## Try demo (guest) sessions

`try_demo` is a classmethod; PocketOption's free demo hands out a short-lived guest
token instead of an account session, so no credentials are needed:

```python
from BinaryOptionsToolsV2 import PocketOptionAsync

async with await PocketOptionAsync.try_demo() as client:
    print(await client.balance())
```

Pass a token (`try_demo("...")`) to rejoin a previous guest session.

## Errors

Failures raise `PocketError` (subclasses such as the invalid-asset error carry the
reason). A closed market fails immediately with the reopen time rather than waiting
for a timeout:

```python
from BinaryOptionsToolsV2 import PocketError

try:
    await client.candles("EURUSD", 60)
except PocketError as e:
    print(e)  # Asset `EURUSD` is not active: its market is closed (reopens ...)
```

## Configuration

`Config` (or a dict / JSON string) is passed as `config=`. See
[the config module](../../crates/core/) for the full field list; the common knobs
are log level and reconnect behaviour.

## Risk management sketch

Check the balance, size from it, and settle through `check_win` — not a fixed sleep:

```python
class SafeTrader:
    def __init__(self, client, max_daily_loss=10.0, risk_per_trade=0.02):
        self.client = client
        self.max_daily_loss = max_daily_loss
        self.risk_per_trade = risk_per_trade
        self.daily_pnl = 0.0

    async def can_trade(self):
        return self.daily_pnl > -self.max_daily_loss

    async def trade(self, asset, action, time):
        if not await self.can_trade():
            raise RuntimeError("Daily loss limit reached")

        amount = round(await self.client.balance() * self.risk_per_trade, 2)
        if action == "call":
            trade_id, _ = await self.client.buy(asset, amount, time)
        else:
            trade_id, _ = await self.client.sell(asset, amount, time)

        settled = await self.client.check_win(trade_id, timeout_seconds=time + 30)
        self.daily_pnl += settled.profit or 0.0
        return settled
```

> The examples above use the signatures implemented today. Older revisions of this
> page showed `buy(asset, time, amount)`, `client.result(id)`, `trade(…)`,
> `subscribe(asset, duration)`, `get_opened_deals()` and an awaited constructor;
> none of those exist.
