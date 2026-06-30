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

- **Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. These examples are provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.