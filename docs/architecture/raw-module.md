---
sidebar_position: 3
slug: /architecture/raw-module
---

# Raw Module Architecture and Usage

This document explains the design of the Raw module: a flexible, validator-driven pipeline that lets you build features not covered by the built-in API (e.g., custom signals) while reusing the WebSocket connection, reconnection, and keep-alive logic.

## Overview

- Platform (PocketOption client) → Create handler for a specific validator → Handler interacts with Raw module to send/receive.
- You define a `Validator` that decides which incoming WS messages you care about.
- The Raw module routes matching messages into a per-validator stream.
- Handlers can send text/binary messages and optionally define a keep-alive message resent on reconnect.
- Dropping a handler removes its validator and stream automatically.

## Components

- **Validator**: enum + trait; runs on `&str` built from WS message content.
- **RawApiModule**: ApiModule that maintains a map of validators and their streams.
- **RawHandle**: top-level handle obtained from `PocketOption` to create/remove handlers.
- **RawHandler**: per-validator handle to send/receive and subscribe to matching messages.

## Message Flow

```mermaid
flowchart LR
    WS[(WebSocket)] --> Router
    subgraph Client
      Router -->|rule: RawRule| RawModule
      RawModule -->|match by Validator| Streams
    end
    Streams --> UserCode
```

- Router forwards only messages for which at least one registered validator returns true.
- RawModule fans out each message to all matching validator streams.

## Lifecycle

```mermaid
sequenceDiagram
    participant User as User Code
    participant PO as PocketOption
    participant RAW as RawApiModule
    participant WS as WebSocket

    User->>PO: raw_handle()
    PO-->>User: RawHandle
    User->>RAW: create(validator, keep_alive)
    RAW->>RAW: register validator + stream
    RAW-->>User: RawHandler (id, receiver)
    User->>WS: send (via RawHandler)
    WS-->>RAW: messages
    RAW->>User: route to stream if validator matches
    User--xRAW: drop RawHandler
    RAW->>RAW: remove validator + stream
```

## API Sketch

### PocketOption
- `raw_handle()` → `RawHandle`
- `create_raw_handler(validator, keep_alive)` → `RawHandler`

### RawHandle
- `create(validator, keep_alive)` → `RawHandler`
- `remove(id)` → `bool`

### RawHandler
- `id()` → `Uuid`
- `send_text(text)`
- `send_binary(bytes)`
- `send_and_wait(msg)` → next matching `Message`
- `wait_next()` → next matching `Message`
- `subscribe()` → `AsyncReceiver<Message>`
- **Drop**: auto-remove validator and stream

## Keep-Alive on Reconnect

If a handler is created with a `keep_alive` message, the module will re-send it after reconnects so servers maintain your subscription.

## Notes

- Validators are stored by UUID; you can remove them explicitly or by dropping their handler.
- Incoming messages are transformed to String for validation; original Message (text/binary) is delivered to the stream.
- The module is best-effort for fan-out; if a user stream is closed, the send is ignored.

## Example (Rust)

```rust
use binary_options_tools_pocketoption::{PocketOption};
use binary_options_tools_pocketoption::validator::Validator;
use binary_options_tools_pocketoption::pocketoption::modules::raw::Outgoing;

async fn demo(ssid: &str) -> anyhow::Result<()> {
    let api = PocketOption::new(ssid).await?;
    let validator = Validator::contains("updateStream".to_string());
    let handler = api
        .create_raw_handler(validator, Some(Outgoing::Text("42[\"ping\"]".into())))
        .await?;

    handler.send_text("42[\"hello\"]").await?;
    let msg = handler.wait_next().await?; // next matching Message
    println!("got: {:?}", msg);
    Ok(())
}
```

## Example (Python)

```python
import asyncio
import json
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        # Create validator for balance messages
        validator = Validator.contains('"balance"')

        # Create raw handler
        handler = await api.create_raw_handler(validator)

        # Send custom message
        await handler.send_text('42["getBalance"]')

        # Wait for response
        response = await handler.wait_next()
        data = json.loads(response)
        print(f"Balance: {data.get('balance', 'N/A')}")

asyncio.run(main("your-ssid"))
```

## Validator Types

### Basic Validators
- `starts_with(prefix)` - Check if message starts with prefix
- `ends_with(suffix)` - Check if message ends with suffix
- `contains(substring)` - Check if message contains substring
- `regex(pattern)` - Match against regex pattern

### Logical Combinators
- `ne(validator)` - Negate a validator (NOT)
- `all(validators)` - All validators must match (AND)
- `any(validators)` - At least one validator must match (OR)

### Instance Method
- `check(message)` - Test if message matches validator

## Use Cases

### 1. Custom Message Monitoring
```python
validator = Validator.all([
    Validator.starts_with("42["),
    Validator.contains('"type":"candle"')
])
handler = await client.create_raw_handler(validator, None)
```

### 2. Low-Level Protocol Implementation
```python
async def send_custom_command(handler, command, args):
    message = json.dumps([command, args])
    response = await handler.send_and_wait(message)
    return json.loads(response)
```

### 3. Debugging and Logging
```python
error_validator = Validator.contains("error")
error_handler = await client.create_raw_handler(error_validator, None)

while True:
    error_msg = await error_handler.wait_next()
    print(f"ERROR: {error_msg}")
```

### 4. Multiple Subscriptions
```python
balance_handler = await client.create_raw_handler(
    Validator.contains("balance"), None
)
trade_handler = await client.create_raw_handler(
    Validator.contains("trade"), None
)
```

## Architecture

```
┌─────────────────────────────────────────────┐
│         BinaryOptionsToolsUni               │
│                                             │
│  ┌──────────────┐      ┌────────────────┐ │
│  │  Validator   │      │  RawHandler    │ │
│  │              │      │                │ │
│  │ • starts_with│      │ • send_text    │ │
│  │ • contains   │      │ • send_binary  │ │
│  │ • regex      │      │ • wait_next    │ │
│  │ • all/any/ne │      │ • send_and_wait│ │
│  └──────┬───────┘      └────────┬───────┘ │
│         │                       │          │
│         └───────────┬───────────┘          │
│                     │                      │
│         ┌───────────▼────────────┐         │
│         │   PocketOption Client  │         │
│         │                        │         │
│         │ • create_raw_handler() │         │
│         │ • payout()             │         │
│         └────────────────────────┘         │
│                                             │
└─────────────────────────────────────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │  binary_options_tools  │
        │  (Rust Core Library)   │
        └────────────────────────┘
```