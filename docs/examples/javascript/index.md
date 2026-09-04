<<<<<<< HEAD
---
sidebar_position: 4
slug: /examples/javascript
---

# JavaScript / TypeScript Examples

This directory contains JavaScript examples using `BinaryOptionsToolsV2`.

## Examples

| File | Description |
|------|-------------|
| `get_balance.js` | Get account balance |
| `stream.js` | Subscribe to real-time data for a symbol |
| `get_candles.js` | Get candle data for a symbol |
| `check_win.js` | Check if a trade was won |
| `history.js` | Get trade history |
| `payout.js` | Get payout information |
| `raw_send.js` | Send raw messages to the server |
| `create_raw_order.js` | Create a raw order |
| `create_raw_iterator.js` | Using the raw iterator for custom processing |
| `get_deal_end_time.js` | Get the end time of a deal |
| `logs.js` | Display logs |
| `validator.js` | Validate session data |

## Prerequisites

1. Node.js 18+ installed
2. Build the native module:
   ```bash
   cd examples/javascript
   npm install
   ```

## Running the Examples

```bash
node get_balance.js
node stream.js
node get_candles.js
# etc.
```

## Example: Get Balance

**File**: `get_balance.js`

```javascript
const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Get balance
  const balance = await api.balance();
  console.log(`Balance: ${balance}`);
}

// Check if ssid is provided as command line argument
const ssid = process.argv[2] || "";

main(ssid).catch(console.error);
```

**Run:**
```bash
node get_balance.js "your-ssid-here"
```

## Example: Stream Real-time Data

**File**: `stream.js`

```javascript
const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Subscribe to a symbol stream
  const stream = await api.subscribe("EURUSD_otc");

  console.log("Starting stream...");

  // Listen to the stream for 1 minute
  const endTime = Date.now() + 60000; // 60 seconds

  try {
    for await (const data of stream) {
      console.log("Received data:", data);

      if (Date.now() > endTime) {
        console.log("Stream time finished");
        break;
      }
    }
  } catch (error) {
    console.error("Stream error:", error);
  } finally {
    // Clean up
    await stream.close();
  }
}

// Check if ssid is provided as command line argument
const ssid = process.argv[2] || "";

main(ssid).catch(console.error);
```

**Run:**
```bash
node stream.js "your-ssid-here"
```

## Example: Get Candles

**File**: `get_candles.js`

```javascript
const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  const api = new PocketOption(ssid);
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Get last 100 candles with 60-second period
  const candles = await api.getCandles("EURUSD_otc", 60, 100);
  
  console.log(`Retrieved ${candles.length} candles`);
  candles.slice(0, 5).forEach(candle => {
    console.log(`  Time: ${candle.time}, Close: ${candle.close}`);
  });

  await api.shutdown();
}

const ssid = process.argv[2] || "";
main(ssid).catch(console.error);
```

## Example: Check Trade Result

**File**: `check_win.js`

```javascript
const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  const api = new PocketOption(ssid);
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Place a trade
  const trade = await api.buy("EURUSD_otc", 60, 1.0);
  console.log(`Trade placed: ${trade.id}`);

  // Wait for trade to complete
  await new Promise((resolve) => setTimeout(resolve, 65000));

  // Check result
  const result = await api.result(trade.id);
  console.log(`Result: ${result.profit > 0 ? 'WIN' : 'LOSS'}`);
  console.log(`Profit: $${result.profit}`);

  await api.shutdown();
}

const ssid = process.argv[2] || "";
main(ssid).catch(console.error);
```

## Example: Raw Handler

**File**: `raw_send.js`

```javascript
const { PocketOption, Validator } = require("./binary-options-tools.node");

async function main(ssid) {
  const api = new PocketOption(ssid);
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Create validator for balance messages
  const validator = Validator.contains('"balance"');

  // Create raw handler
  const handler = await api.createRawHandler(validator, null);

  // Send custom message
  await handler.sendText('42["getBalance"]');

  // Wait for response
  const response = await handler.waitNext();
  console.log("Response:", response);

  await api.shutdown();
}

const ssid = process.argv[2] || "";
main(ssid).catch(console.error);
```

## Example: Trade History

**File**: `history.js`

