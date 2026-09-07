import asyncio

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    async with PocketOptionAsync(ssid) as api:
        # Get historical tick data for an asset (e.g., USDCHF_otc) for the last 300 seconds (5 minutes)
        asset = "USDCHF_otc"
        lookback_seconds = 300

        print(f"Fetching tick history for {asset} (last {lookback_seconds} seconds)...")
        ticks = await api.get_ticks(asset, lookback_seconds)

        if ticks:
            print(f"Retrieved {len(ticks)} ticks.")
            # Show first 5 ticks
            print("First 5 ticks:")
            for ts, price in ticks[:5]:
                print(f"  {ts}: {price}")
            # Show last 5 ticks
            print("Last 5 ticks:")
            for ts, price in ticks[-5:]:
                print(f"  {ts}: {price}")
        else:
            print("No ticks retrieved.")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
