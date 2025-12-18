from BinaryOptionsToolsV2.pocketoption import PocketOption
from BinaryOptionsToolsV2.tracing import start_logs


# Main part of the code
def main(ssid: str):
    # Start logs, it works perfectly on sync code
    start_logs(
        path=".", level="DEBUG", terminal=True
    )  # If false then the logs will only be written to the log files
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    (buy_id, _) = api.buy(asset="EURUSD_otc", amount=1.0, time=300, check_win=False)
    (sell_id, _) = api.sell(asset="EURUSD_otc", amount=1.0, time=300, check_win=False)
    print(buy_id, sell_id)
    # This is the same as setting checkw_win to true on the api.buy and api.sell functions
    buy_data = api.check_win(buy_id)
    sell_data = api.check_win(sell_id)
    print(f"Buy trade result: {buy_data['result']}\nBuy trade data: {buy_data}")
    print(f"Sell trade result: {sell_data['result']}\nSell trade data: {sell_data}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