```javascript
const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  const api = new PocketOption(ssid);
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Get closed deals
  const deals = await api.getClosedDeals();
  
  console.log(`Closed trades: ${deals.length}`);
  deals.forEach(deal => {
    const result = deal.profit > 0 ? 'WIN' : 'LOSS';
    console.log(`  ${deal.asset}: ${result} ($${deal.profit})`);
  });

  await api.shutdown();
}

const ssid = process.argv[2] || "";
main(ssid).catch(console.error);
```

## Key Concepts

### Initialization
Always wait for the WebSocket connection to establish:

```javascript
const api = new PocketOption(ssid);
await new Promise((resolve) => setTimeout(resolve, 5000));
```

### Demo vs Real Account
```javascript
if (!api.isDemo()) {
    console.warn("WARNING: Using REAL account!");
}
```

### Cleanup
Always shutdown the client:

```javascript
await api.shutdown();
```

Or use try/finally:

```javascript
try {
    // trading code
} finally {
    await api.shutdown();
}
```

### Async Iterators
For streams, use `for await`:

```javascript
const stream = await api.subscribe("EURUSD_otc", 60);
for await (const candle of stream) {
    console.log(candle);
}
await stream.close();
```

## TypeScript Support

TypeScript definitions are included. Import types:

```typescript
import { PocketOption, Deal, Candle, Validator } from "./binary-options-tools.node";

const api: PocketOption = new PocketOption(ssid);
const deal: Deal = await api.buy("EURUSD_otc", 60, 1.0);
```

## Common Assets

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## Additional Resources

