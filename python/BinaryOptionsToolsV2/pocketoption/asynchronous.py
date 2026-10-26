import asyncio
import json
import re
import sys
import time
import warnings
from collections import deque
from datetime import timedelta
from typing import TYPE_CHECKING, Awaitable, Callable, Dict, List, Optional, Tuple, Union, AsyncGenerator

from ..config import Config
from ..validator import Validator

from .models import Trade, TradeResult

# `changeSymbol` history can take tens of seconds on a fresh connection; the
# previous 3s cut discarded it, which left `get_candles_live` without backfill.
HISTORY_FETCH_TIMEOUT = 20.0
# Compiling candles from tick pages takes ~1s per hour of history.
TICK_HISTORY_FETCH_TIMEOUT = 10.0

if TYPE_CHECKING:
    from ..BinaryOptionsToolsV2 import Logger, RawPocketOption

if sys.version_info < (3, 10):

    async def anext(iterator):
        """Polyfill for anext for Python < 3.10"""
        return await iterator.__anext__()


class AsyncSubscription:
    def __init__(self, subscription):
        """Asynchronous Iterator over json objects"""
        self.subscription = subscription

    def __aiter__(self):
        return self

    async def __anext__(self):
        return json.loads(await anext(self.subscription))


class AsyncRawSubscription:
    def __init__(self, subscription):
        """Asynchronous Iterator over raw message strings"""
        self.subscription = subscription

    def __aiter__(self):
        return self

    async def __anext__(self):
        return await anext(self.subscription)


class RawHandler:
    """
    Handler for advanced raw WebSocket message operations.

    Provides low-level access to send messages and receive filtered responses
    based on a validator. Each handler maintains its own message stream.
    """

    def __init__(self, rust_handler):
        """
        Initialize RawHandler with a Rust handler instance.

        Args:
            rust_handler: The underlying RawHandlerRust instance from PyO3
        """
        self._handler = rust_handler

    async def send_text(self, message: str) -> None:
        """
        Send a text message through this handler.

        Args:
            message: Text message to send

        Example:
            ```python
            await handler.send_text('42["ping"]')
            ```
        """
        await self._handler.send_text(message)

    async def send_binary(self, data: bytes) -> None:
        """
        Send a binary message through this handler.

        Args:
            data: Binary data to send

        Example:
            ```python
            await handler.send_binary(b'\\x00\\x01\\x02')
            ```
        """
        await self._handler.send_binary(data)

    async def send_and_wait(self, message: str) -> str:
        """
        Send a message and wait for the next matching response.

        Args:
            message: Message to send

        Returns:
            str: The first response that matches this handler's validator

        Example:
            ```python
            response = await handler.send_and_wait('42["getBalance"]')
            data = json.loads(response)
            ```
        """
        return await self._handler.send_and_wait(message)

    async def wait_next(self) -> str:
        """
        Wait for the next message that matches this handler's validator.

        Returns:
            str: The next matching message

        Example:
            ```python
            message = await handler.wait_next()
            print(f"Received: {message}")
            ```
        """
        return await self._handler.wait_next()

    async def subscribe(self):
        """
        Subscribe to messages matching this handler's validator.

        Returns:
            AsyncIterator[str]: Stream of matching messages

        Example:
            ```python
            stream = await handler.subscribe()
            async for message in stream:
                data = json.loads(message)
                print(f"Update: {data}")
            ```
        """
        return self._handler.subscribe()

    def id(self) -> str:
        """
        Get the unique ID of this handler.

        Returns:
            str: Handler UUID
        """
        return self._handler.id()

    async def close(self) -> None:
        """
        Close this handler and clean up resources.
        Note: The handler is automatically cleaned up when it goes out of scope.
        This method is a no-op; resource cleanup is handled by the Rust Drop implementation.
        """
        self._handler = None  # Release reference to allow Rust Drop


def _is_guest_auth_payload(auth_data: dict) -> bool:
    """Return True when `auth_data` is a try-demo (guest) auth payload.

    Guest sessions authenticate with a short-lived `token` and no account
    `session`. Mirrors the Rust `try_demo_payload` rule in `pocketoption/ssid.rs`.
    """
    return "token" in auth_data and "session" not in auth_data and "sessionToken" not in auth_data


def sanitize_and_validate_ssid(ssid: str, logger: "Logger") -> str:
    """Sanitize SSID format and validate session payload semantics.

    Performs three layers of validation:
    1. Format normalization (fix shell-stripped quotes)
    2. JSON structure validation (parseable payload)
    3. Semantic validation (required fields, session format)

    Args:
        ssid: Raw SSID string from user input
        logger: Logger instance for warnings

    Returns:
        Sanitized SSID string ready for the Rust backend

    Raises:
        ValueError: If the SSID payload is missing required fields
    """
    ssid = re.sub(r"""42\[['"]?auth['"]?\s*,""", '42["auth",', ssid, count=1)

    if not ssid.startswith("42["):
        logger.warn(f"SSID does not start with '42[': {ssid[:20]}...")
        return ssid

    try:
        payload = json.loads(ssid[2:])
    except json.JSONDecodeError:
        logger.warn("SSID payload is not valid JSON after sanitization")
        return ssid

    if not isinstance(payload, list) or len(payload) < 2:
        logger.warn("SSID payload is not a valid Socket.IO auth array")
        return ssid

    auth_data = payload[1] if len(payload) > 1 else {}

    if not isinstance(auth_data, dict):
        logger.warn("SSID auth data is not a dictionary")
        return ssid

    # Guest ("try demo") payloads authenticate with a short-lived `token` and no
    # account session; they have no `session`/`uid` to validate.
    if _is_guest_auth_payload(auth_data):
        logger.debug("SSID is a guest (try demo) payload; skipping session validation")
        return ssid

    warnings_list = []

    required_fields = ["session", "uid"]
    for field in required_fields:
        if field not in auth_data:
            warnings_list.append(f"missing required field '{field}'")

    session = auth_data.get("session", "")
    if session and not re.match(r"^[a-zA-Z0-9_\-]{10,}$", str(session)):
        warnings_list.append(f"session token has unexpected format (length={len(str(session))})")

    uid = auth_data.get("uid")
    if uid is not None:
        try:
            uid_int = int(uid)
            if uid_int <= 0:
                warnings_list.append(f"uid should be a positive integer, got {uid_int}")
        except (ValueError, TypeError):
            warnings_list.append(f"uid is not a valid integer: {uid!r}")

    platform = auth_data.get("platform")
    if platform is not None and platform not in (1, 2):
        warnings_list.append(f"unexpected platform value: {platform}")

    is_demo = auth_data.get("isDemo")
    if is_demo is not None and is_demo not in (0, 1):
        warnings_list.append(f"isDemo should be 0 or 1, got {is_demo}")

    for w in warnings_list:
        logger.warn(f"SSID validation: {w}")

    critical = [w for w in warnings_list if "missing required field" in w]
    if critical:
        raise ValueError(
            "Invalid SSID: " + "; ".join(critical) + ". "
            "The SSID payload must contain 'session' and 'uid' fields. "
            "Ensure your SSID follows the format: 42['auth',{{'session':'...','uid':123,...}}]"
        )

    return ssid


