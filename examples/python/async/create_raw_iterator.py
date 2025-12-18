import asyncio
from datetime import timedelta

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
from BinaryOptionsToolsV2.validator import Validator


async def main(ssid: str):
    # Initialize the API client
    api = PocketOptionAsync(ssid)
    await asyncio.sleep(5)  # Wait for connection to establish

    # Create a validator for price updates
    validator = Validator.regex(r'{"price":\d+\.\d+}')

    # Create an iterator with 5 minute timeout
    stream = await api.create_raw_iterator(
        '42["price/subscribe"]',  # WebSocket subscription message
        validator,
        timeout=timedelta(minutes=5),
    )

    try:
        # Process messages as they arrive
        async for message in stream:
            print(f"Received message: {message}")
    except TimeoutError:
        print("Stream timed out after 5 minutes")
    except Exception as e:
        print(f"Error processing stream: {e}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