- **Documentation**: [https://chipatrade.gitlab.io/chipadevorg/BinaryOptionsTools-v2/](https://chipatrade.gitlab.io/chipadevorg/BinaryOptionsTools-v2/)
- **Discord**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. These examples are provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.


---

## Build strategies faster

- **✨ [ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=examples-lang&utm_campaign=BinaryOptionsToolsV2)** — AI-powered algorithmic **trading strategy builder**: describe your
  edge, get a working **CHTL** strategy, backtest it on historical data, deploy it to a live broker.
  Free tier, browser and Android. *(A trading platform — not a general-purpose code editor.)*
- **📈 [ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — hybrid crypto exchange: perpetuals, spot and margin, with demo mode.
- **[Chipa Ecosystem overview](/ecosystem)** — how these fit together with this library.
=======
---
sidebar_position: 4
slug: /examples/javascript
---

# JavaScript / TypeScript Examples

The Node.js bindings live in the `nodejs/` package and are backed by the
native N-API addon built from `crates/bindings_napi`. Runnable versions of
everything below are in [`examples/javascript`](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/tree/master/examples/javascript).

## Examples

| File | Description |
|------|-------------|
| `balance.js` | Get account balance |
| `basic.js` | Connect, read the balance, disconnect |
| `buy.js` | Place a buy trade |
| `sell.js` | Place a sell trade |
| `check_win.js` | Open a trade and wait for its result |
| `get_candles.js` | Get candle data for a symbol |
| `history.js` | Get candle history |
| `payout.js` | Get payout information |
| `subscribe_symbol.js` | Stream real-time candles |
| `raw_send.js` | Send raw messages to the server |
| `create_raw_order.js` | Send raw messages and wait for matching responses |
| `create_raw_iterator.js` | Iterate over raw responses matching a validator |
| `logs.js` | Write library logs to disk and to the terminal |
| `validator.js` | Compose message validators |

## Prerequisites

1. Node.js 18+ and the [Rust toolchain](https://rustup.rs)
2. Build the native addon:

   ```bash
   cd nodejs
   npm run build
   ```

   This runs `cargo build -p binary_options_tools_napi --release` and copies
   the result to `nodejs/binary-options-tools.node`.

## Running the Examples

Every script except `validator.js` takes the session id as its first argument
(or reads `POCKET_OPTION_SSID`):

```bash
cd examples/javascript
node balance.js "your-ssid-here"
node subscribe_symbol.js "your-ssid-here"
```

## Example: Get Balance

**File**: `balance.js`

```javascript
const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  // The constructor returns immediately and connects in the background;
  // every method awaits that connection before running.
  const api = new PocketOption(ssid);

  const balance = await api.balance();
  console.log(`Balance: ${balance}`);

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
```

**Run:**

```bash
node balance.js "your-ssid-here"
```

## Example: Stream Real-time Data

**File**: `subscribe_symbol.js`

```javascript
const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  // create() rejects when the connection cannot be established.
  const api = await PocketOption.create(ssid);

  // Without the second argument every update is yielded as it arrives; with
  // it the updates are aggregated into candles of that many seconds.
  const stream = await api.subscribe("EURUSD_otc", 60);

  const endTime = Date.now() + 60_000;
  for await (const candle of stream) {
    console.log("Received candle:", candle);
    if (Date.now() > endTime) break;
  }

  await api.unsubscribe("EURUSD_otc");
  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
```

## Example: Get Candles

**File**: `get_candles.js`

```javascript
const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // 60-second candles, looking back one hour.
  const candles = await api.getCandles("EURUSD_otc", 60, 3600);

  console.log(`Retrieved ${candles.length} candles`);
  for (const candle of candles.slice(0, 5)) {
    console.log(`  Time: ${candle.time}, Close: ${candle.close}`);
  }

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
```

## Example: Check Trade Result

**File**: `check_win.js`

```javascript
const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // buy(asset, amount, seconds) resolves to [dealId, deal].
  const [dealId] = await api.buy("EURUSD_otc", 1.0, 60);
  console.log(`Trade placed: ${dealId}`);

  // result() resolves once the server settles the trade.
  const deal = await api.result(dealId);
  console.log(`Result: ${deal.profit > 0 ? "WIN" : "LOSS"}`);
  console.log(`Profit: $${deal.profit}`);

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
```

## Example: Raw Handler

**File**: `create_raw_order.js`

```javascript
const { PocketOption, Validator } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // The handler keeps every message its validator accepts, so a response
  // that arrives early is not lost.
  const validator = Validator.contains('"balance"');
  const handler = await api.createRawHandler(validator);

  await handler.sendText('42["getBalance"]');
  console.log("Response:", await handler.waitNext());

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
```

## Example: Trade History

**File**: `history.js`

```javascript
const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // Settled deals, keyed by deal id.
  const deals = await api.closedDeals();

  console.log(`Closed trades: ${Object.keys(deals).length}`);
  for (const deal of Object.values(deals)) {
    console.log(`  ${deal.asset}: ${deal.profit > 0 ? "WIN" : "LOSS"} ($${deal.profit})`);
  }

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
```

## Key Concepts

### Initialization

`new PocketOption(ssid)` returns immediately and connects in the background.
Every method waits for that connection first, so no sleep is needed. Use the
static helper when a failed connection should be reported up front:

```javascript
const api = await PocketOption.create(ssid);
```

### Demo vs Real Account

```javascript
if (!(await api.isDemo())) {
    console.warn("WARNING: Using REAL account!");
}
```

### Cleanup

Always shut the client down:

```javascript
try {
    // trading code
} finally {
    await api.shutdown();
}
```

### Async Iterators

Subscriptions and raw handlers are async iterables:

```javascript
const stream = await api.subscribe("EURUSD_otc", 60);
for await (const candle of stream) {
    console.log(candle);
}
await api.unsubscribe("EURUSD_otc");
```

### Method Names

Every method has both a `camelCase` and a `snake_case` spelling, so
`api.getCandles(...)` and `api.get_candles(...)` are the same function.

### Errors

Failures reject with a regular `Error` whose message is prefixed by the error
kind, for example `TimeoutError: the operation timed out`.

## TypeScript Support

TypeScript definitions ship with the package:

```typescript
import { PocketOption, Candle, Deal, Validator } from "binary-options-tools";

const api: PocketOption = await PocketOption.create(ssid);
const candles: Candle[] = await api.getCandles("EURUSD_otc", 60, 3600);
const [dealId] = await api.buy("EURUSD_otc", 1.0, 60);
const deal: Deal = await api.result(dealId);
```

## Common Assets

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## Additional Resources

- **Documentation**: [https://chipatrade.gitlab.io/chipadevorg/BinaryOptionsTools-v2/](https://chipatrade.gitlab.io/chipadevorg/BinaryOptionsTools-v2/)
- **Discord**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. These examples are provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.


---

## Build strategies faster

- **✨ [ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=examples-lang&utm_campaign=BinaryOptionsToolsV2)** — AI-powered algorithmic **trading strategy builder**: describe your
  edge, get a working **CHTL** strategy, backtest it on historical data, deploy it to a live broker.
  Free tier, browser and Android. *(A trading platform — not a general-purpose code editor.)*
- **📈 [ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — hybrid crypto exchange: perpetuals, spot and margin, with demo mode.
- **[Chipa Ecosystem overview](/ecosystem)** — how these fit together with this library.
>>>>>>> 2a2cd0b1540c6ad4448423c3cceb500c348e17dc
