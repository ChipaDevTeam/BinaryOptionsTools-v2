👉 [Join us on Discord](https://discord.gg/T3FGXcmd)
# [BinaryOptionsToolsV2](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/tree/0.1.6a4)

## How to install
Install it with PyPi using the following command:
```bash
pip install binaryoptionstoolsv2==0.1.6a3
```

## Supported OS
Currently, only support for Windows is available.

## Supported Python versions
Currently, only Python 3.9 to 3.12 is supported.

## Compile from source (Not recommended)
* Make sure you have `rust` and `cargo` installed (Check here)

* Install [`maturin`](https://www.maturin.rs/installation) in order to compile the library

* Once the source is downloaded (using `git clone https://github.com/ChipaDevTeam/BinaryOptionsTools-v2.git`) execute the following commands:
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
2. `asynchronous.py` 

This file implements the `PocketOptionAsync` class, which provides an asynchronous interface to interact with Pocket Option. 

Key Features of PocketOptionAsync 

- **Trade Operations**: 
  - `buy()`: Places a buy trade asynchronously. 
  - `sell()`: Places a sell trade asynchronously. 
  - `check_win()`: Checks the outcome of a trade ('win', 'draw', or 'loss'). 
- **Market Data**: 
  - `get_candles()`: Fetches historical candle data. 
  - `history()`: Retrieves recent data for a specific asset. 
- **Account Management**: 
  - `balance()`: Returns the current account balance. 
  - `opened_deals()`: Lists all open trades. 
  - `closed_deals()`: Lists all closed trades. 
  - `payout()`: Returns payout percentages. 
- **Real-Time Data**: 
  - `subscribe_symbol()`: Provides an asynchronous iterator for real-time candle updates. 
  - `subscribe_symbol_timed()`: Provides an asynchronous iterator for timed real-time candle updates.
  - `subscribe_symbol_chunked()`: Provides an asynchronous iterator for chunked real-time candle updates.

Helper Class - `AsyncSubscription` 

Facilitates asynchronous iteration over live data streams, enabling non-blocking operations. 

Example Usage 

```python
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync 
import asyncio 
 
async def main(): 
    client = PocketOptionAsync(ssid="your-session-id") 
    await asyncio.sleep(5)
    balance = await client.balance() 
    print("Account Balance:", balance) 
 
asyncio.run(main()) 
``` 

3. `synchronous.py` 

This file implements the `PocketOption` class, a synchronous wrapper around the asynchronous interface provided by `PocketOptionAsync`. 

Key Features of PocketOption 

- **Trade Operations**: 
  - `buy()`: Places a buy trade using synchronous execution. 
  - `sell()`: Places a sell trade. 
  - `check_win()`: Checks the trade outcome synchronously. 
- **Market Data**: 
  - `get_candles()`: Fetches historical candle data. 
  - `history()`: Retrieves recent data for a specific asset. 
- **Account Management**: 
  - `balance()`: Retrieves account balance. 
  - `opened_deals()`: Lists all open trades. 
  - `closed_deals()`: Lists all closed trades. 
  - `payout()`: Returns payout percentages. 
- **Real-Time Data**: 
  - `subscribe_symbol()`: Provides a synchronous iterator for live data updates. 
  - `subscribe_symbol_timed()`: Provides a synchronous iterator for timed real-time candle updates.
  - `subscribe_symbol_chunked()`: Provides a synchronous iterator for chunked real-time candle updates.

Helper Class - `SyncSubscription` 

Allows synchronous iteration over real-time data streams for compatibility with simpler scripts. 

Example Usage 

```python
from BinaryOptionsToolsV2.pocketoption import PocketOption 
import time

client = PocketOption(ssid="your-session-id") 
time.sleep(5)
balance = client.balance() 
print("Account Balance:", balance) 
```

4. Differences Between PocketOption and PocketOptionAsync 

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
