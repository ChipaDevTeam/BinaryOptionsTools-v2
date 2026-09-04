# How Much Candle History You Can Fetch

There are two ways to get historical candles, and they have very different
limits. One is capped by this library, the other is not capped by it at all.
Picking the wrong one is the usual reason a call returns fewer candles than
expected.

## The short answer

| Method | Capped by this library? | Practical ceiling |
|---|---|---|
| `get_candles(asset, period, offset)` | No | Whatever the server returns for one request |
| `get_candles_advanced(asset, period, offset, time)` | No | Same, anchored at `time` |
| `candles(asset, period)` | Yes | 20,000 ticks, so ~20,000 seconds of history |
| `history(asset, period)` | Yes | Alias of `candles()` |
| `compile_candles(asset, custom_period, lookback_period)` | Yes | Same 20,000-tick ceiling |

## Server-side: `get_candles`

`get_candles` and `get_candles_advanced` send a single `loadHistoryPeriod`
request and return what comes back. There is no pagination and no client-side
cap, so the limit is PocketOption's, not this library's.

```python
candles = await api.get_candles("EURUSD_otc", 60, 3600)
```

Use these when you want a lot of history on a standard timeframe and do not
need candles aligned to your own boundaries.

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
|---|---|---|---|
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

- **Standard timeframe, want depth** → `get_candles`. No library cap.
- **Non-standard timeframe, or you need UTC-aligned boundaries** →
  `compile_candles`, and keep `lookback_period` under ~20,000 seconds.
- **Short timeframes (≤20s)** → `candles()` behaves as documented.
- **Long timeframes (≥60s) and you need 1000 candles** → use `get_candles`, or
  make several `get_candles_advanced` calls with different `time` anchors and
  join the results.

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

`MAX_TICKS_PER_ASSET` and `MAX_TICK_AGE_SECS` in the same module are unrelated
to historical fetches: they bound the in-memory cache of *live* ticks from
subscriptions and do not affect any of the calls above.

## Version Information

This documentation is for BinaryOptionsTools-v2. Limits may change in future
releases; the constants named above are the authoritative source.

For the latest updates, check the [GitLab repository](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2).
