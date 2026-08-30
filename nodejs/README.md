# binary-options-tools (Node.js)

Node.js bindings for **BinaryOptionsToolsV2**, built on top of the same Rust
core as the Python and UniFFI bindings. The package ships a native N-API addon
(`binary-options-tools.node`) built from `crates/bindings_napi`.

## Requirements

- Node.js 18 or newer
- The Rust toolchain ([rustup.rs](https://rustup.rs)) to build the addon

## Building

```bash
cd nodejs
npm run build          # cargo build --release, then copy the addon here
npm run build:debug    # same, but an unoptimised build
npm test               # runs the smoke tests, no credentials needed
```

`npm run build` writes `nodejs/binary-options-tools.node`. During development
`require("binary-options-tools")` also picks up `target/release` and
`target/debug` directly, so a bare `cargo build -p binary_options_tools_napi`
is enough. Set `BINARY_OPTIONS_TOOLS_NATIVE` to load an addon from an
explicit path.

## Usage

```js
const { PocketOption } = require("binary-options-tools");

async function main(ssid) {
  // The constructor returns immediately and connects in the background;
  // every method awaits that connection before running.
  const api = new PocketOption(ssid);

  console.log(`Balance: ${await api.balance()}`);

  const [dealId] = await api.buy("EURUSD_otc", 1.0, 60);
  const deal = await api.result(dealId);
  console.log(`Profit: ${deal.profit}`);
}

main(process.argv[2]).catch(console.error);
```

Use `PocketOption.create(ssid)` instead of `new` when a failed connection
should reject up front:

```js
const api = await PocketOption.create(ssid);
```

### Streaming

Subscriptions and raw message handlers are async iterables:

```js
for await (const candle of await api.subscribe("EURUSD_otc", 60)) {
  console.log(candle.close);
}
```

### Raw WebSocket access

```js
const { PocketOption, Validator } = require("binary-options-tools");

const api = await PocketOption.create(ssid);
const handler = await api.createRawHandler(Validator.contains('"status":"success"'));
console.log(await handler.sendAndWait('42["signals/subscribe"]'));
```

### Logging

```js
const { startLogs } = require("binary-options-tools");

startLogs({ path: ".", level: "DEBUG", terminal: true });
```

## Checking it from a phone

The addon only loads inside Node, so a browser cannot call it directly. `server/`
is a small dependency-free HTTP wrapper that holds the clients, plus a page that
drives them — run it on a machine on your network and open it from your phone.

```bash
cd nodejs
npm run build          # once, to compile the addon
HOST=0.0.0.0 npm run serve
# then open http://<that-machine's-ip>:8787 on your phone
```

| Variable | Default | Meaning |
|---|---|---|
| `PORT` | `8787` | port to listen on |
| `HOST` | `127.0.0.1` | bind address; use `0.0.0.0` to reach it from another device |
| `AUTH_TOKEN` | unset | when set, every `/api` call needs `?token=…` or an `X-Auth-Token` header |
| `ALLOW_TRADING` | unset | set to `1` to enable the order endpoint; without it trading is refused |
| `SESSION_TTL_MS` | `1800000` | idle time before a session is dropped and its client shut down |

The validator and introspection endpoints need no account. Everything under
`/api/session/:id` needs an ssid, which is posted once, held in the server's
memory, and never sent back to the browser.

| Endpoint | Purpose |
|---|---|
| `GET /api/health` | addon path and size, Node version, whether trading is on |
| `GET /api/surface` | every class and method the addon exports |
| `POST /api/validator` | build a validator from JSON and run it against a message |
| `POST /api/session` | connect with an ssid, returns a session id |
| `GET /api/session/:id/balance` | account balance |
| `GET /api/session/:id/payout` | payout percentage per active asset |
| `GET /api/session/:id/candles` | `?asset=&period=` |
| `GET /api/session/:id/ticks` | `?asset=&seconds=` |
| `GET /api/session/:id/deals` | open and settled deals |
| `GET /api/session/:id/stream` | server-sent events, one per candle |
| `POST /api/session/:id/trade` | place an order — refused unless `ALLOW_TRADING=1` |
| `GET /api/session/:id/result/:dealId` | wait for a trade to settle |
| `DELETE /api/session/:id` | shut the client down |

Validators are described as JSON and nest:

```json
{ "validator": { "type": "all", "of": [
    { "type": "contains", "value": "\"status\":\"success\"" },
    { "type": "not", "of": { "type": "contains", "value": "error" } }
  ] },
  "message": "..." }
```

Bind to `0.0.0.0` only on a network you trust, and set `AUTH_TOKEN` when you do:
anyone who can reach the port can use any session you have open on it.

## API notes

- Every method has both a `camelCase` and a `snake_case` spelling
  (`api.getCandles` and `api.get_candles` are the same function).
- Structured results (deals, candles, assets) are returned as plain JavaScript
  objects, not as JSON strings.
- Timeouts are expressed in milliseconds, durations in seconds, exactly as the
  parameter names say.
- Errors are regular `Error` objects whose message is prefixed with the error
  kind, for example `TimeoutError: the operation timed out`.
- `Validator.custom()` is not available: a JavaScript callback cannot be run
  synchronously on the WebSocket thread. Filter the values yielded by
  `RawHandler.subscribe()` instead.

The full typed surface lives in [`index.d.ts`](./index.d.ts).

## Examples

Runnable scripts are in [`../examples/javascript`](../examples/javascript).
