from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    with PocketOption(ssid) as api:
        stream = api.subscribe_symbol_chunked(
            "EURUSD_otc", 15
        )  # Returns a candle obtained from combining 15 (chunk_size) candles

        # This should run forever so you will need to force close the program
        for candle in stream:
            print(f"Candle: {candle}")  # Each candle is in format of a dictionary


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
