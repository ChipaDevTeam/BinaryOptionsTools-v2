import asyncio
import json
import threading
from datetime import timedelta
from typing import Dict, List, Optional, Tuple, Union

from ..config import Config
from ..validator import Validator
from .asynchronous import PocketOptionAsync, RawHandler, Validator


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


class RawHandlerSync:
    """Synchronous handler for advanced raw WebSocket message operations."""

    def __init__(self, async_handler, loop):
        self._handler = async_handler
        self._loop = loop

    def _run(self, coro):
        return asyncio.run_coroutine_threadsafe(coro, self._loop).result()

    def send_text(self, message: str) -> None:
        self._run(self._handler.send_text(message))

    def send_binary(self, data: bytes) -> None:
        self._run(self._handler.send_binary(data))

    def send_and_wait(self, message: str) -> str:
        return self._run(self._handler.send_and_wait(message))

    def wait_next(self) -> str:
        return self._run(self._handler.wait_next())

    def subscribe(self):
        async_subscription = self._run(self._handler.subscribe())
        return SyncRawSubscription(async_subscription)

    def id(self) -> str:
        return self._handler.id()

    def close(self) -> None:
        self._run(self._handler.close())


class SyncRawSubscription:
    """
    Synchronous subscription wrapper for raw handler message streams.
    """

    def __init__(self, async_subscription):
        self.subscription = async_subscription

    def __iter__(self):
        return self

    def __aiter__(self):
        """Return the async iterator for the raw subscription."""
        return self.subscription

    def __next__(self):
        return next(self.subscription)


