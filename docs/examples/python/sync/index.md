---
sidebar_position: 2
slug: /examples/python/sync
---

# Python Sync Examples

This directory contains synchronous examples using `BinaryOptionsToolsV2`.

## Examples

### Basic Example

**File**: `get_balance.py`

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    with PocketOption(ssid) as api:
        balance = api.balance()
        print(f"Balance: {balance}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Check Trade Result

**File**: `check_win.py`

```python
import time
from BinaryOptionsToolsV2.pocketoption import PocketOption


def main(ssid: str):
    with PocketOption(ssid) as api:
        # Place a trade first
        buy_id, buy = api.buy(
            asset="EURUSD_otc", amount=1.0, time=60, check_win=False
        )
        print(f"Trade placed: {buy_id}")

        # Wait for trade to complete
        time.sleep(65)

        # Check result
        result = api.check_win(buy_id)
        if result.get("profit", 0) > 0:
            print(f"WIN! Profit: ${result['profit']:.2f}")
        else:
            print(f"LOSS! Loss: ${abs(result.get('profit', 0)):.2f}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Get Historical Candles

**File**: `get_candles.py`

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption


def main(ssid: str):
    with PocketOption(ssid) as api:
        # Get last 100 candles with 60-second period
        candles = api.get_candles("EURUSD_otc", 60, 100)
        print(f"Retrieved {len(candles)} candles")

        for candle in candles[:5]:  # Show first 5
            print(f"  Time: {candle.get('time')}, Close: {candle.get('close')}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Subscribe to Real-time Data

**File**: `subscribe_symbol.py`

```python
import time
from BinaryOptionsToolsV2.pocketoption import PocketOption


def main(ssid: str):
    with PocketOption(ssid) as api:
        # Subscribe to 60-second candles
        subscription = api.subscribe("EURUSD_otc", 60)

        print("Subscribed to EURUSD_otc")

        # Iterate over candles (iterator)
        for candle in subscription:
            print(f"Candle: {candle}")
            time.sleep(1)  # Process candles


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Place a Trade

**File**: `trade.py`

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    with PocketOption(ssid) as api:
        (buy_id, buy) = api.buy(
            asset="EURUSD_otc", amount=1.0, time=60, check_win=False
        )
        print(f"Buy trade id: {buy_id}\nBuy trade data: {buy}")
        (sell_id, sell) = api.sell(
            asset="EURUSD_otc", amount=1.0, time=60, check_win=False
        )
        print(f"Sell trade id: {sell_id}\nSell trade data: {sell}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Raw Handler Usage

**File**: `raw_send.py`

```python
import json
from BinaryOptionsToolsV2.pocketoption import PocketOption
from BinaryOptionsToolsV2.validator import Validator


def main(ssid: str):
    with PocketOption(ssid) as api:
        # Create validator for balance messages
        validator = Validator.contains('"balance"')

        # Create raw handler
        handler = api.create_raw_handler(validator)

        # Send custom message
        handler.send_text('42["getBalance"]')

        # Wait for response
        response = handler.wait_next()
        data = json.loads(response)
        print(f"Balance: {data.get('balance', 'N/A')}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Get Payout Information

**File**: `payout.py`

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption


def main(ssid: str):
    with PocketOption(ssid) as api:
        # Get payout for asset
        payout = api.payout("EURUSD_otc")
        print(f"Payout for EURUSD_otc: {payout * 100}%")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
```

### Login with Email and Password

**File**: `login_with_email_and_password.py`

```python
from BinaryOptionsToolsV2.pocketoption.tools.login import login
from BinaryOptionsToolsV2.pocketoption import PocketOption


def main(email: str, password: str):
    # Login to get SSID
    ssid = login(email, password)
    print(f"SSID: {ssid}")

    # Use SSID with client
    with PocketOption(ssid) as api:
        balance = api.balance()
        print(f"Balance: {balance}")


if __name__ == "__main__":
    email = input("Email: ")
    password = input("Password: ")
    main(email, password)
```

### Comprehensive Demo

**File**: `comprehensive_demo.py` (async version available)

```python
# Note: Comprehensive demo is available in async examples
# See examples/python/async/comprehensive_demo.py
```

### Trading Strategy Example

**File**: `strategy_example.py` (async version available)

```python
# Note: Strategy example is available in async examples
# See examples/python/async/strategy_example.py
```

## Running Examples

```bash
cd examples/python/sync
python get_balance.py
python trade.py
python get_candles.py
python subscribe_symbol.py
python raw_send.py
python payout.py
python login_with_email_and_password.py
```

## Key Concepts

### Context Manager
All examples use the context manager pattern:
```python
with PocketOption(ssid) as api:
    # API is automatically connected and cleaned up
```

### Initialization Wait
The synchronous client handles initialization internally, but you can add a small delay:
```python
with PocketOption(ssid) as api:
    import time
    time.sleep(2)  # Optional, for stability
```

### Demo vs Real Account
```python
if not api.is_demo():
    print("WARNING: Using REAL account!")
```

### Proper Cleanup
The context manager handles cleanup automatically, but you can also call:
```python
api.shutdown()
```

## Sync vs Async

| Feature | Sync | Async |
|---------|------|-------|
| Syntax | `with PocketOption(ssid)` | `async with PocketOptionAsync(ssid)` |
| Methods | `api.balance()` | `await api.balance()` |
| Sleep | `time.sleep()` | `await asyncio.sleep()` |
| Iteration | `for candle in subscription` | `async for candle in subscription` |

Use **async** for:
- High-frequency trading
- Multiple concurrent operations
- Better performance with many subscriptions

Use **sync** for:
- Simple scripts
- Single operations
- Easier debugging