# How Much Candle History You Can Fetch

There are two ways to get historical candles, and they have very different
limits. One is capped by this library, the other is not capped by it at all.
Picking the wrong one is the usual reason a call returns fewer candles than
expected.

## The short answer

| Method                                                   | Capped by this library? | Practical ceiling                                                                                  |
| -------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------- |
| `get_candles_advanced(asset, period, offset, time)`      | No                      | Everything the `loadHistoryPeriod` response yields, compiled into candles (often empty, see below) |
| `candles(asset, period, count, end_time=...)`            | Only without `end_time` | With `end_time`: the same endpoint response, compiled into candles. Without: 20,000 ticks          |
| `get_candles(asset, period, offset)` (deprecated)        | Yes                     | Alias of `get_candles_live()`: 20,000 ticks                                                        |
| `candles(asset, period)`                                 | Yes                     | 20,000 ticks, so ~20,000 seconds of history                                                        |
| `history(asset, period)`                                 | Yes                     | Alias of `candles()`                                                                               |
| `compile_candles(asset, custom_period, lookback_period)` | Yes                     | Same 20,000-tick ceiling                                                                           |

## Server-side: `get_candles_advanced`

`get_candles_advanced(asset, period, offset, time)` sends a single
`loadHistoryPeriod` request and returns the candles compiled from its response.
It is a single request, so the window is capped by what one page returns
(about 150 OHLC rows in the captures from 2026-10-30).

```python
candles = await api.get_candles_advanced("EURUSD_otc", 60, 3600, 1700000000)
```

`GetCandlesApiModule::process_result` does not hand the raw endpoint payload
back to you. It turns the response into candles: entries that already carry
OHLC are returned as candles, anything else is treated as ticks and aggregated
into candles of the requested period. It also appends the `updateStream` ticks
it has buffered for that asset (at most 10,000 ticks, no older than five
minutes). So an empty endpoint `data` array does **not** guarantee an empty
returned list - buffered live ticks are compiled into candles too.

> **`get_candles_advanced`'s `offset` is a window in seconds, not a candle
> count.** The server answers with the candles that fit in `time - offset ..=
time`, so a window shorter than one period contains no candle:
> `get_candles_advanced("EURUSD_otc", 60, 10, now)` returns an empty list, while
> `get_candles_advanced("EURUSD_otc", 60, 3600, now)` returns an hour of
> 60-second candles. Ask for at least `period * candles`.
>
> This applies **only** to `get_candles_advanced`. The deprecated
> `get_candles(asset, period, offset)` takes a _count_ of periods, not seconds:
> it multiplies `offset * period` to size its lookback window before delegating
> to `get_candles_live()`.
>
> **One request is one page.** The page is capped by the server (the captures
> from 2026-10-30 return ~150 OHLC rows per request), so longer windows need
> pagination - which is what `candles()` and `compile_candles()` do by walking
> `time` backwards. If the server does not answer at all, the call fails with a
> timeout instead of hanging.

`get_candles(asset, period, offset)` is deprecated and does **not** call this
endpoint: it delegates to `get_candles_live()`, which compiles candles from tick
pages (falling back to the `changeSymbol` chart history when those pages do not
cover the window) and then replays the live ticks it buffered while fetching.
That path is bounded by the tick fetcher described below, which is why
`get_candles()` is capped by this library and `get_candles_advanced()` is not.

## Tick-compiled: `candles`, `history`, `compile_candles`

These fetch raw 1-second ticks and aggregate them client-side, which is what
lets them produce non-standard timeframes (20s, 40s, 90s) and align every
candle to a UTC boundary. The cost is a hard ceiling in the tick fetcher:

```rust
const DEFAULT_PAGE_OFFSET: i64 = 1000;   // ticks requested per page
let mut max_pages = 20;                  // safety limit against infinite loops
```

