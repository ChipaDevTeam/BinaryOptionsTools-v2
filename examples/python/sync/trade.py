from BinaryOptionsToolsV2.pocketoption import PocketOption
import time


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)  # Wait for connection to establish

    (buy_id, buy) = api.buy(asset="EURUSD_otc", amount=1.0, time=60, check_win=False)
    print(f"Buy trade id: {buy_id}\nBuy trade data: {buy}")
    (sell_id, sell) = api.sell(asset="EURUSD_otc", amount=1.0, time=60, check_win=False)
    print(f"Sell trade id: {sell_id}\nSell trade data: {sell}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
