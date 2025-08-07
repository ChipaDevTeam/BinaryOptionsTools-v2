+++
title = "Python Example"
description = "Example of how to use the PocketOption client in Python."
weight = 2
+++

# Python Example

Here is a basic example of how to use the `PocketOption` client in a Python application. This example uses the `asyncronous` client from the `BinaryOptionsToolsV2` library.

For more detailed examples, please refer to the `examples/python/async` directory in the `BinaryOptionsToolsV2` folder.

```python
import asyncio
from BinaryOptionsToolsV2.pocketoption.asyncronous import PocketOption

async def main():
    # Replace with your actual session ID
    ssid = "YOUR_SSID_HERE"

    # Create a new PocketOption client
    client = await PocketOption(ssid)

    # Get the current balance
    balance = await client.get_balance()
    print(f"Current balance: {balance}")

    # Get the list of available assets
    assets = await client.get_assets()
    print(f"Available assets: {assets}")

    # Subscribe to price updates for an asset
    # (Subscription API is not yet fully implemented in the Python wrapper)

    # Close the client connection
    await client.close()

if __name__ == "__main__":
    asyncio.run(main())
```
