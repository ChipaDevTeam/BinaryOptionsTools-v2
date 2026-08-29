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
