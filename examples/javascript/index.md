# JavaScript Examples

Runnable scripts for the Node.js bindings.

## Setup

```bash
cd ../../nodejs
npm run build     # builds the native addon with cargo
```

Every script except `validator.js` needs a PocketOption session id, passed as
the first argument or through the `POCKET_OPTION_SSID` environment variable:

```bash
node balance.js "<ssid>"
POCKET_OPTION_SSID="<ssid>" node balance.js
```

## Examples

- `balance.js`: Get account balance.
- `basic.js`: Initialize client and get balance.
- `buy.js`: Place a buy trade.
- `check_win.js`: Open a trade and wait for its result.
- `create_raw_iterator.js`: Iterate over raw responses matching a validator.
- `create_raw_order.js`: Send raw messages and wait for matching responses.
- `get_candles.js`: Get candle data for a symbol.
- `history.js`: Get candle history.
- `logs.js`: Write library logs to disk and to the terminal.
- `payout.js`: Get payout information.
- `raw_send.js`: Send raw messages to the server.
- `sell.js`: Place a sell trade.
- `subscribe_symbol.js`: Subscribe to real-time data for a symbol.
- `validator.js`: Compose message validators (no connection needed).
