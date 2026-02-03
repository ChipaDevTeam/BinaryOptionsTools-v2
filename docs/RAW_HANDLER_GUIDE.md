# Raw Handler & Validator Examples

This document shows how to use the raw handler and validator features in BinaryOptionsToolsUni.

## Table of Contents
- [Validator Examples](#validator-examples)
- [Raw Handler Examples](#raw-handler-examples)
- [Advanced Patterns](#advanced-patterns)

---

## Validator Examples

### Basic Validators

<details>
<summary><b>Python</b></summary>

```python
import asyncio
from binaryoptionstoolsuni import PocketOption, Validator

async def main():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)
    
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
    
    await client.shutdown()

asyncio.run(main())
```
</details>

<details>
<summary><b>Kotlin</b></summary>

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*
import kotlinx.coroutines.*

suspend fun main() = coroutineScope {
    val client = PocketOption.init("your_ssid")
    delay(2000)
    
    // Starts with validator
    val v1 = Validator.startsWith("42[")
    assert(v1.check("42[\"balance\"]"))
    assert(!v1.check("43[\"balance\"]"))
    
    // Contains validator
    val v2 = Validator.contains("balance")
    assert(v2.check("{\"balance\": 100}"))
    assert(!v2.check("{\"amount\": 50}"))
    
    // Regex validator
    val v3 = Validator.regex("^\\d+")
    assert(v3.check("123 message"))
    assert(!v3.check("abc"))
    
    client.shutdown()
}
```
</details>

<details>
<summary><b>Swift</b></summary>

```swift
import BinaryOptionsToolsUni

Task {
    let client = try await PocketOption.init(ssid: "your_ssid")
    try await Task.sleep(nanoseconds: 2_000_000_000)
    
    // Starts with validator
    let v1 = Validator.startsWith(prefix: "42[")
    assert(v1.check(message: "42[\"balance\"]"))
    assert(!v1.check(message: "43[\"balance\"]"))
    
    // Contains validator
    let v2 = Validator.contains(substring: "balance")
    assert(v2.check(message: "{\"balance\": 100}"))
    assert(!v2.check(message: "{\"amount\": 50}"))
    
    // Regex validator
    let v3 = try Validator.regex(pattern: "^\\d+")
    assert(v3.check(message: "123 message"))
    assert(!v3.check(message: "abc"))
    
    try await client.shutdown()
}
```
</details>

### Combined Validators

<details>
<summary><b>Python</b></summary>

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
</details>

<details>
<summary><b>Kotlin</b></summary>

```kotlin
// ALL: Must satisfy all conditions
val vAll = Validator.all(listOf(
    Validator.startsWith("42["),
    Validator.contains("balance")
))
assert(vAll.check("42[\"balance\"]"))
assert(!vAll.check("42[\"amount\"]"))

// ANY: Must satisfy at least one condition
val vAny = Validator.any(listOf(
    Validator.contains("success"),
    Validator.contains("completed")
))
assert(vAny.check("operation successful"))
assert(vAny.check("task completed"))
assert(!vAny.check("in progress"))

// NOT: Negates validator
val vNot = Validator.ne(Validator.contains("error"))
assert(vNot.check("success message"))
assert(!vNot.check("error occurred"))
```
</details>

---

## Raw Handler Examples

### Basic Usage

<details>
<summary><b>Python</b></summary>

```python
import asyncio
import json
from binaryoptionstoolsuni import PocketOption, Validator

async def main():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)
    
    # Create validator for balance messages
    validator = Validator.contains('"balance"')
    
    # Create raw handler
    handler = await client.create_raw_handler(validator, None)
    
    # Send custom message
    await handler.send_text('42["getBalance"]')
    
    # Wait for response
    response = await handler.wait_next()
    data = json.loads(response)
    print(f"Balance: {data['balance']}")
    
    await client.shutdown()

asyncio.run(main())
```
</details>

<details>
<summary><b>Kotlin</b></summary>

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*
import kotlinx.coroutines.*
import kotlinx.serialization.json.*

suspend fun main() = coroutineScope {
    val client = PocketOption.init("your_ssid")
    delay(2000)
    
    // Create validator for balance messages
    val validator = Validator.contains("\"balance\"")
    
    // Create raw handler
    val handler = client.createRawHandler(validator, null)
    
    // Send custom message
    handler.sendText("42[\"getBalance\"]")
    
    // Wait for response
    val response = handler.waitNext()
    val data = Json.parseToJsonElement(response)
    println("Balance: ${data.jsonObject["balance"]}")
    
    client.shutdown()
}
```
</details>

<details>
<summary><b>Swift</b></summary>

```swift
import BinaryOptionsToolsUni
import Foundation

Task {
    let client = try await PocketOption.init(ssid: "your_ssid")
    try await Task.sleep(nanoseconds: 2_000_000_000)
    
    // Create validator for balance messages
    let validator = Validator.contains(substring: "\"balance\"")
    
    // Create raw handler
    let handler = try await client.createRawHandler(validator: validator, keepAlive: nil)
    
    // Send custom message
    try await handler.sendText(message: "42[\"getBalance\"]")
    
    // Wait for response
    let response = try await handler.waitNext()
    if let data = response.data(using: .utf8),
       let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
        print("Balance: \(json["balance"] ?? "unknown")")
    }
    
    try await client.shutdown()
}
```
</details>

### Send and Wait Pattern

<details>
<summary><b>Python</b></summary>

```python
# Send a message and wait for response in one call
response = await handler.send_and_wait('42["getServerTime"]')
data = json.loads(response)
print(f"Server time: {data['time']}")
```
</details>

<details>
<summary><b>Kotlin</b></summary>

```kotlin
// Send a message and wait for response in one call
val response = handler.sendAndWait("42[\"getServerTime\"]")
val data = Json.parseToJsonElement(response)
println("Server time: ${data.jsonObject["time"]}")
```
</details>

### With Keep-Alive

<details>
<summary><b>Python</b></summary>

```python
# Create handler with keep-alive message
# This message will be sent automatically on reconnect
keep_alive = '42["subscribe",{"asset":"EURUSD_otc"}]'
handler = await client.create_raw_handler(validator, keep_alive)
```
</details>

<details>
<summary><b>Kotlin</b></summary>

```kotlin
// Create handler with keep-alive message
// This message will be sent automatically on reconnect
val keepAlive = "42[\"subscribe\",{\"asset\":\"EURUSD_otc\"}]"
val handler = client.createRawHandler(validator, keepAlive)
```
</details>

---

## Advanced Patterns

### Custom Protocol Implementation

<details>
<summary><b>Python</b></summary>

```python
import asyncio
import json
from binaryoptionstoolsuni import PocketOption, Validator

class CustomProtocol:
    def __init__(self, client):
        self.client = client
        self.handlers = {}
    
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
        
        self.handlers['trades'] = handler
        return handler
    
    async def get_custom_data(self, data_type):
        """Request custom data."""
        validator = Validator.contains(f'"{data_type}"')
        handler = await self.client.create_raw_handler(validator, None)
        
        message = f'42["getData","{data_type}"]'
        response = await handler.send_and_wait(message)
        
        return json.loads(response)

async def main():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)
    
    protocol = CustomProtocol(client)
    
    # Subscribe to trades
    trade_handler = await protocol.subscribe_to_trades()
    
    # Listen for trade updates
    for _ in range(5):
        update = await trade_handler.wait_next()
        print(f"Trade update: {update}")
    
    # Get custom data
    data = await protocol.get_custom_data("statistics")
    print(f"Statistics: {data}")
    
    await client.shutdown()

asyncio.run(main())
```
</details>

### Multiple Handlers

<details>
<summary><b>Python</b></summary>

```python
async def monitor_multiple_events():
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)
    
    # Handler for balance updates
    balance_validator = Validator.contains("balance")
    balance_handler = await client.create_raw_handler(balance_validator, None)
    
    # Handler for trade updates
    trade_validator = Validator.contains("trade")
    trade_handler = await client.create_raw_handler(trade_validator, None)
    
    # Handler for errors
    error_validator = Validator.contains("error")
    error_handler = await client.create_raw_handler(error_validator, None)
    
    # Monitor all handlers concurrently
    async def monitor_balance():
        while True:
            msg = await balance_handler.wait_next()
            print(f"Balance: {msg}")
    
    async def monitor_trades():
        while True:
            msg = await trade_handler.wait_next()
            print(f"Trade: {msg}")
    
    async def monitor_errors():
        while True:
            msg = await error_handler.wait_next()
            print(f"ERROR: {msg}")
    
    # Run all monitors concurrently
    await asyncio.gather(
        monitor_balance(),
        monitor_trades(),
        monitor_errors()
    )
```
</details>

### Binary Message Handling

<details>
<summary><b>Python</b></summary>

```python
# Send binary data
binary_data = b'\x00\x01\x02\x03\x04'
await handler.send_binary(binary_data)

# Receive binary data (converted to string)
response = await handler.wait_next()
# Response is automatically converted to string representation
```
</details>

### Filtering Complex Messages

<details>
<summary><b>Python</b></summary>

```python
# Match messages with multiple conditions
validator = Validator.all([
    Validator.starts_with("42["),  # Socket.IO format
    Validator.contains('"type":"candle"'),  # Must be candle update
    Validator.regex(r'"asset":"EURUSD'),  # Must be EURUSD asset
    Validator.ne(Validator.contains("error"))  # Must not contain error
])

handler = await client.create_raw_handler(validator, None)

# This will only receive candle updates for EURUSD without errors
while True:
    candle_msg = await handler.wait_next()
    data = json.loads(candle_msg)
    print(f"EURUSD Candle: {data}")
```
</details>

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
    Validator.contains("balance"), None
)

trade_handler = await client.create_raw_handler(
    Validator.contains("trade"), None
)

# Each handler only receives relevant messages
```

### 4. Error Handling

```python
try:
    handler = await client.create_raw_handler(validator, None)
    response = await handler.wait_next()
except UniError as e:
    print(f"Error: {e}")
```

---

## Comparison with Python Version

The UniFFI version provides the same functionality as the Python version but with multi-language support:

| Feature | Python API | UniFFI API | Notes |
|---------|-----------|------------|-------|
| Basic validators | ✅ | ✅ | Same API |
| Combined validators | ✅ | ✅ | Same API |
| Custom validators | ✅ | ❌ | Not supported in UniFFI (requires FFI callbacks) |
| Raw handlers | ✅ | ✅ | Same API |
| Keep-alive | ✅ | ✅ | Same API |
| Send/receive | ✅ | ✅ | Same API |

**Note**: Custom validators (using Python functions) are not supported in the UniFFI version because they require calling across the FFI boundary, which is complex and not currently supported by UniFFI.

---

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report bugs](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)
- **Documentation**: [Full docs](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
