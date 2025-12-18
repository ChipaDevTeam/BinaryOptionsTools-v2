import asyncio

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)
    await asyncio.sleep(5)

    # Candñes are returned in the format of a list of dictionaries
    full_payout = await api.payout()  # Returns a dictionary asset: payout
    print(f"Full Payout: {full_payout}")
    partial_payout = await api.payout(
        ["EURUSD_otc", "EURUSD", "AEX25"]
    )  # Returns a list of the payout for each of the passed assets in order
    print(f"Partial Payout: {partial_payout}")
    single_payout = await api.payout(
        "EURUSD_otc"
    )  # Returns the payout for the specified asset
    print(f"Single Payout: {single_payout}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
