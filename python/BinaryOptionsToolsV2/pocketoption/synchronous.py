import asyncio
import json
import threading
from datetime import timedelta
from typing import Dict, List, Optional, Tuple, Union

from ..config import Config
from ..validator import Validator as Validator
from .asynchronous import PocketOptionAsync as PocketOptionAsync


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
        """Send a text message through the raw WebSocket handler.

        Args:
            message: The text message to send.
        """
        self._run(self._handler.send_text(message))

    def send_binary(self, data: bytes) -> None:
        """Send binary data through the raw WebSocket handler.

        Args:
            data: The binary data to send.
        """
        self._run(self._handler.send_binary(data))

    def send_and_wait(self, message: str) -> str:
        """Send a text message and wait for a response.

        Args:
            message: The text message to send.

        Returns:
            The response string received from the server.
        """
        return self._run(self._handler.send_and_wait(message))

    def wait_next(self) -> str:
        """Wait for the next incoming message.

        Returns:
            The next message string received from the server.
        """
        return self._run(self._handler.wait_next())

    def subscribe(self):
        """Subscribe to the raw message stream.

        Returns:
            A SyncRawSubscription for iterating over incoming messages.
        """
        async_subscription = self._run(self._handler.subscribe())
        return SyncRawSubscription(async_subscription)

    def id(self) -> str:
        """Get the unique identifier of this raw handler.

        Returns:
            The handler ID as a string.
        """
        return self._handler.id()

    def close(self) -> None:
        """Close the raw handler and clean up resources."""
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
        """Initialize the synchronous PocketOption client.

        Creates a background event loop and thread, initializes the
        underlying async client, and waits for asset data to load.

        Args:
            ssid: The session ID for authentication.
            url: Optional custom WebSocket URL.
            config: Optional configuration as a Config object, a dict,
                or a path string.
            _: Additional keyword arguments forwarded to the async client.
        """
        self._lock = threading.RLock()
        self._loop = asyncio.new_event_loop()
        asyncio.set_event_loop(self._loop)
        self._loop_thread = threading.Thread(target=self._loop.run_forever, daemon=True)
        self._loop_thread.start()
        self._client = PocketOptionAsync(ssid, url=url, config=config)
        future = asyncio.run_coroutine_threadsafe(self._client.wait_for_assets(), self._loop)
        future.result()

    def __del__(self):
        self._cleanup_loop()

    def _cleanup_loop(self):
        loop = getattr(self, "_loop", None)
        if loop is None or loop.is_closed():
            return
        try:
            client = getattr(self, "_client", None)
            if client is not None:
                future = asyncio.run_coroutine_threadsafe(client.shutdown(), loop)
                future.result(timeout=5)
        except Exception:
            pass
        loop.call_soon_threadsafe(loop.stop)
        thread = getattr(self, "_loop_thread", None)
        if thread is not None and thread.is_alive():
            thread.join(timeout=2)
        loop.close()

    @property
    def loop(self):
        """Get the background asyncio event loop.

        Returns:
            The asyncio event loop running in the background thread.
        """
        return self._loop

    def _run(self, coro):
        """Schedule a coroutine on the background event loop and wait for the result."""
        return asyncio.run_coroutine_threadsafe(coro, self._loop).result()

    @property
    def client(self):
        """Get the underlying async PocketOption client.

        Returns:
            The PocketOptionAsync client instance.
        """
        return self._client

    @property
    def config(self):
        """Get the client configuration.

        Returns:
            The Config object associated with the async client.
        """
        return self._client.config

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def close(self) -> None:
        """Cleanly shut down the client and the background event loop.

        Acquires the internal reentrant lock and performs a full cleanup
        of the event loop, client resources, and background thread.
        """
        with self._lock:
            self._cleanup_loop()

    def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """Place a buy (call) option.

        Args:
            asset: The trading asset name (e.g. "EURUSD").
            amount: The investment amount.
            time: The expiration time in seconds.
            check_win: Whether to immediately check the trade result.

        Returns:
            A tuple of (trade_id, trade_details_dict).
        """
        return self._run(self._client.buy(asset, amount, time, check_win))

    def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """Place a sell (put) option.

        Args:
            asset: The trading asset name.
            amount: The investment amount.
            time: The expiration time in seconds.
            check_win: Whether to immediately check the trade result.

        Returns:
            A tuple of (trade_id, trade_details_dict).
        """
        return self._run(self._client.sell(asset, amount, time, check_win))

    def check_win(self, id: str) -> dict:
        """Check the result of a completed trade.

        Args:
            id: The trade ID to check.

        Returns:
            A dictionary with the win/loss result and details.
        """
        return self._run(self._client.check_win(id))

    def get_deal_end_time(self, trade_id: str) -> Optional[int]:
        """Get the end time of a deal.

        Args:
            trade_id: The trade identifier.

        Returns:
            The end time as a Unix timestamp, or None if not available.
        """
        return self._run(self._client.get_deal_end_time(trade_id))

    def get_candles(self, asset: str, period: int, offset: int) -> List[Dict]:
        """Get historical candle data for an asset.

        Args:
            asset: The trading asset name.
            period: The candle period in seconds.
            offset: The offset from the current time in seconds.

        Returns:
            A list of candle dictionaries.
        """
        return self._run(self._client.get_candles(asset, period, offset))

    def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
        """Get historical candle data with a specific reference time.

        Args:
            asset: The trading asset name.
            period: The candle period in seconds.
            offset: The offset from the reference time in seconds.
            time: The reference Unix timestamp.

        Returns:
            A list of candle dictionaries.
        """
        return self._run(self._client.get_candles_advanced(asset, period, offset, time))

    def candles(self, asset: str, period: int) -> List[Dict]:
        """Get the most recent candles for an asset.

        Args:
            asset: The trading asset name.
            period: The candle period in seconds.

        Returns:
            A list of candle dictionaries.
        """
        return self._run(self._client.candles(asset, period))

    def balance(self) -> float:
        """Get the current account balance.

        Returns:
            The account balance as a float.
        """
        return self._run(self._client.balance())

    def opened_deals(self) -> List[str]:
        """Get a list of currently open deal IDs.

        Returns:
            A list of open trade ID strings.
        """
        return self._run(self._client.opened_deals())

    def get_opened_deal(self, trade_id: str) -> Optional[Dict]:
        """Get details of a specific open deal.

        Args:
            trade_id: The trade identifier.

        Returns:
            A dictionary with deal details, or None if the deal is not found.
        """
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
        """Open a pending order with specified parameters.

        Args:
            open_type: The order type identifier.
            amount: The investment amount.
            asset: The trading asset name.
            open_time: The scheduled open time.
            open_price: The target open price.
            timeframe: The candle timeframe.
            min_payout: The minimum acceptable payout.
            command: The command type.

        Returns:
            A dictionary with the order result.
        """
        return self._run(
            self._client.open_pending_order(
                open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
            )
        )

    def cancel_pending_order(self, ticket: str) -> Dict:
        """Cancel a specific pending order.

        Args:
            ticket: The order ticket/identifier to cancel.

        Returns:
            A dictionary with the cancellation result.
        """
        return self._run(self._client.cancel_pending_order(ticket))

    def cancel_pending_orders(self, tickets: List[str]) -> List[Dict]:
        """Cancel multiple pending orders.

        Args:
            tickets: A list of order ticket/identifiers to cancel.

        Returns:
            A list of dictionaries with the cancellation results.
        """
        return self._run(self._client.cancel_pending_orders(tickets))

    def closed_deals(self) -> List[Dict]:
        """Get a list of closed deals.

        Returns:
            A list of dictionaries with closed deal details.
        """
        return self._run(self._client.closed_deals())

    def get_closed_deal(self, trade_id: str) -> Optional[Dict]:
        """Get details of a specific closed deal.

        Args:
            trade_id: The trade identifier.

        Returns:
            A dictionary with deal details, or None if not found.
        """
        return self._run(self._client.get_closed_deal(trade_id))

    def clear_closed_deals(self) -> None:
        """Clear the list of closed deals from local storage."""
        self._run(self._client.clear_closed_deals())

    def payout(
        self, asset: Optional[Union[str, List[str]]] = None
    ) -> Union[Dict[str, Optional[int]], List[Optional[int]], int, None]:
        """Get payout information for one or more assets.

        Args:
            asset: The asset name, a list of asset names, or None for all assets.

        Returns:
            Payout data: a dict mapping asset names to payouts, a list of
            payouts, a single payout value, or None.
        """
        return self._run(self._client.payout(asset))

    def history(self, asset: str, period: int) -> List[Dict]:
        """Get historical trade data for an asset.

        Args:
            asset: The trading asset name.
            period: The time period in seconds.

        Returns:
            A list of historical trade dictionaries.
        """
        return self._run(self._client.history(asset, period))

    def compile_candles(self, asset: str, custom_period: int, lookback_period: int) -> List[Dict]:
        """Compile candles from raw data with a custom aggregation period.

        Args:
            asset: The trading asset name.
            custom_period: The target candle period in seconds.
            lookback_period: How far back to look for data in seconds.

        Returns:
            A list of compiled candle dictionaries.
        """
        return self._run(self._client.compile_candles(asset, custom_period, lookback_period))

    def subscribe_symbol(self, asset: str) -> SyncSubscription:
        """Subscribe to real-time price updates for a symbol.

        Args:
            asset: The trading asset name to subscribe to.

        Returns:
            A SyncSubscription for iterating over price updates.
        """

        async def _sub():
            return await self._client.client.subscribe_symbol(asset)

        return SyncSubscription(self._run(_sub()))

    def subscribe_symbol_chunked(self, asset: str, chunk_size: int) -> SyncSubscription:
        """Subscribe to real-time price updates with chunked delivery.

        Args:
            asset: The trading asset name to subscribe to.
            chunk_size: The number of updates per chunk.

        Returns:
            A SyncSubscription for iterating over batched price updates.
        """

        async def _sub():
            return await self._client.client.subscribe_symbol_chunked(asset, chunk_size)

        return SyncSubscription(self._run(_sub()))

    def subscribe_symbol_timed(self, asset: str, time: timedelta) -> SyncSubscription:
        """Subscribe to periodic real-time price updates.

        Args:
            asset: The trading asset name to subscribe to.
            time: The interval between updates.

        Returns:
            A SyncSubscription for iterating over timed price updates.
        """

        async def _sub():
            return await self._client.client.subscribe_symbol_timed(asset, time)

        return SyncSubscription(self._run(_sub()))

    def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> SyncSubscription:
        """Subscribe to time-aligned periodic price updates.

        Args:
            asset: The trading asset name to subscribe to.
            time: The interval between updates, aligned to the clock.

        Returns:
            A SyncSubscription for iterating over time-aligned price updates.
        """

        async def _sub():
            return await self._client.client.subscribe_symbol_time_aligned(asset, time)

        return SyncSubscription(self._run(_sub()))

    def get_server_time(self) -> int:
        """Get the current server time.

        Returns:
            The current server time as a Unix timestamp.
        """
        return self._run(self._client.get_server_time())

    def get_pending_deals(self) -> List[Dict]:
        """Get a list of pending deals.

        Returns:
            A list of dictionaries with pending deal details.
        """
        return self._run(self._client.get_pending_deals())

    def is_demo(self) -> bool:
        """Check if the account is a demo account.

        Returns:
            True if the account is a demo account, False otherwise.
        """
        return self._client.is_demo()

    def is_connected(self) -> bool:
        """Check if the client is connected to the server.

        Returns:
            True if connected, False otherwise.
        """
        return self._client.is_connected()

    def max_subscriptions(self) -> int:
        """Get the maximum number of concurrent subscriptions.

        Returns:
            The maximum number of allowed subscriptions.
        """
        return self._client.max_subscriptions()

    def wait_for_assets(self, timeout: float = 60.0) -> None:
        """Wait for asset data to finish loading.

        Args:
            timeout: Maximum time to wait in seconds (default 60.0).
        """
        self._run(self._client.wait_for_assets(timeout))

    def disconnect(self) -> None:
        """Disconnect from the server."""
        self._run(self._client.disconnect())

    def connect(self) -> None:
        """Connect to the server."""
        self._run(self._client.connect())

    def reconnect(self) -> None:
        """Disconnect and reconnect to the server."""
        self._run(self._client.reconnect())

    def unsubscribe(self, asset: str) -> None:
        """Unsubscribe from real-time updates for a symbol.

        Args:
            asset: The trading asset name to unsubscribe from.
        """
        self._run(self._client.unsubscribe(asset))

    def shutdown(self) -> None:
        """Shut down the client and release all resources."""
        self._run(self._client.shutdown())

    def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandlerSync":
        """Create a synchronous raw WebSocket message handler.

        Args:
            validator: A Validator instance for message validation.
            keep_alive: Optional keep-alive message string.

        Returns:
            A RawHandlerSync instance wrapping the async raw handler.
        """
        async_handler = self._run(self._client.create_raw_handler(validator, keep_alive))
        return RawHandlerSync(async_handler, self.loop)

    def send_raw_message(self, message: str) -> None:
        """Send a raw message through the WebSocket connection.

        Args:
            message: The raw message string to send.
        """
        self._run(self._client.send_raw_message(message))

    def create_raw_order(self, message: str, validator: Validator) -> str:
        """Create a raw order and wait for the response.

        Args:
            message: The raw order message string.
            validator: A Validator instance for validating the response.

        Returns:
            The validated response string.
        """
        return self._run(self._client.create_raw_order(message, validator))

    def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Create a raw order with a custom timeout.

        Args:
            message: The raw order message string.
            validator: A Validator instance for validating the response.
            timeout: The maximum time to wait for a response.

        Returns:
            The validated response string.
        """
        return self._run(self._client.create_raw_order_with_timeout(message, validator, timeout))

    def create_raw_order_with_timeout_and_retry(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Create a raw order with timeout and automatic retry on failure.

        Args:
            message: The raw order message string.
            validator: A Validator instance for validating the response.
            timeout: The maximum time to wait for each attempt.

        Returns:
            The validated response string.
        """
        return self._run(self._client.create_raw_order_with_timeout_and_retry(message, validator, timeout))

    def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        """Create an iterator for streaming raw messages.

        Args:
            message: The raw message string to send.
            validator: A Validator instance for message validation.
            timeout: Optional timeout for each iteration step.

        Returns:
            A SyncRawSubscription for iterating over the message stream.
        """
        async_iterator = self._run(self._client.create_raw_iterator(message, validator, timeout))
        return SyncRawSubscription(async_iterator)

    def active_assets(self) -> List[Dict]:
        """Get the list of currently active trading assets.

        Returns:
            A list of dictionaries with active asset details.
        """
        return self._run(self._client.active_assets())