# This file contains all the async code for the PocketOption Module
class PocketOptionAsync:
    def __init__(self, ssid: str, url: Optional[str] = None, config: Optional[Union[Config, dict, str]] = None, **_):
        """
        Initializes a new PocketOptionAsync instance.

        This class provides an asynchronous interface for interacting with the Pocket Option trading platform.
        It supports custom WebSocket URLs and configuration options for fine-tuning the connection behavior.

        Args:
            ssid (str): Session ID for authentication with Pocket Option platform
            url (str | None, optional): Custom WebSocket server URL. Defaults to None, using platform's default URL.
            config (Config | dict | str, optional): Configuration options. Can be provided as:
                - Config object: Direct instance of Config class
                - dict: Dictionary of configuration parameters
                - str: JSON string containing configuration parameters
                Configuration parameters include:
                    - max_allowed_loops (int): Maximum number of event loop iterations
                    - sleep_interval (int): Sleep time between operations in milliseconds
                    - reconnect_time (int): Time to wait before reconnection attempts in seconds
                    - connection_initialization_timeout_secs (int): Connection initialization timeout
                    - timeout_secs (int): General operation timeout
                    - urls (List[str]): List of fallback WebSocket URLs
            **_: Additional keyword arguments (ignored)

        Examples:
            Basic usage:
            ```python
            client = PocketOptionAsync("your-session-id")
            ```

            With custom WebSocket URL:
            ```python
            client = PocketOptionAsync("your-session-id", url="wss://custom-server.com/ws")
            ```


            Warning: This class is designed for asynchronous operations and should be used within an async context.
        Note:
            - The configuration becomes locked once initialized and cannot be modified afterwards
            - Custom URLs provided in the `url` parameter take precedence over URLs in the configuration
            - Invalid configuration values will raise appropriate exceptions
        """
        try:
            from ..BinaryOptionsToolsV2 import RawPocketOption
        except ImportError:
            from BinaryOptionsToolsV2 import RawPocketOption

        from ..tracing import Logger, LogBuilder

        self.logger = Logger()
        self._ssid_valid = True
        self._guest_session = False

        if ssid is not None:
            ssid = sanitize_and_validate_ssid(ssid, self.logger)
            if not ssid.startswith("42["):
                self._ssid_valid = False
            else:
                try:
                    payload = json.loads(ssid[2:])
                    if not isinstance(payload, list) or len(payload) < 2:
                        self._ssid_valid = False
                    elif isinstance(payload[1], dict):
                        self._guest_session = _is_guest_auth_payload(payload[1])
                except json.JSONDecodeError:
                    self._ssid_valid = False
        else:
            self.logger.warn("SSID is None, connection will likely fail")
            self._ssid_valid = False

        if config is not None:
            if isinstance(config, dict):
                self.config = Config.from_dict(config)
            elif isinstance(config, str):
                self.config = Config.from_json(config)
            elif isinstance(config, Config):
                self.config = config
            else:
                raise ValueError("Config type mismatch")
            if url is not None:
                self.config.urls.insert(0, url)
        else:
            self.config = Config()
            if url is not None:
                self.config.urls.insert(0, url)

        if self.config.terminal_logging:
            try:
                lb = LogBuilder()
                lb.terminal(level=self.config.log_level)
                lb.build()
            except Exception:
                pass

        self.client: "RawPocketOption" = RawPocketOption.new_with_config(ssid, self.config.pyconfig)

    @classmethod
    async def try_demo(cls, token: Optional[str] = None) -> "PocketOptionAsync":
        """Create a client on PocketOption's free "try demo" (guest) session.

        Guest sessions authenticate with a short-lived token instead of an account
        session, so no credentials are required.

        Args:
            token: An existing guest token. When omitted, one is generated locally,
                exactly like PocketOption's own "Try demo" client.

        Returns:
            A connected client with assets and balance already loaded.
        """
        try:
            from ..BinaryOptionsToolsV2 import RawPocketOption
        except ImportError:
            from BinaryOptionsToolsV2 import RawPocketOption

        from ..tracing import Logger

        self = cls.__new__(cls)
        self.logger = Logger()
        self._ssid_valid = True
        self._guest_session = True
        self.config = Config()
        self.client = await RawPocketOption.create_try_demo(token)
        return self

    async def __aenter__(self):
        """
        Context manager entry. Waits for assets to be loaded.
        """
        await self.wait_for_assets()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """
        Context manager exit. Shuts down the client and its runner.
        """
        await self.shutdown()

    async def _place_trade(self, method, asset: str, amount: float, time: int, check_win: bool) -> TradeResult:
        """Place a trade, returning ``(trade_id, trade)``; wait for the result when ``check_win``."""
        trade_id, trade = await method(asset, amount, time)
        if check_win:
            return trade_id, await self.check_win(trade_id, timeout_seconds=time + 30)
        return trade_id, Trade(json.loads(trade))

    async def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> TradeResult:
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
        return await self._place_trade(self.client.buy, asset, amount, time, check_win)

    async def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> TradeResult:
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
        return await self._place_trade(self.client.sell, asset, amount, time, check_win)

    async def check_win(self, id: str, timeout_seconds: Optional[int] = None) -> Trade:
        """Wait for and return the settled result of a trade.

        Args:
            id: Trade identifier (as returned by :meth:`buy`/:meth:`sell`).
            timeout_seconds: Maximum seconds to wait. ``None`` uses the configured
                default (300s); ``0`` waits indefinitely.

        Returns:
            The settled :class:`Trade`, with ``result`` set to
            ``"win"``, ``"loss"`` or ``"draw"``.

        Raises:
            ValueError: If the trade result cannot be read.
            TimeoutError: If the result does not arrive in time.
        """

        # Set a reasonable timeout to prevent hanging
        # Default to 300 seconds to accommodate longer trade durations (e.g., 300s timeframes)
        if timeout_seconds is None:
            timeout_seconds = getattr(self.config, "check_win_timeout_secs", 300)

        # If timeout_seconds is 0, we wait indefinitely
        actual_timeout = timeout_seconds if timeout_seconds > 0 else None

        try:
            # Use asyncio.wait_for as additional protection against hanging
            trade = await asyncio.wait_for(self._get_trade_result(id), timeout=actual_timeout)
            return trade
        except asyncio.TimeoutError:
            raise TimeoutError(f"Timeout waiting for trade result for ID: {id}")

    async def get_deal_end_time(self, trade_id: str) -> Optional[int]:
        """Get the expected close time of a deal.

        Args:
            trade_id: Trade identifier returned by :meth:`buy`/:meth:`sell`.

        Returns:
            The close time as a Unix timestamp, or ``None`` if the deal is unknown.
        """
        return await self.client.get_deal_end_time(trade_id)

    async def _get_trade_result(self, id: str) -> Trade:
        """Fetch a trade result and classify it as win/loss/draw."""
        try:
            trade = await self.client.check_win(id)
            trade = Trade(json.loads(trade))
            win = float(trade["profit"])
        except (json.JSONDecodeError, KeyError, ValueError, TypeError) as e:
            raise ValueError(f"Invalid trade result response for ID {id}: {e}") from e
        except Exception as e:
            raise RuntimeError(f"Error getting trade result for ID {id}: {e}") from e

        if win > 0:
            trade["result"] = "win"
        elif win == 0:
            trade["result"] = "draw"
        else:
            trade["result"] = "loss"
        return trade

    async def candles(
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
            async with PocketOptionAsync(ssid) as client:
                candles = await client.candles("EURUSD_otc", 60, count=200)
            ```
        """
        if end_time is not None:
            # RawPocketOption takes (asset, period, offset, time).
            return json.loads(await self.client.get_candles_advanced(asset, period, offset=count, time=end_time))
        lookback_seconds = max(period, count * period)
        hours = max(0.1, lookback_seconds / 3600.0)
        gen = self.get_candles_live(asset, period, hours=hours, max_rows=count)
        try:
            closed, _forming = await anext(gen)
        finally:
            await gen.aclose()
        return closed

    async def get_candles(self, asset: str, period: int, offset: int) -> List[Dict]:
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            period (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            offset (int): Number of periods to look back (e.g., 200 for 200 candles)

        Returns:
            List[Dict]: List of candles, each containing:
                - time: Candle timestamp
                - open: Opening price
                - high: Highest price
                - low: Lowest price
                - close: Closing price

        Note:
            - Available timeframes: 1, 5, 15, 30, 60, 300 seconds
            - Maximum period depends on the timeframe
            - WARNING: This function only fetches closed historical candles and is intended
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
        # Convert to hours for get_candles_live: offset * period seconds = total lookback seconds
        lookback_seconds = offset * period
        hours = max(0.1, lookback_seconds / 3600.0)
        gen = self.get_candles_live(asset, period, hours=hours, max_rows=offset)
        try:
            closed, _forming = await anext(gen)
        finally:
            # Close the feed even when the first `anext` raises, otherwise the
            # subscription task behind it is left running.
            await gen.aclose()
        return closed

    async def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            period (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            offset (int): Number of periods to look back from reference time
            time (int): Reference timestamp to fetch candles from

        Returns:
            List[Dict]: List of candles, each containing:
                - time: Candle timestamp
                - open: Opening price
                - high: Highest price
                - low: Lowest price
                - close: Closing price

        Note:
            - Available timeframes: 1, 5, 15, 30, 60, 300 seconds
            - Maximum period depends on the timeframe
            - WARNING: This function only fetches closed historical candles and is intended
              for training models, backtesting, or historical analysis. It is NOT designed
              for real-time/live trading as it does not include the current forming candle
              and can introduce gaps if called sequentially during live trading.
              For live gap-free candle feeds, use `get_candles_live()` instead.
        """
        warnings.warn(
            "get_candles_advanced() is deprecated; use candles(..., end_time=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        # RawPocketOption takes (asset, period, offset, time).
        candles = await self.client.get_candles_advanced(asset, period, offset=offset, time=time)
        return json.loads(candles)

    async def get_candles_live(
        self,
        asset: str,
        period: int,
        hours: float = 2.0,
        max_rows: int = 100,
    ) -> AsyncGenerator[Tuple[List[Dict], Optional[Dict]], None]:
        """Fetches historical backfill and streams gap-free live candles.

        This method subscribes to raw ticks first and buffers them, then fetches
        historical candles (compiled from raw tick pages, falling back to the chart
        history when those do not cover the requested window), merges them, replays
        the buffered ticks, and yields updated candles (both closed historical
        candles and the current forming candle) in real-time.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            period (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            hours (float): Hours of history to backfill. Defaults to 2.0.
            max_rows (int): Maximum number of closed candles to retain in history. Defaults to 100.

        Yields:
            Tuple[List[Dict], Optional[Dict]]: A tuple containing:
                - List[Dict]: List of closed candles (up to max_rows), each containing
                  'time', 'open', 'high', 'low', 'close'.
                - Optional[Dict]: The currently forming candle, containing 'time', 'open',
                  'high', 'low', 'close', or None if not yet started.
        """
        platform_time_offset = 7200

        def bucket_start(timestamp: int, p: int) -> int:
            return (timestamp // p) * p

        def extract_time(candle: Dict) -> int:
            val = int(float(candle.get("timestamp", candle.get("time", 0))))
            if val > 10_000_000_000:
                val //= 1000
            return val - platform_time_offset

        def merge_candles(*groups: List[Dict]) -> List[Dict]:
            res: Dict[int, Dict] = {}
            for group in groups:
                for candle in group or []:
                    res[extract_time(candle)] = candle
            return [res[ts] for ts in sorted(res)]

        # Initialize feed
        feed_candles: deque = deque(maxlen=max_rows)
        forming: Optional[Dict] = None

        def seed_history(history: List[Dict]) -> None:
            cutoff = bucket_start(int(time.time()), period)
            ordered = sorted(
                (c for c in history if extract_time(c) < cutoff),
                key=extract_time,
            )
            for c in ordered[-max_rows:]:
                feed_candles.append(
                    {
                        "time": extract_time(c),
                        "open": float(c["open"]),
                        "high": float(c["high"]),
                        "low": float(c["low"]),
                        "close": float(c["close"]),
                    }
                )

        def ingest_tick(timestamp: int, price: float) -> None:
            nonlocal forming
            if forming is None:
                start = bucket_start(timestamp, period)
                forming = {
                    "time": start,
                    "open": price,
                    "high": price,
                    "low": price,
                    "close": price,
                }
                return

            start = bucket_start(timestamp, period)
            if start == forming["time"]:
                forming["high"] = max(forming["high"], price)
                forming["low"] = min(forming["low"], price)
                forming["close"] = price
            elif start > forming["time"]:
                feed_candles.append(dict(forming))
                forming = {
                    "time": start,
                    "open": price,
                    "high": price,
                    "low": price,
                    "close": price,
                }

        # 1. Subscribe to ticks FIRST
        tick_buffer: List[Tuple[int, float]] = []
        buffering = True
        stream = await self.subscribe_symbol(asset)

        queue = asyncio.Queue()

        async def tick_reader():
            nonlocal buffering
            try:
                async for tick in stream:
                    ts = extract_time(tick)
                    price = float(tick.get("close", tick.get("price", 0.0)))
                    if buffering:
                        tick_buffer.append((ts, price))
                    else:
                        ingest_tick(ts, price)
                        await queue.put((list(feed_candles), dict(forming) if forming else None))
            except asyncio.CancelledError:
                pass
            finally:
                await queue.put(None)

        reader_task = asyncio.create_task(tick_reader())

        try:
            # 2. Fetch history while buffering ticks
            offset_seconds = int(hours * 3600)

            async def fetch_history(
                request: Callable[[], Awaitable[Optional[str]]],
                timeout: float,
            ) -> List[Dict]:
                """Run one history source, treating a failure as empty history."""
                try:
                    payload = await asyncio.wait_for(request(), timeout=timeout)
                    candles = json.loads(payload) if payload else []
                except Exception:
                    return []
                return candles if isinstance(candles, list) else []

            # Compiling from raw tick pages is the fastest and most dependable
            # source, so ask it first. `get_candles_advanced` is not used at all:
            # its `loadHistoryPeriodFast` request was never answered, and every call
            # left a request hanging that eventually dropped the connection (#141).
            compiled_candles = await fetch_history(
                lambda: self.client.compile_candles(asset, period, offset_seconds),
                TICK_HISTORY_FETCH_TIMEOUT,
            )

            # The chart history covers the same window with more candles but takes
            # much longer, so it only runs when the tick pages did not cover what
            # was asked for. Requesting both for every asset at once is what flooded
            # the socket and dropped subscriptions (#141).
            wanted = min(max_rows, max(1, offset_seconds // period))
            recent_candles = (
                []
                if len(compiled_candles) >= wanted
                else await fetch_history(
                    lambda: self.client.history(asset, period),
                    HISTORY_FETCH_TIMEOUT,
                )
            )

            history = merge_candles(compiled_candles, recent_candles)
            seed_history(history)

            # 3. Replay backlog
            cutoff = 0
            if feed_candles:
                cutoff = feed_candles[-1]["time"] + period
            for ts, price in sorted(tick_buffer):
                if ts < cutoff:
                    continue
                ingest_tick(ts, price)

            buffering = False
            # Yield initial seed state
            yield list(feed_candles), dict(forming) if forming else None

            # 4. Stream loop
            while True:
                item = await queue.get()
                if item is None:
                    break
                yield item

        finally:
            reader_task.cancel()
            try:
                await reader_task
            except asyncio.CancelledError:
                pass
            # Do NOT call self.unsubscribe(asset) here - it removes ALL subscriptions for the asset.
            # The temporary subscription created by subscribe_symbol() will be cleaned up
            # when the stream variable goes out of scope (its Drop sends Unsubscribe for that specific ID).

    async def stream_candles(
        self,
        asset: str,
        period: int,
        *,
        history: float = 2.0,
        max_rows: int = 100,
    ) -> AsyncGenerator[Tuple[List[Dict], Optional[Dict]], None]:
        """Stream gap-free live candles for an asset.

        Backfills `history` hours of closed candles, then yields updated
        (closed_candles, forming_candle) tuples as ticks arrive.

        Args:
            asset: Trading asset (e.g. "EURUSD_otc").
            period: Candle timeframe in seconds.
            history: Hours of history to backfill. Defaults to 2.0.
            max_rows: Maximum number of closed candles to retain.

        Yields:
            (List[Dict], Optional[Dict]): closed candles and the currently
            forming candle.

        Example:
            ```python
            async with PocketOptionAsync(ssid) as client:
                async for candles, forming in client.stream_candles("EURUSD_otc", 60):
                    ...
            ```
        """
        async for item in self.get_candles_live(asset, period, hours=history, max_rows=max_rows):
            yield item

    async def ticks(self, asset: str, seconds: int) -> List[Tuple[int, float]]:
        """Fetch historical raw ticks for an asset.

        Args:
            asset: Trading asset (e.g. "USDCHF_otc").
            seconds: Seconds of tick history to fetch.

        Returns:
            List of (timestamp, price) tuples sorted by timestamp.

        Example:
            ```python
            async with PocketOptionAsync(ssid) as client:
                ticks = await client.ticks("USDCHF_otc", 300)
            ```
        """
        if not isinstance(seconds, int) or seconds <= 0:
            raise ValueError("seconds must be a positive integer")
        raw = await self.client.get_ticks(asset, seconds)
        return [tuple(t) for t in json.loads(raw)]

    async def balance(self) -> float:
        """
        Retrieves current account balance.

        Returns:
            float: Account balance in account currency

        Note:
            Updates in real-time as trades are completed. Guest ("try demo")
            sessions never receive a balance frame, so they return -1.0 straight
            away when the balance is unknown instead of retrying.
        """
        for _ in range(100):
            bal = await self.client.balance()
            if bal >= 0.0:
                return bal
            if self._guest_session:
                # Guest (try demo) sessions never receive a balance frame, so
                # polling cannot produce a different answer.
                return bal
            await asyncio.sleep(0.1)
        return await self.client.balance()

    async def opened_deals(self) -> List[Dict]:
        """Retrieves a list of all currently open (active) deals.

        This method returns the deals themselves that are currently active/open on
        the account, including both pending and executed trades that have not yet
        closed.

        Returns:
            List[Dict]: One dictionary per open deal, ordered by deal id. Each
            entry carries the deal fields, including `id` (UUID string), `asset`,
            `amount`, `direction` and `open_price`.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the response format is invalid

        Examples:
            Basic usage:
            ```python
            async with PocketOptionAsync(ssid) as client:
                open_deals = await client.opened_deals()
                for deal in open_deals:
                    print(f"Deal {deal['id']}: {deal['asset']} {deal['direction']}")
            ```

            Total exposure:
            ```python
            async def monitor_open_deals(client):
                deals = await client.opened_deals()
                total_value = sum(d['amount'] for d in deals)
                print(f"Open deals: {len(deals)}, Total exposure: {total_value}")
            ```
        """
        return json.loads(await self.client.opened_deals())

    async def get_opened_deal(self, trade_id: str) -> Optional[Trade]:
        """
        Retrieves details of a specific opened deal by its ID.

        Args:
            trade_id (str): The unique identifier of the deal to retrieve

        Returns:
            Optional[Trade]: A :class:`Trade` with the deal details if found,
            otherwise None. Deal details include:
                - id: Unique deal identifier
                - asset: Trading asset symbol
                - amount: Trade amount
                - direction: "buy" or "sell"
                - entry_price: Entry price of the trade
                - expiry: Expiration timestamp
                - timestamp: Deal creation timestamp

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the response format is invalid

        Examples:
            Fetch specific deal details:
            ```python
            async with PocketOptionAsync(ssid) as client:
                deal_id = "123e4567-e89b-12d3-a456-426614174000"
                deal_details = await client.get_opened_deal(deal_id)
                if deal_details:
                    print(f"Deal {deal_details['id']}: {deal_details['asset']} {deal_details['direction']}")
                else:
                    print("Deal not found")
            ```
        """
        deal_json = await self.client.get_opened_deal(trade_id)
        if deal_json is None:
            return None
        return Trade(json.loads(deal_json))

    async def open_pending_order(
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
        order = await self.client.open_pending_order(
            open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
        )
        return json.loads(order)

    async def cancel_pending_order(self, ticket: str) -> Dict:
        """
        Cancels a pending order by its ticket identifier.

        Args:
            ticket (str): The unique ticket string identifying the pending order to cancel.

        Returns:
            Dict: Cancellation result containing:
                - ticket: The ticket of the cancelled order
                - status: "cancelled"

        Raises:
            ValueError: If the ticket is invalid
            TimeoutError: If the cancellation times out
            RuntimeError: If the order cannot be cancelled (e.g., already executed)

        Example:
            ```python
            # Cancel a pending order
            result = await client.cancel_pending_order("order-ticket-123")
            print(f"Cancelled: {result['ticket']}")
            ```
        """
        result = await self.client.cancel_pending_order(ticket)
        return json.loads(result)

    async def cancel_pending_orders(self, tickets: List[str]) -> Dict:
        """
        Cancels multiple pending orders in a single batch operation.

        Args:
            tickets (List[str]): A list of ticket strings identifying the pending orders to cancel.

        Returns:
            Dict: Batch cancellation result containing:
                - cancelled: List of tickets that were successfully cancelled
                - failed: List of tickets that failed to cancel (if any)

        Raises:
            ValueError: If any ticket is invalid
            TimeoutError: If the batch cancellation times out

        Note:
            Partial success is possible: some orders may be cancelled while others fail.

        Example:
            ```python
            # Cancel multiple pending orders
            tickets = ["order-1", "order-2", "order-3"]
            result = await client.cancel_pending_orders(tickets)
            print(f"Cancelled {len(result['cancelled'])} orders")
            ```
        """
        result = await self.client.cancel_pending_orders(tickets)
        return json.loads(result)

    async def closed_deals(self) -> List[Dict]:
        """Retrieves all closed/completed deals.

        This method returns the deals that have reached a final outcome (win,
        loss, or draw).

        Returns:
            List[Dict]: One dictionary per closed deal, ordered by deal id. Each
            entry carries the deal fields, including `id` (UUID string), `asset`,
            `amount`, `profit`, `result` and `close_price`.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the response format is invalid

        Examples:
            Basic usage:
            ```python
            async with PocketOptionAsync(ssid) as client:
                for deal in await client.closed_deals():
                    print(f"Deal {deal['id']}: {deal['result']} (profit: {deal['profit']})")
            ```

            Calculate total profit/loss:
            ```python
            async def calculate_pnl():
                async with PocketOptionAsync(ssid) as client:
                    closed = await client.closed_deals()
                    total_pnl = sum(d['profit'] for d in closed)
                    wins = sum(1 for d in closed if d['result'] == 'win')
                    print(f"Total P/L: {total_pnl}, Win rate: {wins}/{len(closed)}")
            ```
        """
        return json.loads(await self.client.closed_deals())

    async def get_closed_deal(self, trade_id: str) -> Optional[Trade]:
        """
        Retrieves details of a specific closed deal by its ID.

        Args:
            trade_id (str): The unique identifier of the closed deal to retrieve

        Returns:
            Optional[Trade]: The :class:`Trade` with the deal details if found,
            otherwise None, containing:
            - id: Unique deal identifier
            - asset: Trading asset symbol
            - amount: Trade amount
            - direction: "buy" or "sell"
            - entry_price: Entry price of the trade
            - close_price: Closing/expiry price
            - expiry: Expiration timestamp
            - result: Final outcome ("win", "loss", or "draw")
            - profit: Profit/loss amount (positive for win, negative for loss, 0 for draw)
            - timestamp: Deal creation and close timestamps

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the response format is invalid
        Examples:
            Fetch specific closed deal details:
            ```python
            async with PocketOptionAsync(ssid) as client:
                deal_id = "123e4567-e89b-12d3-a456-426614174000"
                deal_details = await client.get_closed_deal(deal_id)
                if deal_details:
                    print(f"Closed Deal {deal_details['id']}: {deal_details['result']} (profit: {deal_details['profit']})")
                else:
                    print("Closed deal not found")
            ```
        """
        deal_json = await self.client.get_closed_deal(trade_id)
        if deal_json is None:
            return None
        return Trade(json.loads(deal_json))

    async def clear_closed_deals(self) -> None:
        """Removes all closed deals from the client's memory.

        This method clears the internal cache/storage of closed deals. After calling
        this method, subsequent calls to `closed_deals()` will only return deals
        that have been closed after this operation. This is useful for managing
        memory when dealing with a large number of historical trades.

        Note:
            This operation is irreversible. Once cleared, the closed deal history
            cannot be recovered through the client. However, the data may still
            be available on the server.

        Raises:
            ConnectionError: If the client is not connected to the platform
            RuntimeError: If the clear operation fails on the server

        Examples:
            Clear old closed deals:
            ```python
            async with PocketOptionAsync(ssid) as client:
                # Check current closed deals count
                closed = await client.closed_deals()
                print(f"Before clear: {len(closed)} closed deals")

                # Clear the cache
                await client.clear_closed_deals()

                # Verify cleared
                closed_after = await client.closed_deals()
                print(f"After clear: {len(closed_after)} closed deals")
            ```

            Periodic cleanup:
            ```python
            async def periodic_cleanup():
                async with PocketOptionAsync(ssid) as client:
                    # Clear closed deals every hour
                    while True:
                        await asyncio.sleep(3600)
                        await client.clear_closed_deals()
                        print("Closed deals cache cleared")
            ```
        """
        await self.client.clear_closed_deals()

    async def payouts(self) -> Dict[str, int]:
        """Current payout percentage for every asset, keyed by symbol.

        Returns:
            Mapping of asset symbol to payout percentage, e.g.
            ``{"EURUSD_otc": 85, "GBPUSD_otc": 82}``.
        """
        return json.loads(await self.client.payout())

    async def payout(self, asset: str) -> Optional[int]:
        """Current payout percentage for a single asset.

        Args:
            asset: Asset symbol (e.g. ``"EURUSD_otc"``).

        Returns:
            The payout percentage, or ``None`` if the asset is unknown.

        See Also:
            :meth:`payouts` for the payout of every asset at once.
        """
        return (await self.payouts()).get(asset)

    async def active_assets(self) -> List[Dict]:
        """
        Retrieves a list of all active assets.

        Returns:
            List[Dict]: List of active assets, each containing:
                - id: Asset ID
                - symbol: Asset symbol (e.g., "EURUSD_otc")
                - name: Human-readable name
                - asset_type: Type of asset (stock, currency, commodity, cryptocurrency, index)
                - payout: Payout percentage
                - is_otc: Whether this is an OTC asset
                - is_active: Whether the asset is currently active for trading
                - allowed_candles: List of allowed timeframe durations in seconds

        Example:
            ```python
            async with PocketOptionAsync(ssid) as client:
                active = await client.active_assets()
                for asset in active:
                    print(f"{asset['symbol']}: {asset['name']} (payout: {asset['payout']}%)")
            ```
        """
        assets_json = await self.client.active_assets()
        assets = json.loads(assets_json)
        return list(assets.values()) if isinstance(assets, dict) else assets

    async def history(self, asset: str, period: int) -> List[Dict]:
        """Retrieves historical price data for an asset.

        This method fetches the latest available historical data for the specified asset,
        starting from the given period. The returned data format is identical to
        `get_candles()`, containing OHLC (Open, High, Low, Close) candle data.

        Args:
            asset (str): Trading asset symbol (e.g., "EURUSD_otc", "BTCUSD")
            period (int): Time period in seconds to fetch historical data from.
                For example, period=60 fetches data from the last minute.

        Returns:
            List[Dict]: A list of dictionaries, each representing a candlestick with:
                - time: Candle timestamp (Unix timestamp)
                - open: Opening price
                - high: Highest price during the period
                - low: Lowest price during the period
                - close: Closing price

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the asset is invalid or the period is not supported
            TimeoutError: If the data fetch times out

        Examples:
            Basic usage - fetch last minute of data:
            ```python
            async with PocketOptionAsync(ssid) as client:
                candles = await client.history("EURUSD_otc", 60)
                for candle in candles:
                    print(f"{candle['time']}: O={candle['open']}, C={candle['close']}")
            ```

            Calculate moving average:
            ```python
            async def calculate_ma(asset, period=300):
                async with PocketOptionAsync(ssid) as client:
                    candles = await client.history(asset, period)
                    if candles:
                        closes = [c['close'] for c in candles]
                        ma = sum(closes) / len(closes)
                        print(f"Simple Moving Average: {ma:.5f}")
            ```

        Note:
            This method is similar to `get_candles()` but uses a different API endpoint
            and may have different availability or latency characteristics. For advanced
            historical data with specific time ranges, consider using `get_candles_advanced()`.
        """
        warnings.warn(
            "history() is deprecated; use candles() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return json.loads(await self.client.history(asset, period))

    async def get_ticks(self, asset: str, lookback_seconds: int) -> List[Tuple[int, float]]:
        """Retrieves historical tick data for an asset.

        This method fetches raw tick data using the loadHistoryPeriod WebSocket message
        with pagination to retrieve the specified number of seconds of tick history.

        Args:
            asset (str): Trading symbol (e.g., "USDCHF_otc")
            lookback_seconds (int): Number of seconds of tick history to fetch

        Returns:
            List[Tuple[int, float]]: List of (timestamp, price) tuples sorted by timestamp

        Raises:
            ConnectionError: If the client is not connected
            ValueError: If the asset is invalid or lookback_seconds is zero/negative
            TimeoutError: If tick fetch times out

        Example:
            ```python
            async with PocketOptionAsync(ssid) as client:
                # Get last 5 minutes of tick data
                ticks = await client.get_ticks("USDCHF_otc", 300)
                for ts, price in ticks[:5]:
                    print(f"{ts}: {price}")
            ```

        Note:
            - Uses loadHistoryPeriod pagination internally (period=1 for tick data)
            - Returns raw ticks, not aggregated candles
        """
        warnings.warn(
            "get_ticks() is deprecated; use ticks() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        if not isinstance(lookback_seconds, int) or lookback_seconds <= 0:
            raise ValueError("lookback_seconds must be a positive integer")

        return [tuple(tick) for tick in json.loads(await self.client.get_ticks(asset, lookback_seconds))]

    async def compile_candles(self, asset: str, custom_period: int, lookback_period: int) -> List[Dict]:
        """Compiles custom candlesticks from raw tick history.

        This method fetches raw tick data over the specified lookback period and
        aggregates it into custom-sized candles. This enables non-standard timeframes
        like 20 seconds, 40 seconds, 90 seconds, etc.

        Args:
            asset (str): Trading asset symbol (e.g., "EURUSD_otc")
            custom_period (int): Desired candle duration in seconds (e.g., 20, 40, 90)
            lookback_period (int): Number of seconds of tick history to fetch.
                This determines the time range from which ticks are collected.

        Returns:
            List[Dict]: A list of dictionaries, each representing a compiled candlestick:
                - time: Candle timestamp (Unix timestamp, aligned to period boundaries)
                - open: Opening price
                - high: Highest price during the period
                - low: Lowest price during the period
                - close: Closing price

        Raises:
            ConnectionError: If the client is not connected
            ValueError: If the asset is invalid or periods are zero/negative
            TimeoutError: If tick fetch or compilation times out

        Example:
            ```python
            async with PocketOptionAsync(ssid) as client:
                # Get 20-second candles from last 5 minutes
                candles = await client.compile_candles("EURUSD_otc", 20, 300)
                for candle in candles:
                    print(f"{candle['time']}: O={candle['open']}, C={candle['close']}")
            ```

        Note:
            - This is a compute-intensive operation as it fetches and processes raw ticks.
            - For standard timeframes, use `candles()` or `get_candles()` for better efficiency.
        """
        if not isinstance(custom_period, int) or custom_period <= 0:
            raise ValueError("custom_period must be a positive integer")
        if not isinstance(lookback_period, int) or lookback_period <= 0:
            raise ValueError("lookback_period must be a positive integer")

        warnings.warn(
            "compile_candles() is deprecated; use candles() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return json.loads(await self.client.compile_candles(asset, custom_period, lookback_period))

    async def send_raw(self, message: str) -> None:
        """Send a raw Engine.io/Socket.io message directly over the connection.

        Args:
            message: The raw protocol frame to send verbatim.
        """
        await self.client.send_raw(message)

    async def subscribe_raw(self) -> AsyncRawSubscription:
        """Subscribe to all incoming WebSocket messages verbatim.

        Returns:
            An async iterator of raw message strings.
        """
        return AsyncRawSubscription(await self.client.subscribe_raw())

    async def subscribe_ticks(
        self,
        asset: str,
        *,
        chunk_size: Optional[int] = None,
        interval: Optional[timedelta] = None,
        aligned: bool = False,
    ) -> AsyncSubscription:
        """Subscribe to a live tick stream for an asset.

        This is the single entry point for live price subscriptions. The
        delivery shape is chosen by the keyword arguments:

        - no keywords: one update per raw tick
        - ``chunk_size=N``: aggregate ``N`` ticks into each update
        - ``interval=timedelta(...)``: emit on a fixed time interval
        - ``interval=...`` with ``aligned=True``: align emissions to clock
          boundaries

        Args:
            asset: Trading asset (e.g. ``"EURUSD_otc"``).
            chunk_size: Number of ticks to aggregate per update.
            interval: Time interval between updates.
            aligned: When True, align interval emissions to clock boundaries.

        Returns:
            AsyncSubscription yielding price updates.
        """
        if chunk_size is not None:
            return AsyncSubscription(await self.client.subscribe_symbol_chunked(asset, chunk_size))
        if interval is not None:
            if aligned:
                return AsyncSubscription(await self.client.subscribe_symbol_time_aligned(asset, interval))
            return AsyncSubscription(await self.client.subscribe_symbol_timed(asset, interval))
        return AsyncSubscription(await self.client.subscribe_symbol(asset))

    async def subscribe_symbol(self, asset: str) -> AsyncSubscription:
        """Subscribe to a live raw tick stream for an asset.

        Alias for :meth:`subscribe_ticks` with no aggregation options.

        Args:
            asset: Trading asset (e.g. ``"EURUSD_otc"``).

        Returns:
            An async iterator yielding JSON-parsed price updates.
        """
        return await self.subscribe_ticks(asset)

    async def subscribe_symbol_chunked(self, asset: str, chunk_size: int) -> AsyncSubscription:
        """Deprecated: use ``subscribe_ticks(asset, chunk_size=...)``."""
        warnings.warn(
            "subscribe_symbol_chunked() is deprecated; use subscribe_ticks(asset, chunk_size=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.subscribe_ticks(asset, chunk_size=chunk_size)

    async def subscribe_symbol_timed(self, asset: str, time: timedelta) -> AsyncSubscription:
        """Deprecated: use ``subscribe_ticks(asset, interval=...)``."""
        warnings.warn(
            "subscribe_symbol_timed() is deprecated; use subscribe_ticks(asset, interval=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.subscribe_ticks(asset, interval=time)

    async def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> AsyncSubscription:
        """Deprecated: use ``subscribe_ticks(asset, interval=..., aligned=True)``."""
        warnings.warn(
            "subscribe_symbol_time_aligned() is deprecated; use subscribe_ticks(asset, interval=..., aligned=True) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.subscribe_ticks(asset, interval=time, aligned=True)

    async def get_server_time(self) -> int:
        """Retrieves the current server time from Pocket Option.

        Returns the server's current Unix timestamp (seconds since epoch).
        This is useful for synchronizing local operations with server time,
        calculating time-sensitive parameters, or debugging time-related issues.

        Returns:
            int: Unix timestamp representing the current server time in seconds.

        Raises:
            ConnectionError: If the client is not connected to the platform
            TimeoutError: If the request times out

        Examples:
            Basic usage:
            ```python
            async with PocketOptionAsync(ssid) as client:
                server_time = await client.get_server_time()
                print(f"Server time: {datetime.fromtimestamp(server_time)}")
            ```

            Synchronize local time:
            ```python
            import time

            async def check_time_sync():
                async with PocketOptionAsync(ssid) as client:
                    server_time = await client.get_server_time()
                    local_time = int(time.time())
                    offset = server_time - local_time
                    print(f"Time offset with server: {offset} seconds")
            ```

            Calculate expiry time:
            ```python
            async def place_trade_with_expiry(asset: str, amount: float, duration: int):
                async with PocketOptionAsync(ssid) as client:
                    server_time = await client.get_server_time()
                    expiry = server_time + duration
                    # Use expiry for trade timing
            ```
        """
        return await self.client.get_server_time()

    async def wait_for_assets(self, timeout: float = 60.0) -> None:
        """
        Waits for the assets to be loaded from the server.

        Args:
            timeout (float): The maximum time to wait in seconds. Default is 60.0.

        Raises:
            TimeoutError: If the assets are not loaded within the timeout period.
        """
        await self.client.wait_for_assets(timeout)

    async def get_pending_deals(self) -> List[Dict]:
        """Retrieves a list of all pending orders.

        Returns:
            List[Dict]: List of pending orders, each containing:
                - ticket: Order ticket identifier
                - open_type: Type of pending order
                - amount: Order amount
                - symbol: Asset symbol
                - open_time: Order open time
                - open_price: Order open price
                - timeframe: Trade duration
                - min_payout: Minimum payout percentage
                - command: Trade direction
                - date_created: Order creation date
                - id: Order internal ID
        """
        return json.loads(await self.client.get_pending_deals())

    def is_demo(self) -> bool:
        """
        Checks if the current account is a demo account.

        Returns:
            bool: True if using a demo account, False if using a real account

        Examples:
            ```python
            # Basic account type check
            async with PocketOptionAsync(ssid) as client:
                is_demo = client.is_demo()
                print("Using", "demo" if is_demo else "real", "account")

            # Example with balance check
            async def check_account():
                is_demo = client.is_demo()
                balance = await client.balance()
                print(f"{'Demo' if is_demo else 'Real'} account balance: {balance}")

            # Example with trade validation
            async def safe_trade(asset: str, amount: float, duration: int):
                is_demo = client.is_demo()
                if not is_demo and amount > 100:
                    raise ValueError("Large trades should be tested in demo first")
                return await client.buy(asset, amount, duration)
            ```
        """
        return self.client.is_demo()

    def is_connected(self) -> bool:
        """
        Checks if the client is currently connected to the WebSocket server.

        Use this before performing operations to avoid "channel closed" errors
        when the connection has dropped.

        Returns:
            bool: True if connected, False otherwise
        """
        return self.client.is_connected()

    def is_ssid_valid(self) -> bool:
        """Returns whether the SSID passed basic format validation during init."""
        return self._ssid_valid

    async def disconnect(self) -> None:
        """
        Disconnects the client while keeping the configuration intact.
        The connection will automatically try to re-establish if max_allowed_loops > 0.
        To completely stop the client and its runner, use shutdown().

        Example:
            ```python
            client = PocketOptionAsync(ssid)
            # Use client...
            await client.disconnect()
            # The client will try to reconnect in the background...
            ```
        """
        await self.client.disconnect()

    async def connect(self) -> None:
        """
        Establishes a connection after a manual disconnect.
        Uses the same configuration and credentials.

        Example:
            ```python
            await client.disconnect()
            # Connection is closed
            await client.connect()
            # Connection is re-established
            ```
        """
        await self.client.connect()

    async def reconnect(self) -> None:
        """
        Disconnects and reconnects the client.

        Example:
            ```python
            await client.reconnect()
            ```
        """
        await self.client.reconnect()

    async def unsubscribe(self, asset: str) -> None:
        """
        Unsubscribes from an asset's stream by asset name.

        Args:
            asset (str): Asset name to unsubscribe from (e.g., "EURUSD_otc")

        Example:
            ```python
            # Subscribe to asset
            subscription = await client.subscribe_symbol("EURUSD_otc")
            # ... use subscription ...
            # Unsubscribe when done
            await client.unsubscribe("EURUSD_otc")
            ```
        """
        await self.client.unsubscribe(asset)

    async def shutdown(self) -> None:
        """
        Completely shuts down the client and its background runner.
        Once shut down, the client cannot be used anymore.
        """
        await self.client.shutdown()

    async def raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandler":
        """
        Creates a raw handler for advanced WebSocket message handling.

        Args:
            validator: Validator instance to filter incoming messages
            keep_alive: Optional message to send on reconnection

        Returns:
            RawHandler: Handler instance for sending/receiving messages

        Example:
            ```python
            from BinaryOptionsToolsV2.validator import Validator

            validator = Validator.starts_with('42["signals"')
            handler = await client.raw_handler(validator)

            # Send and wait for response
            response = await handler.send_and_wait('42["signals/subscribe"]')

            # Or subscribe to stream
            async for message in handler.subscribe():
                print(message)
            ```
        """
        rust_handler = await self.client.create_raw_handler(validator.raw_validator, keep_alive)
        return RawHandler(rust_handler)

    async def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandler":
        """Deprecated: use ``raw_handler()`` instead."""
        warnings.warn(
            "create_raw_handler() is deprecated; use raw_handler() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.raw_handler(validator, keep_alive)

    async def send_raw_message(self, message: str) -> None:
        """Deprecated: use ``send_raw()`` instead."""
        warnings.warn(
            "send_raw_message() is deprecated; use send_raw() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        await self.send_raw(message)

    async def raw_request(
        self,
        message: str,
        validator: Validator,
        timeout: Optional[timedelta] = None,
        retry: bool = False,
    ) -> str:
        """Send a raw message and wait for the matching response.

        This is the single entry point for raw request/response interactions.

        Args:
            message: Raw WebSocket message to send, properly formatted as JSON
                or Socket.IO protocol (e.g. ``'42["getBalance"]'``).
            validator: Validator instance used to identify the expected response.
            timeout: Maximum time to wait for a response. ``None`` uses the
                client's configured default timeout.
            retry: When True, retry the request on timeout or failure using the
                client's configured retry strategy. Requires ``timeout``.

        Returns:
            str: The first response matching the validator, as a raw string.

        Raises:
            ValueError: If ``retry`` is True without an explicit ``timeout``.
            ConnectionError: If the client is not connected.
            TimeoutError: If no matching response arrives in time.

        Example:
            ```python
            from BinaryOptionsToolsV2.validator import Validator

            validator = Validator.starts_with('42["balance"')
            response = await client.raw_request('42["getBalance"]', validator)
            print(json.loads(response))
            ```
        """
        if retry:
            if timeout is None:
                raise ValueError("retry=True requires an explicit timeout")
            return await self.client.create_raw_order_with_timeout_and_retry(message, validator.raw_validator, timeout)
        if timeout is not None:
            return await self.client.create_raw_order_with_timeout(message, validator.raw_validator, timeout)
        return await self.client.create_raw_order(message, validator.raw_validator)

    async def create_raw_order(self, message: str, validator: Validator) -> str:
        """Deprecated: use ``raw_request()`` instead."""
        warnings.warn(
            "create_raw_order() is deprecated; use raw_request() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.raw_request(message, validator)

    async def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Deprecated: use ``raw_request(..., timeout=...)`` instead."""
        warnings.warn(
            "create_raw_order_with_timeout() is deprecated; use raw_request(..., timeout=...) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.raw_request(message, validator, timeout=timeout)

    async def create_raw_order_with_timeout_and_retry(
        self, message: str, validator: Validator, timeout: timedelta
    ) -> str:
        """Deprecated: use ``raw_request(..., timeout=..., retry=True)`` instead."""
        warnings.warn(
            "create_raw_order_with_timeout_and_retry() is deprecated; "
            "use raw_request(..., timeout=..., retry=True) instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.raw_request(message, validator, timeout=timeout, retry=True)

    async def raw_stream(
        self,
        message: str,
        validator: Validator,
        timeout: Optional[timedelta] = None,
    ):
        """Send a raw message and stream every matching response.

        Args:
            message: Initial raw WebSocket message to send, properly formatted
                as JSON or Socket.IO protocol.
            validator: Validator instance to filter incoming messages.
            timeout: Optional timeout for the iterator session. If None, the
                iterator may continue indefinitely until closed or the
                connection ends.

        Returns:
            AsyncIterator[str]: Async iterator yielding matching messages as
                raw strings.

        Example:
            ```python
            from BinaryOptionsToolsV2.validator import Validator

            validator = Validator.starts_with('42["update"')
            stream = await client.raw_stream('42["subscribeUpdates"]', validator)
            async for response in stream:
                print(json.loads(response))
            ```
        """
        return await self.client.create_raw_iterator(message, validator.raw_validator, timeout)

    async def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        """Deprecated: use ``raw_stream()`` instead."""
        warnings.warn(
            "create_raw_iterator() is deprecated; use raw_stream() instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return await self.raw_stream(message, validator, timeout)
