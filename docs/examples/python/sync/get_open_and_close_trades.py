import time

from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)  # Wait for connection to establish
    _ = api.buy(asset="EURUSD_otc", amount=1.0, time=60, check_win=False)
    _ = api.sell(asset="EURUSD_otc", amount=1.0, time=60, check_win=False)
    # This is the same as setting checkw_win to true on the api.buy and api.sell functions
    opened_deals = api.opened_deals()
    time.sleep(62)  # Wait for the trades to complete
    closed_deals = api.closed_deals()
    print(
        f"Opened deals: {opened_deals}\nNumber of opened deals: {len(opened_deals)} (should be at least 2)"
    )
    print(
        f"Closed deals: {closed_deals}\nNumber of closed deals: {len(closed_deals)} (should be at least 2)"
    )


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
