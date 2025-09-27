from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
from BinaryOptionsToolsV2.validator import Validator
from datetime import timedelta

import asyncio


async def main(ssid: str):
    # Initialize the API client
    api = PocketOptionAsync(ssid)
    await asyncio.sleep(5)  # Wait for connection to establish

    # Basic raw order example
    try:
        validator = Validator.contains('"status":"success"')
        response = await api.create_raw_order('42["signals/subscribe"]', validator)
        print(f"Basic raw order response: {response}")
    except Exception as e:
        print(f"Basic raw order failed: {e}")

    # Raw order with timeout example
    try:
        validator = Validator.regex(r'{"type":"signal","data":.*}')
        response = await api.create_raw_order_with_timout(
            '42["signals/load"]', validator, timeout=timedelta(seconds=5)
        )
        print(f"Raw order with timeout response: {response}")
    except TimeoutError:
        print("Order timed out after 5 seconds")
    except Exception as e:
        print(f"Order with timeout failed: {e}")

    # Raw order with timeout and retry example
    try:
        # Create a validator that checks for both conditions
        validator = Validator.all(
            [
                Validator.contains('"type":"trade"'),
                Validator.contains('"status":"completed"'),
            ]
        )

        response = await api.create_raw_order_with_timeout_and_retry(
            '42["trade/subscribe"]', validator, timeout=timedelta(seconds=10)
        )
        print(f"Raw order with retry response: {response}")
    except Exception as e:
        print(f"Order with retry failed: {e}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
