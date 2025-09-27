from BinaryOptionsToolsV2.BinaryOptionsToolsV2.pocketoption.syncronous import (
    PocketOption,
)
import time


def main(ssid):
    api = PocketOption(ssid)
    time.sleep(5)
    iterator = api.subscribe_symbol("EURUSD_otc")
    for item in iterator:
        print(item)


if __name__ == "__main__":
    ssid = input("Write your ssid: ")
    main(ssid)
