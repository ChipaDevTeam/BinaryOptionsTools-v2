import os
import time

from BinaryOptionsToolsV2 import PocketOption


def main(ssid):
    api = PocketOption(ssid)
    time.sleep(5)
    iterator = api.subscribe_symbol("EURUSD_otc")
    for item in iterator:
        print(item)


if __name__ == "__main__":
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        ssid = input("Write your ssid: ")
    main(ssid)
