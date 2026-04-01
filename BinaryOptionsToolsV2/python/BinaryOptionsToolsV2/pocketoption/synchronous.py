import asyncio
import json
import queue
import threading
from datetime import timedelta
from typing import Dict, List, Optional, Tuple, Union

from ..config import Config
from ..validator import Validator
from .asynchronous import PocketOptionAsync


class SyncSubscription:
    """Synchronous wrapper around an async subscription iterator.

    Uses a background task on the event loop to continuously pull items from
    the async generator into a thread-safe queue, allowing synchronous iteration
    without blocking the event loop.
    """

    def __init__(self, subscription, loop):
        self.subscription = subscription
        self._loop = loop
        self._queue = queue.Queue()
        self._feeder_task = None
        self._start_feeder()

    def _start_feeder(self):
        """Start a background task that feeds items from the async generator into the queue."""

        async def _feeder():
            try:
                async for item in self.subscription:
                    self._queue.put(item)
            except Exception as e:
                self._queue.put(e)
            finally:
                self._queue.put(StopIteration)

        self._feeder_task = asyncio.run_coroutine_threadsafe(_feeder(), self._loop)

    def cancel(self):
        """Cancel the background feeder task to prevent resource leaks.

        Call this method when you stop iterating before the subscription
        is exhausted. This cancels the internal asyncio task that pumps
        items from the async generator into the queue.

        Note: This is safe to call multiple times. If the task is already
        done or cancelled, this is a no-op.
        """
        if self._feeder_task and not self._feeder_task.done():
            self._feeder_task.cancel()

    def __iter__(self):
        return self

    def __aiter__(self):
        """Return the async iterator for the subscription."""
        return self.subscription

    def __next__(self, timeout=60):
        try:
            item = self._queue.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError(f"Subscription timed out after {timeout} seconds waiting for data")
        if item is StopIteration:
            raise StopIteration
        if isinstance(item, Exception):
            raise item
        return json.loads(item)


class SyncRawSubscription:
    """
    Synchronous subscription wrapper for raw handler message streams.
    Uses the same queue-based approach as SyncSubscription.
    """

    def __init__(self, async_subscription, loop):
        self.subscription = async_subscription
        self._loop = loop
        self._queue = queue.Queue()
        self._feeder_task = None
        self._start_feeder()

    def _start_feeder(self):
        """Start a background task that feeds items from the async iterator into the queue."""

        async def _feeder():
            try:
                async for item in self.subscription:
                    self._queue.put(item)
            except Exception as e:
                self._queue.put(e)
            finally:
                self._queue.put(StopIteration)

        self._feeder_task = asyncio.run_coroutine_threadsafe(_feeder(), self._loop)

    def cancel(self):
        """Cancel the background feeder task to prevent resource leaks.

        Call this method when you stop iterating before the subscription
        is exhausted. This cancels the internal asyncio task that pumps
        items from the async iterator into the queue.

        Note: This is safe to call multiple times. If the task is already
        done or cancelled, this is a no-op.
        """
        if self._feeder_task and not self._feeder_task.done():
            self._feeder_task.cancel()

    def __iter__(self):
        return self

    def __aiter__(self):
        """Return the async iterator for the raw subscription."""
        return self.subscription

    def __next__(self, timeout=60):
        try:
            item = self._queue.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError(f"Subscription timed out after {timeout} seconds waiting for data")
        if item is StopIteration:
            raise StopIteration
        if isinstance(item, Exception):
            raise item
        return item


