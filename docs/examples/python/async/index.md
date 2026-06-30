---
sidebar_position: 1
slug: /examples/python/async
---

# Python Async Examples

This directory contains asynchronous examples using `BinaryOptionsToolsV2`.

## Examples

### Basic Example

**File**: `get_balance.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    async with PocketOptionAsync(ssid) as api:
        balance = await api.balance()
        print(f"Balance: {balance}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Check Trade Result

**File**: `check_win.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        # Place a trade first
        trade = await api.buy("EURUSD_otc", 60, 1.0)
        print(f"Trade placed: {trade.id}")

        # Wait for trade to complete
        await asyncio.sleep(65)

        # Check result
        result = await api.check_win(trade.id)
        if result.profit > 0:
            print(f"WIN! Profit: ${result.profit:.2f}")
        else:
            print(f"LOSS! Loss: ${abs(result.profit):.2f}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Get Historical Candles

**File**: `get_candles.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        # Get last 100 candles with 60-second period
        candles = await api.get_candles("EURUSD_otc", 60, 100)
        print(f"Retrieved {len(candles)} candles")

        for candle in candles[:5]:  # Show first 5
            print(f"  Time: {candle.time}, Close: {candle.close}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Subscribe to Real-time Data

**File**: `subscribe_symbol.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        # Subscribe to 60-second candles
        subscription = await api.subscribe("EURUSD_otc", 60)

        print("Subscribed to EURUSD_otc")

        # Iterate over candles (async iterator)
        async for candle in subscription:
            print(f"Candle: {candle}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Place a Trade

**File**: `trade.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        # Check if demo account
        if not api.is_demo():
            print("⚠️ WARNING: Using REAL account!")
            return

        # Place a CALL (buy) trade
        trade = await api.buy("EURUSD_otc", 60, 1.0)
        print(f"Trade placed! ID: {trade.id}")
        print(f"Asset: {trade.asset}")
        print(f"Amount: ${trade.amount}")
        print(f"Time: {trade.time} seconds")

        # Wait for result
        await asyncio.sleep(65)

        # Check result
        result = await api.check_win(trade.id)
        if result.profit > 0:
            print(f"✅ WIN! Profit: ${result.profit:.2f}")
        else:
            print(f"❌ LOSS! Loss: ${abs(result.profit):.2f}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Raw Handler Usage

**File**: `raw_send.py`

```python
import asyncio
import json
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
from BinaryOptionsToolsV2.validator import Validator


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


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Get Payout Information

**File**: `payout.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        # Get payout for asset
        payout = await api.payout("EURUSD_otc")
        print(f"Payout for EURUSD_otc: {payout * 100}%")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Login with Email and Password

**File**: `login_with_email_and_password.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption.tools.login import login


async def main(email: str, password: str):
    # Login to get SSID
    ssid = await login(email, password)
    print(f"SSID: {ssid}")

    # Use SSID with client
    from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
    async with PocketOptionAsync(ssid) as api:
        balance = await api.balance()
        print(f"Balance: {balance}")


if __name__ == "__main__":
    email = input("Email: ")
    password = input("Password: ")
    asyncio.run(main(email, password))
```

### Comprehensive Demo

**File**: `comprehensive_demo.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        print("=== BinaryOptionsToolsV2 Comprehensive Demo ===\n")

        # 1. Check account type
        is_demo = api.is_demo()
        print(f"Account type: {'Demo' if is_demo else 'Real'}")

        # 2. Get balance
        balance = await api.balance()
        print(f"Balance: ${balance:.2f}")

        # 3. Get server time
        server_time = await api.get_server_time()
        print(f"Server time: {server_time}")

        # 4. Get open trades
        open_trades = await api.get_open_and_close_trades()
        print(f"Open trades: {len(open_trades)}")

        # 5. Get historical candles
        candles = await api.get_candles("EURUSD_otc", 60, 10)
        print(f"Recent candles: {len(candles)}")

        # 6. Get payout
        payout = await api.payout("EURUSD_otc")
        print(f"Payout: {payout * 100}%")

        # 7. Place a test trade (only on demo)
        if is_demo:
            trade = await api.buy("EURUSD_otc", 60, 1.0)
            print(f"Test trade placed: {trade.id}")

            # Wait for result
            await asyncio.sleep(65)
            result = await api.check_win(trade.id)
            status = "WIN" if result.profit > 0 else "LOSS"
            print(f"Result: {status} (${result.profit:.2f})")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

### Trading Strategy Example

**File**: `strategy_example.py`

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


class SimpleStrategy:
    def __init__(self, client):
        self.client = client
        self.balance = 0
        self.trades = []

    async def analyze(self, asset: str, period: int):
        """Simple momentum strategy"""
        candles = await self.client.get_candles(asset, period, 10)

        if len(candles) < 2:
            return None

        # Simple: if last close > previous close, buy
        if candles[-1].close > candles[-2].close:
            return "buy"
        else:
            return "sell"

    async def execute(self, asset: str, action: str, period: int, amount: float):
        if action == "buy":
            trade = await self.client.buy(asset, period, amount)
        else:
            trade = await self.client.sell(asset, period, amount)

        self.trades.append(trade)
        return trade

    async def run(self, asset: str = "EURUSD_otc", period: int = 60, amount: float = 1.0):
        print(f"Running strategy on {asset}...")

        # Get initial balance
        self.balance = await self.client.balance()
        print(f"Starting balance: ${self.balance:.2f}")

        # Analyze
        action = await self.analyze(asset, period)
        if not action:
            print("Insufficient data")
            return

        print(f"Signal: {action.upper()}")

        # Execute trade
        trade = await self.execute(asset, action, period, amount)
        print(f"Trade placed: {trade.id}")

        # Wait and check
        await asyncio.sleep(period + 5)
        result = await self.client.check_win(trade.id)

        if result.profit > 0:
            print(f"✅ WIN! Profit: ${result.profit:.2f}")
        else:
            print(f"❌ LOSS! Loss: ${abs(result.profit):.2f}")

        final_balance = await self.client.balance()
        print(f"Final balance: ${final_balance:.2f}")


async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        strategy = SimpleStrategy(api)
        await strategy.run()


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
```

## Running Examples

```bash
cd examples/python/async
python get_balance.py
python trade.py
python get_candles.py
python subscribe_symbol.py
python raw_send.py
python payout.py
python login_with_email_and_password.py
python comprehensive_demo.py
python strategy_example.py
```

## Key Concepts

### Context Manager
All examples use the async context manager pattern:
```python
async with PocketOptionAsync(ssid) as api:
    # API is automatically connected and cleaned up
```

### Initialization Wait
Always wait ~2 seconds after creating the client:
```python
async with PocketOptionAsync(ssid) as api:
    await asyncio.sleep(2)  # Critical for connection!
```

### Demo vs Real Account
```python
if not api.is_demo():
    print("WARNING: Using REAL account!")
```

### Proper Cleanup
The context manager handles cleanup automatically, but you can also call:
```python
await api.shutdown()
```