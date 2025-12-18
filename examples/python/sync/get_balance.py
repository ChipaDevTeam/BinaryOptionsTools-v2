import time

from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOption(ssid)
    time.sleep(5)

    balance = api.balance()
    print(f"Balance: {balance}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
