# BinaryOptionsToolsV2 - Python Package

[![Discord](https://img.shields.io/discord/your-discord-id?color=7289da&label=Discord&logo=discord&logoColor=white)](https://discord.gg/T3FGXcmd)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://pypi.org/project/binaryoptionstoolsv2/)

Python bindings for BinaryOptionsTools - A powerful library for automated binary options trading on PocketOption platform.

## Current Status

**Available Features**:

- Authentication and secure connection
- Buy/Sell trading operations
- Balance retrieval
- Server time synchronization
- Symbol subscriptions with different types (real-time, time-aligned, chunked)
- Trade result checking
- Opened deals management
- Asset information and validation
- Automatic reconnection handling
- Historical candle data (`get_candles`, `get_candles_advanced`)
- Advanced validators

**Temporarily Unavailable Features** (returning "work in progress" errors):

- Trade history (`history`)
- Closed deals management
- Payout information retrieval
- Raw message sending
- Deal end time queries

We're actively working to restore all functionality with improved stability and performance.

## How to install

Install it with PyPi using the following command:

```bash
pip install binaryoptionstoolsv2
```

## Supported OS

Currently, only support for Windows is available.

## Supported Python versions

Currently, only Python 3.9 to 3.12 is supported.

## Compile from source (Not recommended)

- Make sure you have `rust` and `cargo` installed (Check here)

- Install [`maturin`](https://www.maturin.rs/installation) in order to compile the library

- Once the source is downloaded (using `git clone https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git`) execute the following commands:
To create the `.whl` file

```bash
// Inside the root folder
cd BinaryOptionsToolsV2
maturin build -r 

// Once the command is executed it should print a path to a .whl file, copy it and then run
pip install path/to/file.whl
```

To install the library in a local virtual environment

```bash
// Inside the root folder
cd BinaryOptionsToolsV2

// Activate the virtual environment if not done already 

// Execute the following command and it should automatically install the library in the VM
maturin develop
```

## Docs

Comprehensive Documentation for BinaryOptionsToolsV2

1. `__init__.py`

This file initializes the Python module and organizes the imports for both synchronous and asynchronous functionality.

Key Details

- **Imports `BinaryOptionsToolsV2`**: Imports all elements and documentation from the Rust module.
- **Includes Submodules**: Imports and exposes `pocketoption` and `tracing` modules for user convenience.

Purpose

Serves as the entry point for the package, exposing all essential components of the library.

### Inside the `pocketoption` folder there are 2 main files

1. `asynchronous.py`

This file implements the `PocketOptionAsync` class, which provides an asynchronous interface to interact with Pocket Option.

Key Features of PocketOptionAsync

- **Trade Operations**:
  - `buy()`: Places a buy trade asynchronously.
  - `sell()`: Places a sell trade asynchronously.
  - `check_win()`: Checks the outcome of a trade ('win', 'draw', or 'loss').
- **Market Data**:
  - `get_candles()`: Fetches historical candle data.
  - ~~`history()`: Retrieves recent data for a specific asset.~~ (Work in Progress)
- **Account Management**:
  - `balance()`: Returns the current account balance.
  - `opened_deals()`: Lists all open trades.
  - ~~`closed_deals()`: Lists all closed trades.~~ (Work in Progress)
  - ~~`payout()`: Returns payout percentages.~~ (Work in Progress)
- **Real-Time Data**:
  - `subscribe_symbol()`: Provides an asynchronous iterator for real-time candle updates.
  - `subscribe_symbol_timed()`: Provides an asynchronous iterator for timed real-time candle updates.
  - `subscribe_symbol_chunked()`: Provides an asynchronous iterator for chunked real-time candle updates.
- **Server Information**:
  - `server_time()`: Gets the current server time.
- **Connection Management**:
  - `reconnect()`: Manually reconnect to the server.
  - `shutdown()`: Properly close the connection.

Helper Class - `AsyncSubscription`

Facilitates asynchronous iteration over live data streams, enabling non-blocking operations.

Example Usage

```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync 
import asyncio 
 
async def main(): 
    # Initialize the client
    client = PocketOptionAsync(ssid="your-session-id")
    
    # IMPORTANT: Wait for connection to establish
    await asyncio.sleep(5)
    
    # Get account balance
    balance = await client.balance() 
    print(f"Account Balance: ${balance}")
    
    # Place a buy trade
    trade_id, deal = await client.buy("EURUSD_otc", 60, 1.0)
    print(f"Trade placed: {deal}")
    
    # Check result
    result = await client.check_win(trade_id)
    print(f"Trade result: {result}")
    
    # Subscribe to real-time data
    async for candle in client.subscribe_symbol("EURUSD_otc"):
        print(f"New candle: {candle}")
        break  # Just print one candle for demo
 
asyncio.run(main()) 
```

1. `synchronous.py`

This file implements the `PocketOption` class, a synchronous wrapper around the asynchronous interface provided by `PocketOptionAsync`.

Key Features of PocketOption

- **Trade Operations**:
  - `buy()`: Places a buy trade using synchronous execution.
  - `sell()`: Places a sell trade.
  - `check_win()`: Checks the trade outcome synchronously.
- **Market Data**:
  - `get_candles()`: Fetches historical candle data.
  - ~~`history()`: Retrieves recent data for a specific asset.~~ (Work in Progress)
- **Account Management**:
  - `balance()`: Retrieves account balance.
  - `opened_deals()`: Lists all open trades.
  - ~~`closed_deals()`: Lists all closed trades.~~ (Work in Progress)
  - ~~`payout()`: Returns payout percentages.~~ (Work in Progress)
- **Real-Time Data**:
  - `subscribe_symbol()`: Provides a synchronous iterator for live data updates.
  - `subscribe_symbol_timed()`: Provides a synchronous iterator for timed real-time candle updates.
  - `subscribe_symbol_chunked()`: Provides a synchronous iterator for chunked real-time candle updates.
- **Server Information**:
  - `server_time()`: Gets the current server time.
- **Connection Management**:
  - `reconnect()`: Manually reconnect to the server.
  - `shutdown()`: Properly close the connection.

Helper Class - `SyncSubscription`

Allows synchronous iteration over real-time data streams for compatibility with simpler scripts.

Example Usage

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption 
import time

# Initialize the client
client = PocketOption(ssid="your-session-id")

# IMPORTANT: Wait for connection to establish
time.sleep(5)

# Get account balance
balance = client.balance() 
print(f"Account Balance: ${balance}")

# Place a buy trade
trade_id, deal = client.buy("EURUSD_otc", 60, 1.0)
print(f"Trade placed: {deal}")

# Check result
result = client.check_win(trade_id)
print(f"Trade result: {result}")

# Subscribe to real-time data
stream = client.subscribe_symbol("EURUSD_otc")
for candle in stream:
    print(f"New candle: {candle}")
    break  # Just print one candle for demo
```

1. Differences Between PocketOption and PocketOptionAsync

| Feature                | PocketOption (Synchronous) | PocketOptionAsync (Asynchronous) |
|------------------------|----------------------------|----------------------------------|
| **Execution Type**     | Blocking                  | Non-blocking                    |
| **Use Case**           | Simpler scripts           | High-frequency or real-time tasks |
| **Performance**        | Slower for concurrent tasks | Scales well with concurrent operations |

### Tracing

The `tracing` module provides functionality to initialize and manage logging for the application.

Key Functions of Tracing

- **start_logs()**:
  - Initializes the logging system for the application.
  - **Arguments**:
    - `path` (str): Path where log files will be stored.
    - `level` (str): Logging level (default is "DEBUG").
    - `terminal` (bool): Whether to display logs in the terminal (default is True).
  - **Returns**: None
  - **Raises**: Exception if there's an error starting the logging system.

Example Usage

```python
from BinaryOptionsToolsV2.tracing import start_logs

# Initialize logging
start_logs(path="logs/", level="INFO", terminal=True)
```

## 📖 Detailed Examples

### Basic Trading Example (Synchronous)

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption
import time

def main():
    # Initialize client
    client = PocketOption(ssid="your-session-id")
    
    # IMPORTANT: Wait for connection
    time.sleep(5)
    
    # Get balance
    balance = client.balance()
    print(f"Current Balance: ${balance}")
    
    # Place a buy trade on EURUSD for 60 seconds with $1
    trade_id, deal = client.buy(asset="EURUSD_otc", time=60, amount=1.0)
    print(f"Trade ID: {trade_id}")
    print(f"Deal Data: {deal}")
    
    # Wait for trade to complete (60 seconds)
    time.sleep(65)
    
    # Check the result
    result = client.check_win(trade_id)
    print(f"Trade Result: {result['result']}")  # 'win', 'loss', or 'draw'
    print(f"Profit: ${result.get('profit', 0)}")

if __name__ == "__main__":
    main()
```

### Basic Trading Example (Asynchronous)

```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import asyncio

async def main():
    # Initialize client
    client = PocketOptionAsync(ssid="your-session-id")
    
    # IMPORTANT: Wait for connection
    await asyncio.sleep(5)
    
    # Get balance
    balance = await client.balance()
    print(f"Current Balance: ${balance}")
    
    # Place a buy trade on EURUSD for 60 seconds with $1
    trade_id, deal = await client.buy(asset="EURUSD_otc", time=60, amount=1.0)
    print(f"Trade ID: {trade_id}")
    print(f"Deal Data: {deal}")
    
    # Wait for trade to complete (60 seconds)
    await asyncio.sleep(65)
    
    # Check the result
    result = await client.check_win(trade_id)
    print(f"Trade Result: {result['result']}")  # 'win', 'loss', or 'draw'
    print(f"Profit: ${result.get('profit', 0)}")

if __name__ == "__main__":
    asyncio.run(main())
```

### Real-Time Data Subscription (Synchronous)

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption
import time

def main():
    client = PocketOption(ssid="your-session-id")
    time.sleep(5)  # Wait for connection
    
    # Subscribe to real-time candle data
    stream = client.subscribe_symbol("EURUSD_otc")
    
    print("Listening for real-time candles...")
    for candle in stream:
        print(f"Time: {candle.get('time')}")
        print(f"Open: {candle.get('open')}")
        print(f"High: {candle.get('high')}")
        print(f"Low: {candle.get('low')}")
        print(f"Close: {candle.get('close')}")
        print("---")

if __name__ == "__main__":
    main()
```

### Real-Time Data Subscription (Asynchronous)

```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import asyncio

async def main():
    client = PocketOptionAsync(ssid="your-session-id")
    await asyncio.sleep(5)  # Wait for connection
    
    # Subscribe to real-time candle data
    async for candle in client.subscribe_symbol("EURUSD_otc"):
        print(f"Time: {candle.get('time')}")
        print(f"Open: {candle.get('open')}")
        print(f"High: {candle.get('high')}")
        print(f"Low: {candle.get('low')}")
        print(f"Close: {candle.get('close')}")
        print("---")

if __name__ == "__main__":
    asyncio.run(main())
```

### Checking Opened Deals

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption
import time

def main():
    client = PocketOption(ssid="your-session-id")
    time.sleep(5)  # Wait for connection
    
    # Get all opened deals
    opened_deals = client.opened_deals()
    
    if opened_deals:
        print(f"You have {len(opened_deals)} opened deals:")
        for deal in opened_deals:
            print(f"  - Trade ID: {deal.get('id')}")
            print(f"    Asset: {deal.get('asset')}")
            print(f"    Amount: ${deal.get('amount')}")
            print(f"    Direction: {deal.get('action')}")
    else:
        print("No opened deals")

if __name__ == "__main__":
    main()
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the connection to establish properly. The library connects to the WebSocket in a separate thread/task, so the code continues immediately. Without the wait, API calls will fail.

```python
# Synchronous
client = PocketOption(ssid="your-session-id")
time.sleep(5)  # Critical!

# Asynchronous
client = PocketOptionAsync(ssid="your-session-id")
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
- And many more...

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## 📚 Additional Resources

- **Full Examples**: [examples/python](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/tree/master/examples/python)
- **API Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/python.html](https://chipadevteam.github.io/BinaryOptionsTools-v2/python.html)
- **Discord Community**: [Join us](https://discord.gg/T3FGXcmd)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
