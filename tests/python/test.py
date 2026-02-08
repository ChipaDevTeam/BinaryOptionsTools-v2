import asyncio
import sys
import os

# Ensure we use the local version of the library
sys.path.insert(
    0,
    os.path.abspath(
        os.path.join(os.path.dirname(__file__), "../../BinaryOptionsToolsV2")
    ),
)

# print(BinaryOptionsToolsV2)
from BinaryOptionsToolsV2 import PocketOptionAsync

# import pandas as pd # type: ignore
# import json

# import BinaryOptionsToolsV2
# from BinaryOptionsToolsV2 import connect


# async def main(ssid):
#     api = await async_connect(ssid)
#     await asyncio.sleep(10)
#     payout = await api.payout()
#     candles = await api.history("EURUSD_otc", 7200)
#     trade = await api.buy("EURUSD_otc", 1, 5)
#     print(f"Payout: {payout}")
#     print(f"Candles: {candles}")
#     print(f"Trade: {trade}")
#     df = pd.DataFrame.from_dict(candles)
#     df.to_csv("candles_eurusd_otc.csv")
async def main(ssid):
    # Testing the new iterator
    async with PocketOptionAsync(ssid) as api:
        print("Connected and assets loaded.")
        stream = await api.subscribe_symbol("EURUSD_otc")
        async for item in stream:
            # Check if 'time' key exists (it might be 'timestamp' or different structure)
            if "time" in item:
                print(item["time"], item.get("open"))
            elif "timestamp" in item:
                print(item["timestamp"], item.get("open"))
            else:
                print("Received item:", item)


if __name__ == "__main__":
    import os

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        ssid = input("Write your ssid: ")
    asyncio.run(main(ssid))
