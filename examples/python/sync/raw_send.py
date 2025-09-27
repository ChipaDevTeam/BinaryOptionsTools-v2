from BinaryOptionsToolsV2.pocketoption import PocketOption
import time


def main(ssid: str):
    # Initialize the API client
    api = PocketOption(ssid)
    time.sleep(5)  # Wait for connection to establish

    # Example of sending a raw message
    try:
        # Subscribe to signals
        api.raw_send('42["signals/subscribe"]')
        print("Sent signals subscription message")

        # Subscribe to price updates
        api.raw_send('42["price/subscribe"]')
        print("Sent price subscription message")

        # Custom message example
        custom_message = '42["custom/event",{"param":"value"}]'
        api.raw_send(custom_message)
        print(f"Sent custom message: {custom_message}")

    except Exception as e:
        print(f"Error sending message: {e}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
