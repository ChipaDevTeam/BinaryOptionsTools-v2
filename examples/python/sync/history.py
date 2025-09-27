from BinaryOptionsToolsV2.pocketoption import PocketOption

import pandas as pd
import time


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)

    # Candles are returned in the format of a list of dictionaries
    candles = api.history("EURUSD_otc", 3600)
    print(f"Raw Candles: {candles}")
    candles_pd = pd.DataFrame.from_dict(candles)
    print(f"Candles: {candles_pd}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
