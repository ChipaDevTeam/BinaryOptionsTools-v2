import asyncio
import json
import threading
import sys
import warnings
from datetime import timedelta
from typing import Dict, List, Optional, Tuple, Union
from ..config import Config
from ..validator import Validator as Validator
from .asynchronous import PocketOptionAsync as PocketOptionAsync
from .models import Trade, TradeResult


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
                return await self.async_gen.__anext__()
            except StopAsyncIteration:
                raise StopIteration

        try:
            return asyncio.run_coroutine_threadsafe(get_next(), self.loop).result()
        except StopIteration:
            raise StopIteration

    def close(self) -> None:
        """Close the underlying async feed and release its subscription."""
        asyncio.run_coroutine_threadsafe(self.async_gen.aclose(), self.loop).result()


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
        loop = asyncio.new_event_loop()
        self._loop = loop

        def _run_loop(loop_to_run):
            asyncio.set_event_loop(loop_to_run)
            loop_to_run.run_forever()

        self._loop_thread = threading.Thread(target=_run_loop, args=(loop,), daemon=True)
        self._loop_thread.start()
        self._client = PocketOptionAsync(ssid, url=url, config=config)
        future = asyncio.run_coroutine_threadsafe(self._client.wait_for_assets(), self._loop)
        future.result()

    @classmethod
    def try_demo(cls, token: Optional[str] = None) -> "PocketOption":
        """Create a client on PocketOption's free "try demo" (guest) session.

        Args:
            token: An existing guest token. When omitted, one is generated locally,
                exactly like PocketOption's own "Try demo" client.

        Returns:
            A connected client with assets and balance already loaded.
        """
        self = cls.__new__(cls)
        self._lock = threading.RLock()
        loop = asyncio.new_event_loop()
        self._loop = loop

        def _run_loop(loop_to_run):
            asyncio.set_event_loop(loop_to_run)
            loop_to_run.run_forever()

        self._loop_thread = threading.Thread(target=_run_loop, args=(loop,), daemon=True)
        self._loop_thread.start()
        try:
            self._client = asyncio.run_coroutine_threadsafe(PocketOptionAsync.try_demo(token), self._loop).result()
        except Exception:
            # Don't leak the background loop/thread when setup fails.
            self._cleanup_loop()
            raise
        return self

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
            # During interpreter shutdown, joining threads raises PythonFinalizationError.
            # The daemon thread will be killed automatically, so just skip the join.
            if not sys.is_finalizing():
                thread.join(timeout=2)

        try:
            loop.close()
        except Exception:
            pass

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

    def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> TradeResult:
        """Place a buy (call) order.

        Args:
            asset: Trading asset symbol (e.g. ``"EURUSD_otc"``).
            amount: Amount to invest.
            time: Trade duration in seconds.
            check_win: When ``True``, block until the trade settles and return
                the settled :class:`Trade` instead of the opening order.

        Returns:
            ``(trade_id, trade)`` where ``trade`` is a :class:`Trade`.
        """
        return self._run(self._client.buy(asset, amount, time, check_win))

    def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> TradeResult:
        """Place a sell (put) order.

        Args:
            asset: Trading asset symbol (e.g. ``"EURUSD_otc"``).
            amount: Amount to invest.
            time: Trade duration in seconds.
            check_win: When ``True``, block until the trade settles and return
                the settled :class:`Trade` instead of the opening order.

        Returns:
            ``(trade_id, trade)`` where ``trade`` is a :class:`Trade`.
        """
        return self._run(self._client.sell(asset, amount, time, check_win))

    def check_win(self, id: str, timeout_seconds: Optional[int] = None) -> Trade:
        """Wait for and return the settled result of a trade.

        Args:
            id: Trade identifier (as returned by :meth:`buy`/:meth:`sell`).
            timeout_seconds: Maximum seconds to wait. ``None`` uses the configured
                default (300s); ``0`` waits indefinitely.

        Returns:
            The settled :class:`Trade`, with ``result`` set to
            ``"win"``, ``"loss"`` or ``"draw"``.
        """
        return self._run(self._client.check_win(id, timeout_seconds))

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
            offset: The number of periods to look back.

        Returns:
            A list of candle dictionaries.

        Note:
            WARNING: This function only fetches closed historical candles and is intended
            for training models, backtesting, or historical analysis. It is NOT designed
            for real-time/live trading as it does not include the current forming candle
            and can introduce gaps if called sequentially during live trading.
            For live gap-free candle feeds, use `get_candles_live()` instead.
        """
        warnings.warn(
            "get_candles() is deprecated and will be removed in a new release. "
            "Please use get_candles_live() for live gap-free candles instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        # offset = number of periods back, period = candle timeframe in seconds
        lookback_seconds = offset * period
        hours = max(0.1, lookback_seconds / 3600.0)
        iterator = self.get_candles_live(asset, period, hours=hours, max_rows=offset)
        try:
            closed, _forming = next(iterator)
        finally:
            # Close the feed even when the first `next` raises, otherwise the
            # subscription task behind it is left running.
            iterator.close()
        return closed

    def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
        """Get historical candle data with a specific reference time.

        Args:
            asset: The trading asset name.
            period: The candle period in seconds.
            offset: The number of periods to look back from reference time.
            time: The reference Unix timestamp.

        Returns:
            A list of candle dictionaries.

        Note:
            This method only returns closed candles for historical analysis.
            Use :meth:`candles` (or :meth:`stream_candles` for live data) instead.
        """
        warnings.warn(
            "get_candles_advanced() is deprecated; use candles() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        # The raw binding takes (asset, period, offset, time).
        return self._run(self._client.get_candles_advanced(asset, period, offset, time))

    def candles(
        self,
        asset: str,
        period: int,
        count: int = 100,
        end_time: Optional[int] = None,
    ) -> List[Dict]:
        """Fetch closed historical candles for an asset.

        Args:
            asset: Trading asset (e.g. "EURUSD_otc").
            period: Candle timeframe in seconds (e.g. 60 for 1-minute candles).
            count: Number of closed candles to return. Defaults to 100.
            end_time: Optional Unix timestamp to end the window at. When
                omitted, the most recent candles are returned.

        Returns:
            List of closed candles, each with 'time', 'open', 'high',
            'low', 'close'.

        Example:
            ```python
            with PocketOption(ssid) as client:
                candles = client.candles("EURUSD_otc", 60, count=200)
            ```
        """
        return self._run(self._client.candles(asset, period, count=count, end_time=end_time))

    def stream_candles(
        self,
        asset: str,
        period: int,
        *,
        history: float = 2.0,
        max_rows: int = 100,
    ) -> SyncCandleLiveIterator:
        """Stream gap-free live candles for an asset.

        Backfills `history` hours of closed candles, then yields updated
        (closed_candles, forming_candle) tuples as ticks arrive.

        Args:
            asset: Trading asset (e.g. "EURUSD_otc").
            period: Candle timeframe in seconds.
            history: Hours of history to backfill. Defaults to 2.0.
            max_rows: Maximum number of closed candles to retain.

        Returns:
            An iterator yielding (closed_candles, forming_candle).

        Example:
            ```python
            with PocketOption(ssid) as client:
                for candles, forming in client.stream_candles("EURUSD_otc", 60):
                    ...
            ```
        """
        return self.get_candles_live(asset, period, hours=history, max_rows=max_rows)

    def ticks(self, asset: str, seconds: int) -> List[Tuple[int, float]]:
        """Fetch historical raw ticks for an asset.

        Args:
            asset: Trading asset (e.g. "USDCHF_otc").
            seconds: Seconds of tick history to fetch.

        Returns:
            List of (timestamp, price) tuples sorted by timestamp.

        Example:
            ```python
            with PocketOption(ssid) as client:
                ticks = client.ticks("USDCHF_otc", 300)
            ```
        """
        return self._run(self._client.ticks(asset, seconds))

    def get_candles_live(
        self,
        asset: str,
        period: int,
        hours: float = 2.0,
        max_rows: int = 100,
    ) -> SyncCandleLiveIterator:
        """Fetches historical backfill and streams live candles.

        This method subscribes to raw ticks first and buffers them, then fetches
        historical candles, merges them, replays the buffered ticks, and returns
        a generator yielding updated candles (both closed historical candles and
        the current forming candle) in real-time.

        Args:
            asset: Trading asset (e.g., "EURUSD_otc")
            period: Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            hours: Hours of history to backfill. Defaults to 2.0.
            max_rows: Maximum number of closed candles to retain in history. Defaults to 100.

        Returns:
            An iterator yielding (closed_candles, forming_candle).
        """

        async def create_gen():
            return self._client.get_candles_live(
                asset=asset,
                period=period,
                hours=hours,
                max_rows=max_rows,
            )

        async_gen = self._run(create_gen())
        return SyncCandleLiveIterator(async_gen, self.loop)

    def balance(self) -> float:
        """Get the current account balance.

        Returns:
            The account balance as a float.
        """
        return self._run(self._client.balance())

    def opened_deals(self) -> List[Dict]:
        """Get the currently open deals.

        Returns:
            One dictionary per open deal, ordered by deal id. Each entry carries
            the deal fields, including `id`, `asset`, `amount` and `direction`.
        """
        return self._run(self._client.opened_deals())

    def get_opened_deal(self, trade_id: str) -> Optional[Trade]:
        """Get details of a specific open deal.

        Args:
            trade_id: The trade identifier.

        Returns:
            A :class:`Trade` with the deal details, or None if not found.
        """
        return self._run(self._client.get_opened_deal(trade_id))

    def open_pending_order(
        self,
        *,
        open_type: int,
        amount: float,
        asset: str,
        open_time: str,
        open_price: float,
        timeframe: int,
        min_payout: int,
        command: int,
    ) -> Dict:
        """Place a pending order that triggers on a time or a price.

        All fields are keyword-only so the eight similar scalars cannot be
        transposed at the call site.

        Args:
            open_type: ``0`` triggers at ``open_time``, ``1`` triggers at ``open_price``.
            amount: Amount to invest.
            asset: Trading asset symbol (e.g. ``"EURUSD_otc"``).
            open_time: Trigger time, ``"YYYY-MM-DD HH:MM:SS"`` in UTC for
                time-based orders or ``"0"`` for price-based orders.
            open_price: Trigger price for price-based orders, ``0`` for time-based.
            timeframe: Trade duration in seconds.
            min_payout: Minimum payout percentage required to open the order.
            command: ``0`` for call/buy, ``1`` for put/sell.

        Returns:
            The created pending order.
        """
        return self._run(
            self._client.open_pending_order(
                open_type=open_type,
                amount=amount,
                asset=asset,
                open_time=open_time,
                open_price=open_price,
                timeframe=timeframe,
                min_payout=min_payout,
                command=command,
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

    def cancel_pending_orders(self, tickets: List[str]) -> Dict:
        """Cancel multiple pending orders in one batch.

        Args:
            tickets: A list of order ticket/identifiers to cancel.

        Returns:
            A dictionary with a ``cancelled`` key holding the tickets that were
            cancelled. Partial success is possible.
        """
        return self._run(self._client.cancel_pending_orders(tickets))

    def closed_deals(self) -> List[Dict]:
        """Get a list of closed deals.

        Returns:
            A list of dictionaries with closed deal details.
        """
        return self._run(self._client.closed_deals())

    def get_closed_deal(self, trade_id: str) -> Optional[Trade]:
        """Get details of a specific closed deal.

        Args:
            trade_id: The trade identifier.

        Returns:
            A :class:`Trade` with the deal details, or None if not found.
        """
        return self._run(self._client.get_closed_deal(trade_id))

    def clear_closed_deals(self) -> None:
        """Clear the list of closed deals from local storage."""
        self._run(self._client.clear_closed_deals())

    def payouts(self) -> Dict[str, int]:
        """Get the current payout percentage for every asset.

        Returns:
            Mapping of asset symbol to payout percentage.
        """
        return self._run(self._client.payouts())

    def payout(self, asset: str) -> Optional[int]:
        """Get the current payout percentage for a single asset.

        Args:
            asset: Asset symbol (e.g. ``"EURUSD_otc"``).

        Returns:
            The payout percentage, or ``None`` if the asset is unknown.

        See Also:
            :meth:`payouts` for the payout of every asset at once.
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
        warnings.warn(
            "history() is deprecated; use candles() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self._run(self._client.history(asset, period))

    def get_ticks(self, asset: str, lookback_seconds: int) -> List[Tuple[int, float]]:
        """Get historical tick data for an asset.

        This method fetches raw tick data using the loadHistoryPeriod WebSocket message
        with pagination to retrieve the specified number of seconds of tick history.

        Args:
            asset: The trading asset name (e.g., "USDCHF_otc").
            lookback_seconds: Number of seconds of tick history to fetch.

        Returns:
            A list of (timestamp, price) tuples sorted by timestamp.

        Raises:
            ConnectionError: If the client is not connected.
            ValueError: If the asset is invalid or lookback_seconds is zero/negative.
            TimeoutError: If tick fetch times out.

        Example:
            ```python
            with PocketOption(ssid) as client:
                # Get last 5 minutes of tick data
                ticks = client.get_ticks("USDCHF_otc", 300)
                for ts, price in ticks[:5]:
                    print(f"{ts}: {price}")
            ```

        Note:
            - Uses loadHistoryPeriod pagination internally (period=1 for tick data).
            - Returns raw ticks, not aggregated candles.
        """
        warnings.warn(
            "get_ticks() is deprecated; use ticks() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        if not isinstance(lookback_seconds, int) or lookback_seconds <= 0:
            raise ValueError("lookback_seconds must be a positive integer")

        return self._run(self._client.get_ticks(asset, lookback_seconds))

    def compile_candles(self, asset: str, custom_period: int, lookback_period: int) -> List[Dict]:
        """Compile candles from raw data with a custom aggregation period.

        Args:
            asset: The trading asset name.
            custom_period: The target candle period in seconds.
            lookback_period: How far back to look for data in seconds.

        Returns:
            A list of compiled candle dictionaries.
        """
        warnings.warn(
            "compile_candles() is deprecated; use candles() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self._run(self._client.compile_candles(asset, custom_period, lookback_period))

    def send_raw(self, message: str) -> None:
        """Send a raw Engine.io/Socket.io message directly over the connection.

        Args:
            message: The raw protocol frame to send verbatim.
        """
        self._run(self._client.send_raw(message))

    def subscribe_raw(self) -> SyncRawSubscription:
        """Subscribe to all incoming WebSocket messages verbatim.

        Returns:
            An iterator of raw message strings.
        """
        return SyncRawSubscription(self._run(self._client.subscribe_raw()))

    def subscribe_ticks(
        self,
        asset: str,
        *,
        chunk_size: Optional[int] = None,
        interval: Optional[timedelta] = None,
        aligned: bool = False,
    ) -> SyncSubscription:
        """Subscribe to a live tick stream for an asset.

        Single entry point for live price subscriptions. See
        :meth:`PocketOptionAsync.subscribe_ticks` for mode selection: no
        keywords yields raw ticks, ``chunk_size`` aggregates ticks, and
        ``interval`` (with optional ``aligned``) emits on a timer.

        Args:
            asset: Trading asset (e.g. ``"EURUSD_otc"``).
            chunk_size: Number of ticks to aggregate per update.
            interval: Time interval between updates.
            aligned: When True, align interval emissions to clock boundaries.

        Returns:
            A SyncSubscription for iterating over price updates.
        """
        if chunk_size is not None:
            return self._subscribe_stream("subscribe_symbol_chunked", asset, chunk_size)
        if interval is not None:
            if aligned:
                return self._subscribe_stream("subscribe_symbol_time_aligned", asset, interval)
            return self._subscribe_stream("subscribe_symbol_timed", asset, interval)
        return self._subscribe_stream("subscribe_symbol", asset)

    def _subscribe_stream(self, method: str, *args) -> SyncSubscription:
        """Await a Rust subscription method on the event-loop thread.

        The Rust subscription bindings need a running loop, so they are
        dispatched as a coroutine instead of being called on the caller thread.
        """
        rust_method = getattr(self._client.client, method)

        async def subscribe():
            return await rust_method(*args)

        return SyncSubscription(self._run(subscribe()))

    def subscribe_symbol(self, asset: str) -> SyncSubscription:
        """Subscribe to a live raw tick stream for an asset.

        Alias for :meth:`subscribe_ticks` with no aggregation options.

        Args:
            asset: Trading asset (e.g. ``"EURUSD_otc"``).

        Returns:
            A SyncSubscription iterating over price updates.
        """
        return self.subscribe_ticks(asset)

    def subscribe_symbol_chunked(self, asset: str, chunk_size: int) -> SyncSubscription:
        """Deprecated: use ``subscribe_ticks(asset, chunk_size=...)``."""
        warnings.warn(
            "subscribe_symbol_chunked() is deprecated; use subscribe_ticks(asset, chunk_size=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.subscribe_ticks(asset, chunk_size=chunk_size)

    def subscribe_symbol_timed(self, asset: str, time: timedelta) -> SyncSubscription:
        """Deprecated: use ``subscribe_ticks(asset, interval=...)``."""
        warnings.warn(
            "subscribe_symbol_timed() is deprecated; use subscribe_ticks(asset, interval=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.subscribe_ticks(asset, interval=time)

    def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> SyncSubscription:
        """Deprecated: use ``subscribe_ticks(asset, interval=..., aligned=True)``."""
        warnings.warn(
            "subscribe_symbol_time_aligned() is deprecated; use subscribe_ticks(asset, interval=..., aligned=True) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.subscribe_ticks(asset, interval=time, aligned=True)

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

    def is_ssid_valid(self) -> bool:
        """Check whether the SSID passed format validation at construction time.

        Returns:
            True if the SSID was valid, False otherwise.
        """
        return self._client.is_ssid_valid()

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
        self.close()

    def raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandlerSync":
        """Create a synchronous raw WebSocket message handler.

        Args:
            validator: A Validator instance for message validation.
            keep_alive: Optional keep-alive message string.

        Returns:
            A RawHandlerSync instance wrapping the async raw handler.
        """
        async_handler = self._run(self._client.raw_handler(validator, keep_alive))
        return RawHandlerSync(async_handler, self.loop)

    def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandlerSync":
        """Deprecated: use ``raw_handler()`` instead."""
        warnings.warn(
            "create_raw_handler() is deprecated; use raw_handler() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.raw_handler(validator, keep_alive)

    def send_raw_message(self, message: str) -> None:
        """Deprecated: use ``send_raw()`` instead."""
        warnings.warn(
            "send_raw_message() is deprecated; use send_raw() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        self.send_raw(message)

    def raw_request(
        self,
        message: str,
        validator: Validator,
        timeout: Optional[timedelta] = None,
        retry: bool = False,
    ) -> str:
        """Send a raw message and wait for the first matching response.

        Args:
            message: Raw WebSocket message to send, formatted as JSON or
                Socket.IO protocol.
            validator: Validator identifying the expected response.
            timeout: Maximum time to wait. ``None`` uses the client default.
            retry: When True, retry on timeout or failure; requires ``timeout``.

        Returns:
            The first response matching the validator, as a raw string.
        """
        return self._run(self._client.raw_request(message, validator, timeout=timeout, retry=retry))

    def create_raw_order(self, message: str, validator: Validator) -> str:
        """Deprecated: use ``raw_request()`` instead."""
        warnings.warn(
            "create_raw_order() is deprecated; use raw_request() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.raw_request(message, validator)

    def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Deprecated: use ``raw_request(..., timeout=...)`` instead."""
        warnings.warn(
            "create_raw_order_with_timeout() is deprecated; use raw_request(..., timeout=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.raw_request(message, validator, timeout=timeout)

    def create_raw_order_with_timeout_and_retry(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Deprecated: use ``raw_request(..., timeout=..., retry=True)`` instead."""
        warnings.warn(
            "create_raw_order_with_timeout_and_retry() is deprecated; "
            "use raw_request(..., timeout=..., retry=True) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.raw_request(message, validator, timeout=timeout, retry=True)

    def raw_stream(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        """Send a raw message and iterate over every matching response.

        Args:
            message: Raw WebSocket message to send, formatted as JSON or
                Socket.IO protocol.
            validator: Validator identifying the responses to yield.
            timeout: Optional timeout for the iterator session.

        Returns:
            A SyncRawSubscription iterating matching messages as raw strings.
        """
        async_iterator = self._run(self._client.raw_stream(message, validator, timeout))
        return SyncRawSubscription(async_iterator)

    def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        """Deprecated: use ``raw_stream()`` instead."""
        warnings.warn(
            "create_raw_iterator() is deprecated; use raw_stream() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return self.raw_stream(message, validator, timeout)

    def active_assets(self) -> List[Dict]:
        """Get the list of currently active trading assets.

        Returns:
            A list of dictionaries with active asset details.
        """
        return self._run(self._client.active_assets())
