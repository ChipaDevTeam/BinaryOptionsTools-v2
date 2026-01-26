import asyncio
import os
import sys
from BinaryOptionsToolsV2 import PocketOption

# Mock SSID (won't connect effectively but allows object creation)
SSID = r'42["auth",{"session":"mock_session","isDemo":1,"uid":12345,"platform":1}]'

async def trade_task(api, asset, amount, time, task_id):
    print(f"Task {task_id}: Starting trade...")
    try:
        # buying usually returns a tuple (uuid, deal)
        result = await api.buy(asset, amount, time)
        print(f"Task {task_id}: Trade completed: {result}")
    except Exception as e:
        print(f"Task {task_id}: Trade failed: {e}")

async def main():
    # This test assumes we can mock the connection or at least instantiate the client
    # Without a live server or extensive mocking, this script is illustrative.
    # However, if we could run it, it would hang.

    try:
        api = await PocketOption(SSID)
    except Exception as e:
        print(f"Failed to init api (expected if no connection): {e}")
        return

    # Simulate two concurrent trades
    task1 = asyncio.create_task(trade_task(api, "EURUSD_otc", 1.0, 60, 1))
    task2 = asyncio.create_task(trade_task(api, "EURUSD_otc", 1.0, 60, 2))

    await asyncio.gather(task1, task2)

    await api.disconnect()

if __name__ == "__main__":
    asyncio.run(main())
