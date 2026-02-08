from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    with PocketOption(ssid) as api:
        balance = api.balance()
        print(f"Balance: {balance}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