**20 pages × 1000 ticks = 20,000 ticks, maximum.** When the requested window is
not covered within those 20 pages the loop stops, logs
`Reached max pagination pages`, and returns what it collected. Pagination also
stops early as soon as the earliest tick reaches the target time, so the cap
only matters when you ask for more than it can reach.

### The part that surprises people

`candles(asset, period)` asks for `1000 * period` seconds of history — it is
trying to give you 1000 candles. Ticks arrive at roughly one per second, so
that request only fits inside the 20,000-tick budget while `period` is 20
seconds or less. Above that, the tick budget runs out before the window is
covered:

| `period` | Lookback requested | Ticks needed | Candles you actually get   |
| -------- | ------------------ | ------------ | -------------------------- |
| 1s       | 1,000s             | ~1,000       | ~1000                      |
| 5s       | 5,000s             | ~5,000       | ~1000                      |
| 15s      | 15,000s            | ~15,000      | ~1000                      |
| 20s      | 20,000s            | ~20,000      | ~1000 (right at the limit) |
| 60s      | 60,000s            | ~60,000      | **~333**                   |
| 300s     | 300,000s           | ~300,000     | **~66**                    |

So `candles("EURUSD_otc", 60)` returns roughly a third of the 1000 candles the
call implies, and the only signal is a `warn!` in the logs. This is a real
limitation, not a bug in your code.

`compile_candles(asset, custom_period, lookback_period)` lets you set the
lookback explicitly, but it goes through the same fetcher — asking for more
than about 20,000 seconds (~5.5 hours) will not get you more data.

## Watching many symbols at once

Two separate broker limits decide this, both measured against the live servers on
2026-10-30:

| Limit                       | Measured behaviour                                                                                                                          | What to do                                                          |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Concurrent live streams     | A session streams the `changeSymbol` symbol plus about one watchlist symbol. Extra subscriptions never receive ticks.                       | Keep a small rotating pool, or poll snapshots instead of streaming. |
| Concurrent history requests | Requests spaced ~200 ms apart are answered every time; a burst loses answers, and a hammered session degrades until _everything_ times out. | Nothing - the client paces requests for you.                        |
| History requests per second | The 200 ms pace and the four-slot cap work out to roughly five requests per second for the whole client.                                     | Budget `len(assets) / 5` seconds; nothing can bypass the pace.       |

`unsubscribeSymbol` is accepted but ignored by the server, so a released stream
only actually stops when the chart moves to another symbol. The client tracks
which symbol owns the chart stream and hands it to a symbol that is still
subscribed, so rotating through assets keeps working; it also skips redundant
`changeSymbol` frames, each of which interrupts the stream.

For "give me the latest M1 candle of every symbol each minute", poll snapshots:

```python
feed = await client.candles_many(["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"], 60, count=2)
for symbol, candles in feed.items():
    print(symbol, candles[-1]["close"])
```

This is not subject to the stream ceiling and is the supported path for a dozen
or more instruments. A 12-symbol batch completes reliably; streaming the same 12
does not, because the server will only feed about two of them. `candles_many`
compiles each snapshot from tick pages directly (no per-symbol subscription) and
overlaps up to four requests, but the 200 ms pace is a hard floor: expect roughly
`len(assets) / 5` seconds.

That floor only applies to actual fetches. Closed candles are immutable, so
`candles_many` caches the snapshot per `(symbol, period, count)` until the next
candle boundary: a 2.5 s poll of a 60 s timeframe reuses the cached candles for
about 24 of every 25 calls, and concurrent rooms asking for the same key share one
in-flight request. Polling with `count=2` instead of the default `count=100` keeps
each fetch to a single tick page. A disconnected client, or one whose channel
closed, raises `ConnectionError` instead of returning an empty mapping.

The pacing also sets a hard size limit per timeframe: a full sweep of _n_ symbols
takes about `n / 5` seconds, so 100 symbols need ~20 s and cannot feed a 5 s
timeframe from one client. Divide a wide, fast feed across several clients (each
has its own pacer and its own five requests per second) or poll fewer symbols at
that cadence. Cache hits do not consume the budget, so a 60 s timeframe costs one
sweep per minute no matter how often it is polled.

