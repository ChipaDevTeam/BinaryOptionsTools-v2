import asyncio
import os
import sys

# Ensure we use the local version of the library
sys.path.insert(
    0,
    os.path.abspath(
        os.path.join(os.path.dirname(__file__), "../../BinaryOptionsToolsV2")
    ),
)

from BinaryOptionsToolsV2 import PocketOptionAsync

# Get SSID from environment variable
SSID = os.getenv("POCKET_OPTION_SSID")


async def trade_task(api, asset, amount, time, task_id):
    print(f"Task {task_id}: Starting trade...")
    try:
        # buying usually returns a tuple (uuid, deal)
        result = await api.buy(asset, amount, time)
        print(f"Task {task_id}: Trade completed: {result}")
    except Exception as e:
        print(f"Task {task_id}: Trade failed: {e}")


async def main():
    if not SSID:
        print("POCKET_OPTION_SSID not set. Skipping test.")
        return

    try:
        print("Connecting...")
        async with PocketOptionAsync(SSID) as api:
            print("Connected and assets loaded.")

            # Verify connection
            await api.balance()

            # Simulate two concurrent trades
            print("Starting concurrent trades...")
            task1 = asyncio.create_task(trade_task(api, "EURUSD_otc", 1.0, 60, 1))
            task2 = asyncio.create_task(trade_task(api, "EURUSD_otc", 1.0, 60, 2))

            await asyncio.gather(task1, task2)

    except Exception as e:
        print(f"Failed to init api or run test: {e}")


if __name__ == "__main__":
    asyncio.run(main())
