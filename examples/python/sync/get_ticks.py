import time

import pandas as pd

from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)

    # Get historical tick data for an asset (e.g., USDCHF_otc) for the last 300 seconds (5 minutes)
    asset = "USDCHF_otc"
    lookback_seconds = 300

    print(f"Fetching tick history for {asset} (last {lookback_seconds} seconds)...")
    ticks = api.get_ticks(asset, lookback_seconds)

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

        # Convert to pandas DataFrame for easier viewing
        df = pd.DataFrame(ticks, columns=["timestamp", "price"])
        print(f"\nDataFrame:\n{df}")
    else:
        print("No ticks retrieved.")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
