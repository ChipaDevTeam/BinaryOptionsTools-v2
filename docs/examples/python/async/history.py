import asyncio
import pandas as pd
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    async with PocketOptionAsync(ssid) as api:
        # Get history for an asset (e.g., EURUSD_otc) with a specific period (e.g., 60 seconds)
        asset = "EURUSD_otc"
        period = 60

        print(f"Fetching history for {asset}...")
        candles = await api.history(asset, period)

        if candles:
            print(f"Retrieved {len(candles)} candles.")
            # Convert to pandas DataFrame for easier viewing
            df = pd.DataFrame(candles)
            print(df.tail(10))
        else:
            print("No candles retrieved.")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
