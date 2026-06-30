import time

import pandas as pd

from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)

    # Candñes are returned in the format of a list of dictionaries
    candles = api.get_candles("EURUSD_otc", 60, 3600)
    print(f"Raw Candles: {candles}")
    candles_pd = pd.DataFrame.from_dict(candles)
    print(f"Candles: {candles_pd}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
