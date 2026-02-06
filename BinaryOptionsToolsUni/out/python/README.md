# BinaryOptionsTools - Python UniFFI Bindings

This is the Python binding for BinaryOptionsTools generated via UniFFI. It provides async-only access to PocketOption trading platform.

## 🚀 Features

- ✅ **Async Trading Operations**: Place buy/sell trades
- ✅ **Account Management**: Get balance and account information
- ✅ **Real-time Data**: Subscribe to asset price feeds
- ✅ **Trade Monitoring**: Check trade results and opened deals

## 📦 Installation

```bash
pip install binary-options-tools-uni
```

## 🔧 Quick Start

### Basic Example

```python
from binary_options_tools_uni import PocketOption
import asyncio

async def main():
    # Initialize client with your session ID
    client = await PocketOption.new("your-session-id")

    # IMPORTANT: Wait for connection to establish
    await asyncio.sleep(5)

    # Get account balance
    balance = await client.balance()
    print(f"Account Balance: ${balance}")

    # Place a buy trade
    deal = await client.buy("EURUSD_otc", 60, 1.0)
    print(f"Trade placed: {deal}")

    # Subscribe to real-time data
    subscription = await client.subscribe("EURUSD_otc", 60)
    # Process subscription data...

if __name__ == "__main__":
    asyncio.run(main())
```

## 📖 Detailed Examples

### Buy Trade Example

```python
from binary_options_tools_uni import PocketOption
import asyncio

async def buy_trade_example():
    # Initialize client
    client = await PocketOption.new("your-session-id")
    await asyncio.sleep(5)  # Wait for connection

    # Place a buy trade on EURUSD for 60 seconds with $1
    deal = await client.buy(
        asset="EURUSD_otc",
        time=60,
        amount=1.0
    )

    print(f"Trade placed successfully!")
    print(f"Deal data: {deal}")

asyncio.run(buy_trade_example())
```

### Sell Trade Example

```python
from binary_options_tools_uni import PocketOption
import asyncio

async def sell_trade_example():
    # Initialize client
    client = await PocketOption.new("your-session-id")
    await asyncio.sleep(5)  # Wait for connection

    # Place a sell trade on EURUSD for 60 seconds with $1
    deal = await client.sell(
        asset="EURUSD_otc",
        time=60,
        amount=1.0
    )

    print(f"Trade placed successfully!")
    print(f"Deal data: {deal}")

asyncio.run(sell_trade_example())
```

### Check Balance Example

```python
from binary_options_tools_uni import PocketOption
import asyncio

async def balance_example():
    # Initialize client
    client = await PocketOption.new("your-session-id")
    await asyncio.sleep(5)  # Wait for connection

    # Get current balance
    balance = await client.balance()
    print(f"Your current balance is: ${balance}")

asyncio.run(balance_example())
```

### Check Trade Result Example

```python
from binary_options_tools_uni import PocketOption
import asyncio

async def check_win_example():
    # Initialize client
    client = await PocketOption.new("your-session-id")
    await asyncio.sleep(5)  # Wait for connection

    # Place a trade
    deal = await client.buy("EURUSD_otc", 60, 1.0)
    trade_id = deal.id  # Extract trade ID from deal

    # Wait for trade to complete
    await asyncio.sleep(65)

    # Check the result
    result = await client.check_win(trade_id)
    print(f"Trade result: {result}")

asyncio.run(check_win_example())
```

### Subscribe to Real-time Data

```python
from binary_options_tools_uni import PocketOption
import asyncio

async def subscribe_example():
    # Initialize client
    client = await PocketOption.new("your-session-id")
    await asyncio.sleep(5)  # Wait for connection

    # Subscribe to real-time candle data for EURUSD
    # Duration in seconds for each candle
    subscription = await client.subscribe("EURUSD_otc", duration_secs=60)

    print("Listening for real-time candles...")
    # Process subscription stream
    # (Implementation depends on the SubscriptionStream interface)

asyncio.run(subscribe_example())
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```python
client = await PocketOption.new("your-session-id")
await asyncio.sleep(5)  # Critical!
```

### Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value

### Supported Assets

Common assets include:

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## 🆚 Differences from Main Python Package

This UniFFI binding differs from the main `BinaryOptionsToolsV2` package:

- **Async Only**: Only async/await syntax is supported (no sync wrapper)
- **API Surface**: May have slightly different method signatures
- **Use Case**: Prefer the main package (`BinaryOptionsToolsV2`) for production use

## 📚 Additional Resources

- **Main Python Package**: [BinaryOptionsToolsV2](../../BinaryOptionsToolsV2/Readme.md)
- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