class PocketOption:
    def __init__(self, ssid: str, url: Optional[str] = None, config: Union[Config, dict, str] = None, **_):
        self._lock = threading.RLock()
        self._loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self._loop)
        self._loop_thread = threading.Thread(target=self._loop.run_forever, daemon=True)
        self._loop_thread.start()
        self._client = PocketOptionAsync(ssid, url=url, config=config)
        future = asyncio.run_coroutine_threadsafe(
            self._client.wait_for_assets(), self._loop
        )
        future.result()

    def __del__(self):
        self._cleanup_loop()

    def _cleanup_loop(self):
        loop = getattr(self, '_loop', None)
        if loop is None or loop.is_closed():
            return
        try:
            client = getattr(self, '_client', None)
            if client is not None:
                future = asyncio.run_coroutine_threadsafe(
                    client.shutdown(), loop
                )
                future.result(timeout=5)
        except Exception:
            pass
        loop.call_soon_threadsafe(loop.stop)
        thread = getattr(self, '_loop_thread', None)
        if thread is not None and thread.is_alive():
            thread.join(timeout=2)
        loop.close()

    @property
    def loop(self):
        return self._loop

    def _run(self, coro):
        """Schedule a coroutine on the background event loop and wait for the result."""
        return asyncio.run_coroutine_threadsafe(coro, self._loop).result()

    @property
    def client(self):
        return self._client

    @property
    def config(self):
        return self._client.config

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def close(self) -> None:
        with self._lock:
            self._cleanup_loop()

    def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        return self._run(self._client.buy(asset, amount, time, check_win))

    def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        return self._run(self._client.sell(asset, amount, time, check_win))

    def check_win(self, id: str) -> dict:
        return self._run(self._client.check_win(id))

    def get_deal_end_time(self, trade_id: str) -> Optional[int]:
        return self._run(self._client.get_deal_end_time(trade_id))

    def get_candles(self, asset: str, period: int, offset: int) -> List[Dict]:
        return self._run(self._client.get_candles(asset, period, offset))

    def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
        return self._run(self._client.get_candles_advanced(asset, period, offset, time))

    def candles(self, asset: str, period: int) -> List[Dict]:
        return self._run(self._client.candles(asset, period))

    def balance(self) -> float:
        return self._run(self._client.balance())

    def opened_deals(self) -> List[str]:
        return self._run(self._client.opened_deals())

    def get_opened_deal(self, trade_id: str) -> Optional[Dict]:
        return self._run(self._client.get_opened_deal(trade_id))

    def open_pending_order(
        self,
        open_type: int,
        amount: float,
        asset: str,
        open_time: int,
        open_price: float,
        timeframe: int,
        min_payout: int,
        command: int,
    ) -> Dict:
        return self._run(
            self._client.open_pending_order(
                open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
            )
        )

    def cancel_pending_order(self, ticket: str) -> Dict:
        return self._run(self._client.cancel_pending_order(ticket))

    def cancel_pending_orders(self, tickets: List[str]) -> List[Dict]:
        return self._run(self._client.cancel_pending_orders(tickets))

    def closed_deals(self) -> List[Dict]:
        return self._run(self._client.closed_deals())

    def get_closed_deal(self, trade_id: str) -> Optional[Dict]:
        return self._run(self._client.get_closed_deal(trade_id))

    def clear_closed_deals(self) -> None:
        self._run(self._client.clear_closed_deals())

    def payout(
        self, asset: Optional[Union[str, List[str]]] = None
    ) -> Union[Dict[str, Optional[int]], List[Optional[int]], int, None]:
        return self._run(self._client.payout(asset))

    def history(self, asset: str, period: int) -> List[Dict]:
        return self._run(self._client.history(asset, period))

    def compile_candles(self, asset: str, custom_period: int, lookback_period: int) -> List[Dict]:
        return self._run(self._client.compile_candles(asset, custom_period, lookback_period))

    def subscribe_symbol(self, asset: str) -> SyncSubscription:
        async def _sub():
            return await self._client.client.subscribe_symbol(asset)
        return SyncSubscription(self._run(_sub()))

    def subscribe_symbol_chunked(self, asset: str, chunk_size: int) -> SyncSubscription:
        async def _sub():
            return await self._client.client.subscribe_symbol_chunked(asset, chunk_size)
        return SyncSubscription(self._run(_sub()))

    def subscribe_symbol_timed(self, asset: str, time: timedelta) -> SyncSubscription:
        async def _sub():
            return await self._client.client.subscribe_symbol_timed(asset, time)
        return SyncSubscription(self._run(_sub()))

    def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> SyncSubscription:
        async def _sub():
            return await self._client.client.subscribe_symbol_time_aligned(asset, time)
        return SyncSubscription(self._run(_sub()))

    def get_server_time(self) -> int:
        return self._run(self._client.get_server_time())

    def get_pending_deals(self) -> List[Dict]:
        return self._run(self._client.get_pending_deals())

    def is_demo(self) -> bool:
        return self._client.is_demo()

    def is_connected(self) -> bool:
        return self._client.is_connected()

    def max_subscriptions(self) -> int:
        return self._client.max_subscriptions()

    def wait_for_assets(self, timeout: float = 60.0) -> None:
        self._run(self._client.wait_for_assets(timeout))

    def disconnect(self) -> None:
        self._run(self._client.disconnect())

    def connect(self) -> None:
        self._run(self._client.connect())

    def reconnect(self) -> None:
        self._run(self._client.reconnect())

    def unsubscribe(self, asset: str) -> None:
        self._run(self._client.unsubscribe(asset))

    def shutdown(self) -> None:
        self._run(self._client.shutdown())

    def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandlerSync":
        async_handler = self._run(self._client.create_raw_handler(validator, keep_alive))
        return RawHandlerSync(async_handler, self.loop)

    def send_raw_message(self, message: str) -> None:
        self._run(self._client.send_raw_message(message))

    def create_raw_order(self, message: str, validator: Validator) -> str:
        return self._run(self._client.create_raw_order(message, validator))

    def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        return self._run(self._client.create_raw_order_with_timeout(message, validator, timeout))

    def create_raw_order_with_timeout_and_retry(self, message: str, validator: Validator, timeout: timedelta) -> str:
        return self._run(self._client.create_raw_order_with_timeout_and_retry(message, validator, timeout))

    def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        async_iterator = self._run(self._client.create_raw_iterator(message, validator, timeout))
        return SyncRawSubscription(async_iterator)

    def active_assets(self) -> List[Dict]:
        return self._run(self._client.active_assets())
