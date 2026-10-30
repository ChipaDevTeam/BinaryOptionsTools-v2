# How Much Candle History You Can Fetch

There are two ways to get historical candles, and they have very different
limits. One is capped by this library, the other is not capped by it at all.
Picking the wrong one is the usual reason a call returns fewer candles than
expected.

## The short answer

| Method | Capped by this library? | Practical ceiling |
| --- | --- | --- |
| `get_candles_advanced(asset, period, offset, time)` | No | Everything the `loadHistoryPeriod` response yields, compiled into candles (often empty, see below) |
| `candles(asset, period, count, end_time=...)` | Only without `end_time` | With `end_time`: the same endpoint response, compiled into candles. Without: 20,000 ticks |
| `get_candles(asset, period, offset)` (deprecated) | Yes | Alias of `get_candles_live()`: 20,000 ticks |
| `candles(asset, period)` | Yes | 20,000 ticks, so ~20,000 seconds of history |
| `history(asset, period)` | Yes | Alias of `candles()` |
| `compile_candles(asset, custom_period, lookback_period)` | Yes | Same 20,000-tick ceiling |

## Server-side: `get_candles_advanced`

`get_candles_advanced(asset, period, offset, time)` sends a single
`loadHistoryPeriod` request and returns the candles compiled from its response.
There is no pagination and no client-side cap, so the limit is PocketOption's,
not this library's.

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

> **`offset` is a window in seconds, not a candle count.** The server answers
> with the candles that fit in `time - offset ..= time`, and a window shorter
> than one period contains no candle: `get_candles("EURUSD_otc", 60, 10)`
> therefore returns an empty list, while `get_candles("EURUSD_otc", 60, 3600)`
> returns an hour of 60-second candles. Ask for at least `period * candles`.
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

| `period` | Lookback requested | Ticks needed | Candles you actually get |
| --- | --- | --- | --- |
| 1s | 1,000s | ~1,000 | ~1000 |
| 5s | 5,000s | ~5,000 | ~1000 |
| 15s | 15,000s | ~15,000 | ~1000 |
| 20s | 20,000s | ~20,000 | ~1000 (right at the limit) |
| 60s | 60,000s | ~60,000 | **~333** |
| 300s | 300,000s | ~300,000 | **~66** |

So `candles("EURUSD_otc", 60)` returns roughly a third of the 1000 candles the
call implies, and the only signal is a `warn!` in the logs. This is a real
limitation, not a bug in your code.

`compile_candles(asset, custom_period, lookback_period)` lets you set the
lookback explicitly, but it goes through the same fetcher — asking for more
than about 20,000 seconds (~5.5 hours) will not get you more data.

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
trades. A quiet asset produces sparser ticks, so a page covers *more* time and
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
in-memory cache of *live* ticks from subscriptions. They do not cap historical
fetches; they limit how many buffered live ticks can be appended to
`get_candles_advanced` and `get_ticks` responses.

## Version Information

This documentation is for BinaryOptionsTools-v2. Limits may change in future
releases; the constants named above are the authoritative source.

For the latest updates, check the [GitLab repository](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2).
