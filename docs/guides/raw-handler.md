# Raw Handler & Validator Examples

This document shows how to use the raw handler and validator features in `BinaryOptionsToolsV2`.

## Table of Contents

- [Validator Examples](#validator-examples)
- [Raw Handler Examples](#raw-handler-examples)
- [Advanced Patterns](#advanced-patterns)

---

## Validator Examples

### Basic Validators

```python
import asyncio
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

async def main():
    async with PocketOptionAsync(ssid="your_ssid") as client:
        # Starts with validator
        v1 = Validator.starts_with("42[")
        assert v1.check('42["balance"]') == True
        assert v1.check('43["balance"]') == False

        # Contains validator
        v2 = Validator.contains("balance")
        assert v2.check('{"balance": 100}') == True
        assert v2.check('{"amount": 50}') == False

        # Regex validator
        v3 = Validator.regex(r"^\d+")
        assert v3.check("123 message") == True
        assert v3.check("abc") == False

asyncio.run(main())
```

### Combined Validators

```python
# ALL: Must satisfy all conditions
v_all = Validator.all([
    Validator.starts_with("42["),
    Validator.contains("balance")
])
assert v_all.check('42["balance"]') == True
assert v_all.check('42["amount"]') == False

# ANY: Must satisfy at least one condition
v_any = Validator.any([
    Validator.contains("success"),
    Validator.contains("completed")
])
assert v_any.check("operation successful") == True
assert v_any.check("task completed") == True
assert v_any.check("in progress") == False

# NOT: Negates validator
v_not = Validator.ne(Validator.contains("error"))
assert v_not.check("success message") == True
assert v_not.check("error occurred") == False
```

---

## Raw Handler Examples

### Basic Usage

```python
import asyncio
import json
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

async def main():
    async with PocketOptionAsync(ssid="your_ssid") as client:
        # Create validator for balance messages
        validator = Validator.contains('"balance"')

        # Create raw handler
        handler = await client.create_raw_handler(validator)

        # Send custom message
        await handler.send_text('42["getBalance"]')

        # Wait for response
        response = await handler.wait_next()
        data = json.loads(response)
        print(f"Balance: {data['balance']}")

asyncio.run(main())
```

### Send and Wait Pattern

```python
# Send a message and wait for response in one call
response = await handler.send_and_wait('42["getServerTime"]')
data = json.loads(response)
print(f"Server time: {data['time']}")
```

### With Keep-Alive

```python
# Create handler with keep-alive message
# This message will be sent automatically on reconnect
keep_alive = '42["subscribe",{"asset":"EURUSD_otc"}]'
handler = await client.create_raw_handler(validator, keep_alive)
```

---

## Advanced Patterns

### Custom Protocol Implementation

```python
import asyncio
import json
from BinaryOptionsToolsV2 import PocketOptionAsync, Validator

class CustomProtocol:
    def __init__(self, client):
        self.client = client

    async def subscribe_to_trades(self):
        """Subscribe to trade updates."""
        validator = Validator.all([
            Validator.starts_with("42["),
            Validator.contains("trade")
        ])

        handler = await self.client.create_raw_handler(
            validator,
            '42["subscribe","trades"]'
        )
        return handler

    async def get_custom_data(self, data_type):
        """Request custom data."""
        validator = Validator.contains(f'"{data_type}"')
        handler = await self.client.create_raw_handler(validator)

        message = f'42["getData","{data_type}"]'
        response = await handler.send_and_wait(message)

        return json.loads(response)

async def main():
    async with PocketOptionAsync(ssid="your_ssid") as client:
        protocol = CustomProtocol(client)

        # Subscribe to trades
        trade_handler = await protocol.subscribe_to_trades()

        # Listen for trade updates in background or loop
        # async for msg in trade_handler.subscribe(): ...

        # Get custom data
        try:
            data = await protocol.get_custom_data("statistics")
            print(f"Statistics: {data}")
        except Exception as e:
            print(f"Failed to get data: {e}")

asyncio.run(main())
```

### Custom Python Validators

`BinaryOptionsToolsV2` supports custom Python functions as validators.

> **Warning**: The function must be synchronous, accept one string argument, and return a boolean. It runs on the Rust thread, so keep it fast. Exceptions are swallowed (validator returns False).

```python
def my_custom_check(msg: str) -> bool:
    return "secret_token" in msg and len(msg) < 100

validator = Validator.custom(my_custom_check)
handler = await client.create_raw_handler(validator)
```

### Binary Message Handling

```python
# Send binary data
binary_data = b'\x00\x01\x02\x03\x04'
await handler.send_binary(binary_data)

# Receive binary data (automatically converted to string representation by the library)
response = await handler.wait_next()
```

---

## Best Practices

### 1. Use Specific Validators

```python
# ❌ Too broad - matches too many messages
validator = Validator.contains("data")

# ✅ More specific - matches only what you need
validator = Validator.all([
    Validator.starts_with("42["),
    Validator.contains('"type":"balance"')
])
```

### 2. Keep-Alive for Subscriptions

```python
# ✅ Use keep-alive for subscriptions that need to persist on reconnect
validator = Validator.contains('"candles"')
keep_alive = '42["subscribe",{"asset":"EURUSD_otc","period":60}]'
handler = await client.create_raw_handler(validator, keep_alive)
```

### 3. Multiple Handlers for Different Message Types

```python
# ✅ Separate handlers for different concerns
balance_handler = await client.create_raw_handler(
    Validator.contains("balance")
)

trade_handler = await client.create_raw_handler(
    Validator.contains("trade")
)
```

---

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report bugs](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- **Documentation**: [Full docs](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
