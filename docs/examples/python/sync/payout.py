import time

from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)

    # Candñes are returned in the format of a list of dictionaries
    full_payout = api.payout()  # Returns a dictionary asset: payout
    print(f"Full Payout: {full_payout}")
    partial_payout = api.payout(
        ["EURUSD_otc", "EURUSD", "AEX25"]
    )  # Returns a list of the payout for each of the passed assets in order
    print(f"Partial Payout: {partial_payout}")
    single_payout = api.payout(
        "EURUSD_otc"
    )  # Returns the payout for the specified asset
    print(f"Single Payout: {single_payout}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
