import time

from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)

    # payouts() returns a dict of asset: payout for every asset.
    all_payouts = api.payouts()
    print(f"All Payouts: {all_payouts}")

    # payout(asset) returns the payout for a single asset.
    single_payout = api.payout("EURUSD_otc")
    print(f"Single Payout: {single_payout}")

    # Pick specific assets out of the full mapping when you need several.
    selected = {
        asset: all_payouts.get(asset) for asset in ["EURUSD_otc", "EURUSD", "AEX25"]
    }
    print(f"Selected Payouts: {selected}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