### Recovering from a stalled session

A session that stops answering does not start returning data on its own, and
`candles_many` will not paper over it with an empty result. Two tools make the
failure detectable and recoverable without restarting the process:

- `client.is_responsive(max_silence=...)` (synchronous on both clients) reports
  whether the socket is connected **and** a frame — including the broker's
  Engine.IO ping — arrived recently. It reads local state, so it cannot block,
  even from a worker thread that is otherwise waiting on the same client.
- `candles_many(..., timeout=...)` (and `payout` / `payouts`) cancels the request
  inside the engine when the deadline expires, releasing its history slot instead
  of leaving the work running behind the caller.

```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def poll(client, symbols, period, retries=2):
    for attempt in range(retries + 1):
        if not client.is_responsive(max_silence=90):
            # Connected but silent: a request would only queue behind the dead
            # ones. Rebuild first, then try.
            await client.reconnect()
        try:
            return await client.candles_many(
                symbols, period, count=2, timeout=60
            )
        except (ConnectionError, TimeoutError):
            # `reconnect()` re-authenticates over the same client and drops the
            # cached snapshots; give up after the last retry and restart the task.
            if attempt == retries:
                raise
            await client.reconnect()
```

`reconnect()` keeps the client object, so callers holding it do not have to be
rebuilt; the cached `candles_many` snapshots are dropped and the next poll fetches
fresh ones. Run reconnects one at a time per client (a lock around the call is
enough) so several loops do not cycle the same connection concurrently.

## Which to use

- **Standard timeframe, want endpoint history** → `get_candles_advanced` (or
  `candles(..., end_time=...)`, which calls it). There is no library cap, but the
  endpoint may answer with nothing, and there is no pagination.
- **`get_candles` is not the deep-history option.** It is deprecated and
  delegates to `get_candles_live()`, so it is bound by the 20,000-tick ceiling
  above and cannot return 1,000 long-timeframe candles either.
- **Non-standard timeframe, or you need UTC-aligned boundaries** →
  `compile_candles`, and keep `lookback_period` under ~20,000 seconds.
- **Short timeframes (≤20s)** → `candles()` behaves as documented.
- **Long timeframes (≥60s) and you need 1000 candles** → `get_candles_advanced`
  with different `time` anchors, joined together, keeping its documented
  empty-response limitation in mind.

## A caveat on "roughly one tick per second"

The tick-rate assumption is what turns "20,000 ticks" into "~20,000 seconds",
and it is an approximation. Each page is requested with `period=1, offset=1000`,
so how much wall-clock time a page spans depends on how actively the asset
trades. A quiet asset produces sparser ticks, so a page covers _more_ time and
the same budget reaches further back. Treat the numbers above as the busy-market
case, which is the pessimistic one.

To measure it for an asset you care about, fetch the ticks directly and compare
the span to the count:

```python
ticks = await api.get_ticks("EURUSD_otc", 3600)
print(len(ticks), ticks[-1][0] - ticks[0][0])  # count vs seconds covered
```

## Where these limits live

- `crates/binary_options_tools/src/pocketoption/modules/get_candles.rs` —
  `DEFAULT_PAGE_OFFSET`, `max_pages`, and the `get_ticks` pagination loop.
- `crates/binary_options_tools/src/pocketoption/pocket_client.rs` —
  `candles()` choosing `1000 * period` as its lookback, and `compile_candles()`.

`MAX_TICKS_PER_ASSET` and `MAX_TICK_AGE_SECS` in the same module bound the
in-memory cache of _live_ ticks from subscriptions. They do not cap historical
fetches; they limit how many buffered live ticks can be appended to
`get_candles_advanced` and `get_ticks` responses.

## Version Information

This documentation is for BinaryOptionsTools-v2. Limits may change in future
releases; the constants named above are the authoritative source.

For the latest updates, check the [GitLab repository](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/tree/synced/master).
