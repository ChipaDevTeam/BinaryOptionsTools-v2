from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import asyncio


async def main(ssid: str):
    # Initialize the API client
    api = PocketOptionAsync(ssid)
    await asyncio.sleep(5)  # Wait for connection to establish

    # Example of sending a raw message
    try:
        # Subscribe to signals
        await api.send_raw_message('42["signals/subscribe"]')
        print("Sent signals subscription message")

        # Subscribe to price updates
        await api.send_raw_message('42["price/subscribe"]')
        print("Sent price subscription message")

        # Custom message example
        custom_message = '42["custom/event",{"param":"value"}]'
        await api.send_raw_message(custom_message)
        print(f"Sent custom message: {custom_message}")

        # Multiple messages in sequence
        messages = [
            '42["chart/subscribe",{"asset":"EURUSD"}]',
            '42["trades/subscribe"]',
            '42["notifications/subscribe"]',
        ]

        for msg in messages:
            await api.send_raw_message(msg)
            print(f"Sent message: {msg}")
            await asyncio.sleep(1)  # Small delay between messages

    except Exception as e:
        print(f"Error sending message: {e}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