class RawHandlerSync:
    """
    Synchronous handler for advanced raw WebSocket message operations.

    Provides low-level access to send messages and receive filtered responses
    based on a validator. Each handler maintains its own message stream.
    """

    def __init__(self, async_handler, loop, lock=None):
        self._handler = async_handler
        self._loop = loop
        self._lock = lock

    def _run_async(self, coro):
        """Submit a coroutine to the background loop and wait for the result."""
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result(timeout=300)

    def send_text(self, message: str) -> None:
        """Send a text message through this handler."""
        if self._lock:
            with self._lock:
                self._run_async(self._handler.send_text(message))
        else:
            self._run_async(self._handler.send_text(message))

    def send_binary(self, data: bytes) -> None:
        """Send a binary message through this handler."""
        if self._lock:
            with self._lock:
                self._run_async(self._handler.send_binary(data))
        else:
            self._run_async(self._handler.send_binary(data))

    def send_and_wait(self, message: str) -> str:
        """Send a message and wait for the next matching response."""
        if self._lock:
            with self._lock:
                return self._run_async(self._handler.send_and_wait(message))
        else:
            return self._run_async(self._handler.send_and_wait(message))

    def wait_next(self) -> str:
        """Wait for the next message that matches this handler's validator."""
        if self._lock:
            with self._lock:
                return self._run_async(self._handler.wait_next())
        else:
            return self._run_async(self._handler.wait_next())

    def subscribe(self):
        """Subscribe to messages matching this handler's validator."""
        if self._lock:
            with self._lock:
                async_subscription = self._run_async(self._handler.subscribe())
        else:
            async_subscription = self._run_async(self._handler.subscribe())
        return SyncRawSubscription(async_subscription, self._loop)

    def id(self) -> str:
        """Get the unique ID of this handler."""
        return self._handler.id()

    def close(self) -> None:
        """Close this handler and clean up resources."""
        if self._lock:
            with self._lock:
                self._run_async(self._handler.close())
        else:
            self._run_async(self._handler.close())


