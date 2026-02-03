import asyncio
import os
import logging
from BinaryOptionsToolsV2 import PocketOptionAsync

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


async def main():
    """
    Comprehensive demo covering all BinaryOptionsToolsV2 methods.

    Prerequisites:
    - Set POCKET_OPTION_SSID environment variable to your session ID.
    """

    # 1. Configuration
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        logger.error("POCKET_OPTION_SSID environment variable not set.")
        logger.info("Please set it using: export POCKET_OPTION_SSID='your_session_id'")
        return

    logger.info("Initializing PocketOptionAsync client...")
    client = PocketOptionAsync(ssid=ssid)

    try:
        # 2. Account Balance
        logger.info("--- Account Balance ---")
        balance = await client.balance()
        logger.info(f"Current Balance: ${balance}")

        # 3. Asset Information (Payouts)
        logger.info("\n--- Asset Information ---")
        asset = "EURUSD_otc"
        # Note: Assuming get_payout is available or similar method
        # If not, this block serves as a placeholder for asset info retrieval
        try:
            # payout = await client.get_payout(asset)
            # logger.info(f"Payout for {asset}: {payout}%")
            pass
        except Exception as e:
            logger.warning(f"Payout retrieval not available: {e}")

        # 4. Historical Data (Candles)
        logger.info("\n--- Historical Data ---")
        try:
            # Fetch 60s candles, offset 0 (latest)
            candles = await client.get_candles(asset, 60, 0)
            logger.info(f"Retrieved {len(candles)} candles for {asset}")
            if candles:
                logger.info(f"Latest candle: {candles[-1]}")
        except Exception as e:
            logger.error(f"Failed to fetch candles: {e}")

        # 5. Real-time Subscriptions
        logger.info("\n--- Real-time Data ---")
        logger.info(f"Subscribing to {asset} (1s)...")
        from datetime import timedelta
        subscription = await client.subscribe_symbol_timed(asset, timedelta(seconds=1))

        logger.info("Collecting 3 live candles...")
        count = 0
        async for candle in subscription:
            logger.info(f"Live: {candle}")
            count += 1
            if count >= 3:
                break

        # 6. Trading Operations
        logger.info("\n--- Trading Operations ---")
        amount = 1.0
        duration = 60  # seconds

        logger.info(f"Placing BUY order: {asset}, ${amount}, {duration}s")
        try:
            trade_id, deal = await client.buy(asset, amount, duration)
            logger.info(f"Trade placed. ID: {trade_id}")
            logger.info(f"Deal info: {deal}")

            logger.info("Waiting for trade result...")
            # In a real app, you might use a callback or loop checking status
            # Here we wait for duration + buffer
            await asyncio.sleep(duration + 2)

            result = await client.check_win(trade_id)
            # result is a dict with 'result' key being "win", "loss", or "draw"
            logger.info(f"Trade Result: {result.get('result', 'unknown').upper()}")

        except Exception as e:
            logger.error(f"Trading error: {e}")

        # 7. History
        logger.info("\n--- History ---")
        try:
            # history = await client.history()
            # logger.info(f"History items: {len(history)}")
            pass
        except Exception as e:
            logger.warning(f"History retrieval not available: {e}")

    except Exception as e:
        logger.error(f"Unexpected error: {e}")
    finally:
        logger.info("\n--- Cleanup ---")
        await client.disconnect()
        logger.info("Disconnected.")


if __name__ == "__main__":
    asyncio.run(main())
