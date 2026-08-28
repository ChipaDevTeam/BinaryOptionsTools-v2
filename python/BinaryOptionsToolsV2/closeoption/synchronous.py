import asyncio
import json
import threading
import sys
import warnings
from datetime import timedelta
from typing import Dict, List, Optional, Tuple, Union
from ..config import Config
from ..validator import Validator as Validator
from .asynchronous import CloseOptionAsync as CloseOptionAsync


class SyncSubscription:
    def __init__(self, subscription):
        self.subscription = subscription

    def __iter__(self):
        return self

    def __aiter__(self):
        """Return the async iterator for the subscription."""
        return self.subscription

    def __next__(self):
        return json.loads(next(self.subscription))


class SyncCandleLiveIterator:
    """Synchronous iterator for live candle updates."""

    def __init__(self, async_gen, loop):
        self.async_gen = async_gen
        self.loop = loop

    def __iter__(self):
        return self

    def __next__(self):
        async def get_next():
            try:
                return await anext(self.async_gen)
            except StopAsyncIteration:
                raise StopIteration

        return self.loop.run_until_complete(get_next())


class RawHandlerSync:
    """Synchronous handler for advanced raw WebSocket message operations."""

    def __init__(self, handler):
        self._handler = handler
        self._loop = asyncio.new_event_loop()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

    def _run_loop(self):
        asyncio.set_event_loop(self._loop)
        self._loop.run_forever()

    def _run(self, coro):
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result()

    def send(self, message: str) -> None:
        """Send a raw message through the WebSocket connection."""
        self._run(self._handler.send(message))

    def wait_for(self, event: str, timeout: float = 30.0) -> dict:
        """Wait for a specific event from the server."""
        return self._run(self._handler.wait_for(event, timeout))

    def subscribe(self, event: str) -> 'SyncRawSubscription':
        """Subscribe to a specific event type."""
        sub = self._run(self._handler.subscribe(event))
        return SyncRawSubscription(sub)

    def close(self) -> None:
        """Close the raw handler and release resources."""
        if self._handler:
            self._run(self._handler.close())
            self._handler = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()


class SyncRawSubscription:
    """Synchronous iterator for raw subscriptions."""

    def __init__(self, subscription):
        self.subscription = subscription

    def __iter__(self):
        return self

    def __next__(self):
        return next(self.subscription)


class CloseOption:
    def __init__(self, ssid: str, url: Optional[str] = None, config: Union[Config, dict, str] = None, **_):
        """
        Initialize CloseOption synchronous client.

        Args:
            ssid: Session ID in format "token|sid|demo|public_code|hidden_code" or JSON
            url: WebSocket URL (optional, defaults to CloseOption)
            config: Configuration object (optional)
        """
        self._ssid = ssid
        self._url = url
        self._config = config if isinstance(config, Config) else Config(config) if config else Config()
        self._async_client = CloseOptionAsync(ssid, url, self._config)
        self._loop = asyncio.new_event_loop()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

    def _run_loop(self):
        asyncio.set_event_loop(self._loop)
        self._loop.run_forever()

    def _run(self, coro):
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result()

    def __enter__(self):
        self._run(self._async_client.connect())
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.shutdown()

    def buy(self, asset: str, amount: float, time: int) -> dict:
        """Place a BUY (CALL) order."""
        return self._run(self._async_client.buy(asset, amount, time))

    def sell(self, asset: str, amount: float, time: int) -> dict:
        """Place a SELL (PUT) order."""
        return self._run(self._async_client.sell(asset, amount, time))

    def check_win(self, order_id: str) -> dict:
        """Check the result of a trade."""
        return self._run(self._async_client.check_win(order_id))

    def balance(self) -> float:
        """Get current balance."""
        return self._run(self._async_client.balance())

    def candles(self, asset: str, period: int) -> List[dict]:
        """Get historical candles."""
        return self._run(self._async_client.candles(asset, period))

    def get_candles(self, asset: str, period: int, count: int = 100) -> List[dict]:
        """Get historical candles with count."""
        return self._run(self._async_client.get_candles(asset, period, count))

    def get_candles_live(self, asset: str, period: int) -> SyncCandleLiveIterator:
        """Get live candle updates."""
        async_gen = self._run(self._async_client.get_candles_live(asset, period))
        return SyncCandleLiveIterator(async_gen, self._loop)

    def subscribe_symbol(self, symbol: str) -> SyncSubscription:
        """Subscribe to price updates for a symbol."""
        sub = self._run(self._async_client.subscribe_symbol(symbol))
        return SyncSubscription(sub)

    def subscribe_raw(self) -> SyncRawSubscription:
        """Subscribe to all raw messages."""
        sub = self._run(self._async_client.subscribe_raw())
        return SyncRawSubscription(sub)

    def send_raw(self, message: str) -> None:
        """Send a raw message."""
        self._run(self._async_client.send_raw(message))

    def active_assets(self) -> List[dict]:
        """Get list of active assets."""
        return self._run(self._async_client.active_assets())

    def payout(self, asset: str) -> float:
        """Get payout for an asset."""
        return self._run(self._async_client.payout(asset))

    def history(self, limit: int = 100) -> List[dict]:
        """Get trade history."""
        return self._run(self._async_client.history(limit))

    def opened_deals(self) -> List[dict]:
        """Get opened deals."""
        return self._run(self._async_client.opened_deals())

    def closed_deals(self) -> List[dict]:
        """Get closed deals."""
        return self._run(self._async_client.closed_deals())

    def get_server_time(self) -> int:
        """Get server time."""
        return self._run(self._async_client.get_server_time())

    def raw_handler(self) -> RawHandlerSync:
        """Get raw handler for advanced operations."""
        handler = self._run(self._async_client.raw_handler())
        return RawHandlerSync(handler)

    def shutdown(self) -> None:
        """Close the connection and cleanup."""
        if self._async_client:
            self._run(self._async_client.shutdown())
        if self._loop and self._loop.is_running():
            self._loop.call_soon_threadsafe(self._loop.stop)

    def reconnect(self) -> None:
        """Reconnect to the server."""
        return self._run(self._async_client.reconnect())