class PocketOption:
    def __init__(self, ssid: str, url: Optional[str] = None, config: Optional[Union[Config, dict, str]] = None, **_):
        """
        Initializes a new PocketOption instance.

        This class provides a synchronous wrapper around the asynchronous PocketOptionAsync class,
        making it easier to interact with the Pocket Option trading platform in synchronous code.
        It supports custom WebSocket URLs and configuration options for fine-tuning the connection behavior.

        Args:
            ssid (str): Session ID for authentication with Pocket Option platform
            url (str | None, optional): Custom WebSocket server URL. Defaults to None, using platform's default URL.
            config (Config | dict | str, optional): Configuration options. Can be provided as:
                - Config object: Direct instance of Config class
                - dict: Dictionary of configuration parameters
                - str: JSON string containing configuration parameters

        Examples:
            Basic usage:
            ```python
            client = PocketOption("your-session-id")
            balance = client.balance()
            print(f"Current balance: {balance}")
            ```
        """
        self.loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self.loop)
        self._lock = threading.RLock()
        self._loop_thread = threading.Thread(target=self._run_loop, daemon=True)
        self._loop_thread.start()
        # Small delay to ensure the background thread's run_forever is active
        import time

        time.sleep(0.01)
        self._client = PocketOptionAsync(ssid, url=url, config=config)
        self._run_async(self._client.wait_for_assets())

    def _run_loop(self):
        """Run the event loop in a background thread."""
        asyncio.set_event_loop(self.loop)
        assert self.loop is not None
        self.loop.run_forever()

    def _run_async(self, coro, timeout: Union[int, float] = 300):
        """Submit a coroutine to the background loop and wait for the result."""
        assert self.loop is not None
        future = asyncio.run_coroutine_threadsafe(coro, self.loop)
        return future.result(timeout=timeout)

    @property
    def client(self):
        """Returns the underlying PocketOptionAsync client."""
        return self._client

    @property
    def config(self):
        """Returns the configuration object."""
        return self._client.config

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def close(self) -> None:
        """Explicitly closes the client and its event loop."""
        with self._lock:
            if self.loop is None or self.loop.is_closed():
                return
            # Cancel all pending tasks on the loop before shutdown
            try:
                pending = asyncio.run_coroutine_threadsafe(self._cancel_all_tasks(), self.loop)
                pending.result(timeout=5)
            except Exception:
                pass
            try:
                self._run_async(self._client.shutdown(), timeout=10)
            except Exception:
                pass
            if not self.loop.is_closed():
                self.loop.call_soon_threadsafe(self.loop.stop)
            if hasattr(self, "_loop_thread") and self._loop_thread.is_alive():
                self._loop_thread.join(timeout=5)
            if not self.loop.is_closed():
                self.loop.close()
            self.loop = None

    def __del__(self):
        """Destructor safety net for event loop cleanup.

        Ensures the background event loop thread is stopped and closed
        even if the caller forgot to invoke close() or the context manager
        exit was skipped due to an exception. This prevents thread leaks
        in long-running processes or test suites.

        Unlike close(), __del__ uses aggressive timeouts and skips the
        client shutdown (which requires a network connection that may
        already be dropped). It only stops the loop and joins the thread.

        Note: Relying on __del__ is not deterministic. Always prefer
        explicit close() or using the context manager protocol.
        """
        try:
            if hasattr(self, "loop") and self.loop is not None and not self.loop.is_closed():
                with self._lock:
                    if self.loop is None or self.loop.is_closed():
                        return
                    try:
                        pending = asyncio.run_coroutine_threadsafe(
                            self._cancel_all_tasks(), self.loop
                        )
                        pending.result(timeout=2)
                    except Exception:
                        pass
                    if not self.loop.is_closed():
                        self.loop.call_soon_threadsafe(self.loop.stop)
                    if hasattr(self, "_loop_thread") and self._loop_thread.is_alive():
                        self._loop_thread.join(timeout=2)
                    if not self.loop.is_closed():
                        self.loop.close()
                    self.loop = None
        except Exception:
            pass

    async def _cancel_all_tasks(self):
        """Cancel all pending asyncio tasks on the event loop.

        Python 3.11+ asyncio.Task.cancel() recursively cancels child tasks,
        which causes RecursionError with deeply nested task trees. We work
        around this by temporarily increasing the recursion limit during
        cancellation, then stopping the event loop immediately.
        """
        import sys

        try:
            tasks = [t for t in asyncio.all_tasks(self.loop) if not t.done()]
        except (RuntimeError, RecursionError):
            self.loop.stop()
            return
        if not tasks:
            self.loop.stop()
            return
        # Temporarily increase recursion limit to handle deeply nested cancel trees
        old_limit = sys.getrecursionlimit()
        try:
            sys.setrecursionlimit(max(old_limit * 4, 10000))
            for task in tasks:
                task.cancel(msg="PocketOption client shutting down")
        except RecursionError:
            pass
        finally:
            sys.setrecursionlimit(old_limit)
        self.loop.stop()

    def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        with self._lock:
            return self._run_async(self._client.buy(asset, amount, time, check_win))

    def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        with self._lock:
            return self._run_async(self._client.sell(asset, amount, time, check_win))

    def check_win(self, id: str, timeout_seconds: Optional[int] = None) -> dict:
        with self._lock:
            return self._run_async(self._client.check_win(id, timeout_seconds), timeout=timeout_seconds or 300)

    def get_deal_end_time(self, trade_id: str) -> Optional[int]:
        with self._lock:
            return self._run_async(self._client.get_deal_end_time(trade_id))

    def get_candles(self, asset: str, period: int, offset: int) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.get_candles(asset, period, offset))

    def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.get_candles_advanced(asset, period, offset, time))

    def candles(self, asset: str, period: int) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.candles(asset, period))

    def balance(self) -> float:
        with self._lock:
            return self._run_async(self._client.balance())

    def opened_deals(self) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.opened_deals())

    def get_pending_deals(self) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.get_pending_deals())

    def open_pending_order(
        self, open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
    ) -> Dict:
        with self._lock:
            return self._run_async(
                self._client.open_pending_order(
                    open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
                )
            )

    def cancel_pending_order(self, ticket: str) -> Dict:
        with self._lock:
            return self._run_async(self._client.cancel_pending_order(ticket))

    def cancel_pending_orders(self, tickets: List[str]) -> Dict:
        with self._lock:
            return self._run_async(self._client.cancel_pending_orders(tickets))

    def closed_deals(self) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.closed_deals())

    def clear_closed_deals(self) -> None:
        with self._lock:
            self._run_async(self._client.clear_closed_deals())

    def payout(self, asset: Optional[Union[str, List[str]]] = None):
        with self._lock:
            return self._run_async(self._client.payout(asset))

    def history(self, asset: str, period: int) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.history(asset, period))

    def compile_candles(self, asset: str, custom_period: int, lookback_period: int) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.compile_candles(asset, custom_period, lookback_period))

    def subscribe_symbol(self, asset: str) -> SyncSubscription:
        with self._lock:
            subscription = self._run_async(self._client._subscribe_symbol_inner(asset))
        return SyncSubscription(subscription, self.loop)

    def subscribe_symbol_chuncked(self, asset: str, chunck_size: int) -> SyncSubscription:
        with self._lock:
            subscription = self._run_async(self._client._subscribe_symbol_chuncked_inner(asset, chunck_size))
        return SyncSubscription(subscription, self.loop)

    def subscribe_symbol_timed(self, asset: str, time: timedelta) -> SyncSubscription:
        with self._lock:
            subscription = self._run_async(self._client._subscribe_symbol_timed_inner(asset, time))
        return SyncSubscription(subscription, self.loop)

    def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> SyncSubscription:
        with self._lock:
            subscription = self._run_async(self._client._subscribe_symbol_time_aligned_inner(asset, time))
        return SyncSubscription(subscription, self.loop)

    def get_server_time(self) -> int:
        with self._lock:
            return self._run_async(self._client.get_server_time())

    def is_demo(self) -> bool:
        return self._client.is_demo()

    def is_connected(self) -> bool:
        return self._client.is_connected()

    def is_ssid_valid(self) -> bool:
        return self._client.is_ssid_valid()

    def max_subscriptions(self) -> int:
        return self._client.max_subscriptions()

    def wait_for_assets(self, timeout: float = 60.0) -> None:
        with self._lock:
            self._run_async(self._client.wait_for_assets(timeout), timeout=timeout)

    def disconnect(self) -> None:
        with self._lock:
            self._run_async(self._client.disconnect())

    def connect(self) -> None:
        with self._lock:
            self._run_async(self._client.connect())

    def reconnect(self) -> None:
        with self._lock:
            self._run_async(self._client.reconnect())

    def unsubscribe(self, asset: str) -> None:
        with self._lock:
            self._run_async(self._client.unsubscribe(asset))

    def shutdown(self) -> None:
        with self._lock:
            self._run_async(self._client.shutdown())

    def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> RawHandlerSync:
        with self._lock:
            async_handler = self._run_async(self._client.create_raw_handler(validator, keep_alive))
        return RawHandlerSync(async_handler, self.loop, self._lock)

    def send_raw_message(self, message: str) -> None:
        with self._lock:
            self._run_async(self._client.send_raw_message(message))

    def create_raw_order(self, message: str, validator: Validator) -> str:
        with self._lock:
            return self._run_async(self._client.create_raw_order(message, validator))

    def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        with self._lock:
            return self._run_async(self._client.create_raw_order_with_timeout(message, validator, timeout))

    def create_raw_order_with_timeout_and_retry(self, message: str, validator: Validator, timeout: timedelta) -> str:
        with self._lock:
            return self._run_async(self._client.create_raw_order_with_timeout_and_retry(message, validator, timeout))

    def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        with self._lock:
            async_iterator = self._run_async(self._client.create_raw_iterator(message, validator, timeout))
        return SyncRawSubscription(async_iterator, self.loop)

    def active_assets(self) -> List[Dict]:
        with self._lock:
            return self._run_async(self._client.active_assets())
