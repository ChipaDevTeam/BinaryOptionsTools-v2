import asyncio

import pandas as pd

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)
    await asyncio.sleep(5)

    # Candles are returned in the format of a list of dictionaries
    candles = await api.history("EURUSD_otc", 3600)
    print(f"Raw Candles: {candles}")
    candles_pd = pd.DataFrame.from_dict(candles)
    print(f"Candles: {candles_pd}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
