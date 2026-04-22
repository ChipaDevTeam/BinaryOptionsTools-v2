import asyncio
import json
import re
import sys
from datetime import timedelta
from typing import TYPE_CHECKING, Dict, List, Optional, Tuple, Union

from ..config import Config
from ..validator import Validator

if TYPE_CHECKING:
    from ..BinaryOptionsToolsV2 import RawPocketOption

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
        """
        # The Rust Drop implementation handles cleanup automatically
        pass


# This file contains all the async code for the PocketOption Module
class PocketOptionAsync:
    def __init__(self, ssid: str, url: Optional[str] = None, config: Union[Config, dict, str] = None, **_):
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
        # SSID Sanitizer: fix common shell-stripping issues (missing quotes around "auth")
        if ssid is not None:
            ssid = re.sub(r"42\[['\"]?auth['\"]?,", '42["auth",', ssid, count=1)

        from ..tracing import Logger

        self.logger = Logger()

        # Ensure it looks like a Socket.IO message
        if ssid is not None and not ssid.startswith("42["):
            self.logger.warn(f"SSID does not start with '42[': {ssid[:20]}...")
        elif ssid is None:
            self.logger.warn("SSID is None, connection will likely fail")

        # Enforce configuration and instantiation
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

        from ..tracing import LogBuilder

        # Enable terminal logging only if explicitly requested in config
        if self.config.terminal_logging:
            try:
                lb = LogBuilder()
                lb.terminal(level=self.config.log_level)
                lb.build()
            except Exception:
                pass

        # Link to Rust Backend
        self.client: "RawPocketOption" = RawPocketOption.new_with_config(ssid, self.config.pyconfig)

    async def __aenter__(self):
        """
        Context manager entry. Waits for assets to be loaded.
        """
        await self.wait_for_assets(timeout=60.0)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """
        Context manager exit. Shuts down the client and its runner.
        """
        await self.shutdown()

    async def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """
        Places a buy (call) order for the specified asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc", "EURUSD")
            amount (float): Trade amount in account currency
            time (int): Expiry time in seconds (e.g., 60 for 1 minute)
            check_win (bool): If True, waits for trade result. Defaults to False.

        Returns:
            Tuple[str, Dict]: Tuple containing (trade_id, trade_details)
            trade_details includes:
                - asset: Trading asset
                - amount: Trade amount
                - direction: "buy"
                - expiry: Expiry timestamp
                - result: Trade result if check_win=True ("win"/"loss"/"draw")
                - profit: Profit amount if check_win=True

        Raises:
            ConnectionError: If connection to platform fails
            ValueError: If invalid parameters are provided
            TimeoutError: If trade confirmation times out
        """
        (trade_id, trade) = await self.client.buy(asset, amount, time)
        if check_win:
            return trade_id, await self.check_win(trade_id)
        else:
            trade = json.loads(trade)
            return trade_id, trade

    async def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """
        Places a sell (put) order for the specified asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc", "EURUSD")
            amount (float): Trade amount in account currency
            time (int): Expiry time in seconds (e.g., 60 for 1 minute)
            check_win (bool): If True, waits for trade result. Defaults to False.

        Returns:
            Tuple[str, Dict]: Tuple containing (trade_id, trade_details)
            trade_details includes:
                - asset: Trading asset
                - amount: Trade amount
                - direction: "sell"
                - expiry: Expiry timestamp
                - result: Trade result if check_win=True ("win"/"loss"/"draw")
                - profit: Profit amount if check_win=True

        Raises:
            ConnectionError: If connection to platform fails
            ValueError: If invalid parameters are provided
            TimeoutError: If trade confirmation times out
        """
        (trade_id, trade) = await self.client.sell(asset, amount, time)
        if check_win:
            return trade_id, await self.check_win(trade_id)
        else:
            trade = json.loads(trade)
            return trade_id, trade

    async def check_win(self, id: str) -> dict:
        """
        Checks the result of a specific trade.

        Args:
            trade_id (str): ID of the trade to check

        Returns:
            dict: Trade result containing:
                - result: "win", "loss", or "draw"
                - profit: Profit/loss amount
                - details: Additional trade details
                - timestamp: Result timestamp

        Raises:
            ValueError: If trade_id is invalid
            TimeoutError: If result check times out
        """

        # Let Rust handle the timeout based on the trade close timestamp.
        return await self._get_trade_result(id)

    async def get_deal_end_time(self, trade_id: str) -> Optional[int]:
        """
        Returns the expected close time of a deal as a Unix timestamp.
        Returns None if the deal is not found.
        """
        return await self.client.get_deal_end_time(trade_id)

    async def _get_trade_result(self, id: str) -> dict:
        """Internal method to get trade result with timeout protection"""
        try:
            # The Rust client should handle its own timeout, but we'll add a safeguard
            trade = await self.client.check_win(id)
            trade = json.loads(trade)
            win = float(trade["profit"])
            if win > 0:
                trade["result"] = "win"
            elif win == 0:
                trade["result"] = "draw"
            else:
                trade["result"] = "loss"
            return trade
        except Exception as e:
            # Catch any other errors from the Rust client
            raise Exception(f"Error getting trade result for ID {id}: {str(e)}")

    async def candles(self, asset: str, period: int) -> List[Dict]:
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            period (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)

        Returns:
            List[Dict]: List of candles, each containing:
                - time: Candle timestamp
                - open: Opening price
                - high: Highest price
                - low: Lowest price
                - close: Closing price
        """
        candles = await self.client.candles(asset, period)
        return json.loads(candles)

    async def get_candles(self, asset: str, period: int, offset: int) -> List[Dict]:
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            timeframe (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            period (int): Historical period in seconds to fetch

        Returns:
            List[Dict]: List of candles, each containing:
                - time: Candle timestamp
                - open: Opening price
                - high: Highest price
                - low: Lowest price
                - close: Closing price

        Note:
            Available timeframes: 1, 5, 15, 30, 60, 300 seconds
            Maximum period depends on the timeframe
        """
        candles = await self.client.get_candles(asset, period, offset)
        return json.loads(candles)

    async def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            timeframe (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            period (int): Historical period in seconds to fetch
            time (int): Time to fetch candles from

        Returns:
            List[Dict]: List of candles, each containing:
                - time: Candle timestamp
                - open: Opening price
                - high: Highest price
                - low: Lowest price
                - close: Closing price

        Note:
            Available timeframes: 1, 5, 15, 30, 60, 300 seconds
            Maximum period depends on the timeframe
        """
        candles = await self.client.get_candles_advanced(asset, period, offset, time)
        return json.loads(candles)

    async def balance(self) -> float:
        """
        Retrieves current account balance.

        Returns:
            float: Account balance in account currency

        Note:
            Updates in real-time as trades are completed
        """
        return await self.client.balance()

    async def opened_deals(self) -> List[Dict]:
        """Retrieves a list of all currently open (active) deals.

        This method returns all deals that are currently active/open on the account,
        including both pending and executed trades that have not yet closed.

        Returns:
            List[Dict]: A list of dictionaries, each representing an open deal with details such as:
                - id: Unique deal identifier
                - asset: Trading asset symbol
                - amount: Trade amount
                - direction: "buy" or "sell"
                - entry_price: Entry price of the trade
                - current_price: Current market price
                - expiry: Expiration timestamp
                - status: Current status of the deal
                - timestamp: Deal creation time
                - profit: Potential or realized profit (may be None for open deals)

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

            Filtering active deals:
            ```python
            async def monitor_open_deals(client):
                deals = await client.opened_deals()
                total_value = sum(d['amount'] for d in deals)
                print(f"Open deals: {len(deals)}, Total exposure: {total_value}")
            ```
        """
        return json.loads(await self.client.opened_deals())

    async def get_pending_deals(self) -> List[Dict]:
        """
        Retrieves a list of all currently pending trade orders.

        Returns:
            List[Dict]: List of pending orders, each containing order details.
        """
        return json.loads(await self.client.get_pending_deals())

    async def open_pending_order(
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
        """
        Opens a pending order on the PocketOption platform.

        Args:
            open_type (int): The type of the pending order.
            amount (float): The amount to trade.
            asset (str): The asset symbol (e.g., "EURUSD_otc").
            open_time (int): The server time to open the trade (Unix timestamp).
            open_price (float): The price to open the trade at.
            timeframe (int): The duration of the trade in seconds.
            min_payout (int): The minimum payout percentage required.
            command (int): The trade direction (0 for Call, 1 for Put).

        Returns:
            Dict: The created pending order details.
        """
        order = await self.client.open_pending_order(
            open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
        )
        return json.loads(order)

    async def cancel_pending_order(self, ticket: str) -> Dict:
        """Cancels a pending order by ticket UUID."""
        result = await self.client.cancel_pending_order(ticket)
        return json.loads(result)

    async def cancel_pending_orders(self, tickets: List[str]) -> List[Dict]:
        """Cancels multiple pending orders by ticket UUID."""
        result = await self.client.cancel_pending_orders(tickets)
        return json.loads(result)

    async def closed_deals(self) -> List[Dict]:
        """Retrieves a list of all closed/completed deals.

        This method returns all deals that have been completed, including trades
        that have expired and reached a final outcome (win, loss, or draw).

        Returns:
            List[Dict]: A list of dictionaries, each representing a closed deal with details such as:
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
            Basic usage:
            ```python
            async with PocketOptionAsync(ssid) as client:
                closed = await client.closed_deals()
                for deal in closed:
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

    async def payout(
        self, asset: Optional[Union[str, List[str]]] = None
    ) -> Union[Dict[str, Optional[int]], List[Optional[int]], int, None]:
        """
        Retrieves current payout percentages for all assets.

        Returns:
            dict: Asset payouts mapping:
                {
                    "EURUSD_otc": 85,  # 85% payout
                    "GBPUSD": 82,      # 82% payout
                    ...
                }
            list: If asset is a list, returns a list of payouts for each asset in the same order
            int: If asset is a string, returns the payout for that specific asset
            none: If asset didn't match and valid asset none will be returned
        """
        payout = json.loads(await self.client.payout())
        if isinstance(asset, str):
            return payout.get(asset)
        elif isinstance(asset, list):
            return [payout.get(ast) for ast in asset]
        else:
            return payout

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
        return json.loads(await self.client.history(asset, period))

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

        return json.loads(await self.client.compile_candles(asset, custom_period, lookback_period))

    async def _subscribe_symbol_inner(self, asset: str):
        """Internal method to establish a real-time subscription for an asset.

        This method directly calls the underlying client's subscribe_symbol method
        and is used internally by `subscribe_symbol()`. It returns a raw subscription
        iterator that yields JSON strings.

        Args:
            asset (str): Trading asset symbol to subscribe to (e.g., "EURUSD_otc")

        Returns:
            AsyncIterator[str]: Raw async iterator yielding JSON string messages

        Note:
            This is an internal method. Users should typically use `subscribe_symbol()`
            which wraps this method in an `AsyncSubscription` for easier handling.
        """
        return await self.client.subscribe_symbol(asset)

    async def _subscribe_symbol_chuncked_inner(self, asset: str, chunck_size: int):
        """Internal method to establish a chunked real-time subscription for an asset.

        This method creates a subscription that aggregates raw price updates into
        candlesticks of the specified chunk size. It directly calls the underlying
        client's subscribe_symbol_chuncked method and returns a raw subscription iterator.

        Args:
            asset (str): Trading asset symbol to subscribe to (e.g., "EURUSD_otc")
            chunck_size (int): Number of raw ticks to aggregate into each candle.
                For example, chunck_size=10 will create a candle from every 10 price ticks.

        Returns:
            AsyncIterator[str]: Raw async iterator yielding JSON string messages,
                each representing a completed candlestick.

        Note:
            This is an internal method. Users should typically use `subscribe_symbol_chuncked()`
            which wraps this method in an `AsyncSubscription` for easier handling.
        """
        return await self.client.subscribe_symbol_chuncked(asset, chunck_size)

    async def _subscribe_symbol_timed_inner(self, asset: str, time: timedelta):
        """Internal method to establish a timed real-time subscription for an asset.

        This method creates a subscription that yields price updates at regular
        time intervals. It directly calls the underlying client's subscribe_symbol_timed
        method and returns a raw subscription iterator.

        Args:
            asset (str): Trading asset symbol to subscribe to (e.g., "EURUSD_otc")
            time (timedelta): Time interval between updates. For example, timedelta(seconds=5)
                will yield an update every 5 seconds.

        Returns:
            AsyncIterator[str]: Raw async iterator yielding JSON string messages
                at the specified time intervals.

        Note:
            This is an internal method. Users should typically use `subscribe_symbol_timed()`
            which wraps this method in an `AsyncSubscription` for easier handling.
        """
        return await self.client.subscribe_symbol_timed(asset, time)

    async def _subscribe_symbol_time_aligned_inner(self, asset: str, time: timedelta):
        """Internal method to establish a time-aligned real-time subscription.

        This method creates a subscription that yields price updates aligned to
        specific time boundaries (e.g., on the minute, on the hour). It directly
        calls the underlying client's subscribe_symbol_time_aligned method and
        returns a raw subscription iterator.

        Args:
            asset (str): Trading asset symbol to subscribe to (e.g., "EURUSD_otc")
            time (timedelta): Time alignment interval. For example, timedelta(minutes=1)
                will align updates to the start of each minute.

        Returns:
            AsyncIterator[str]: Raw async iterator yielding JSON string messages
                aligned to the specified time boundaries.

        Note:
            This is an internal method. Users should typically use `subscribe_symbol_time_aligned()`
            which wraps this method in an `AsyncSubscription` for easier handling.
        """
        return await self.client.subscribe_symbol_time_aligned(asset, time)

    async def subscribe_symbol(self, asset: str) -> AsyncSubscription:
        """
        Creates a real-time data subscription for an asset.

        Args:
            asset (str): Trading asset to subscribe to

        Returns:
            AsyncSubscription: Async iterator yielding real-time price updates

        Example:
            ```python
            async with api.subscribe_symbol("EURUSD_otc") as subscription:
                async for update in subscription:
                    print(f"Price update: {update}")
            ```
        """
        return AsyncSubscription(await self._subscribe_symbol_inner(asset))

    async def subscribe_symbol_chuncked(self, asset: str, chunck_size: int) -> AsyncSubscription:
        """Returns an async iterator over the associated asset, it will return real time candles formed with the specified amount of raw candles and will return new candles while the 'PocketOptionAsync' class is loaded if the class is droped then the iterator will fail"""
        return AsyncSubscription(await self._subscribe_symbol_chuncked_inner(asset, chunck_size))

    async def subscribe_symbol_timed(self, asset: str, time: timedelta) -> AsyncSubscription:
        """
        Creates a timed real-time data subscription for an asset.

        Args:
            asset (str): Trading asset to subscribe to
            interval (int): Update interval in seconds

        Returns:
            AsyncSubscription: Async iterator yielding price updates at specified intervals

        Example:
            ```python
            # Get updates every 5 seconds
            async with api.subscribe_symbol_timed("EURUSD_otc", 5) as subscription:
                async for update in subscription:
                    print(f"Timed update: {update}")
            ```
        """
        return AsyncSubscription(await self._subscribe_symbol_timed_inner(asset, time))

    async def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> AsyncSubscription:
        """
        Creates a time-aligned real-time data subscription for an asset.

        Args:
            asset (str): Trading asset to subscribe to
            time (timedelta): Time interval for updates

        Returns:
            AsyncSubscription: Async iterator yielding price updates aligned with specified time intervals

        Example:
            ```python
            # Get updates aligned with 1-minute intervals
            async with api.subscribe_symbol_time_aligned("EURUSD_otc", timedelta(minutes=1)) as subscription:
                async for update in subscription:
                    print(f"Time-aligned update: {update}")
            ```
        """
        return AsyncSubscription(await self._subscribe_symbol_time_aligned_inner(asset, time))

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

    async def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandler":
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
            handler = await client.create_raw_handler(validator)

            # Send and wait for response
            response = await handler.send_and_wait('42["signals/subscribe"]')

            # Or subscribe to stream
            async for message in handler.subscribe():
                print(message)
            ```
        """
        rust_handler = await self.client.create_raw_handler(validator.raw_validator, keep_alive)
        return RawHandler(rust_handler)

    async def send_raw_message(self, message: str) -> None:
        """Sends a raw WebSocket message without waiting for a response.

        This method allows sending arbitrary WebSocket messages directly to the server.
        It is fire-and-forget - no response is expected or returned. Useful for
        sending commands that don't require acknowledgment or for one-way communication.

        Args:
            message (str): Raw WebSocket message to send. Must be properly formatted
                as a JSON string or Socket.IO protocol message (e.g., '42["event",{"data":...}]')

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the message format is invalid

        Examples:
            Send a simple ping:
            ```python
            async with PocketOptionAsync(ssid) as client:
                await client.send_raw_message('42["ping"]')
            ```

            Send custom event:
            ```python
            async def send_custom_notification():
                async with PocketOptionAsync(ssid) as client:
                    payload = {"event": "notification", "message": "Hello"}
                    await client.send_raw_message(f'42{json.dumps(payload)}')
            ```

            Broadcast to channel:
            ```python
            async def broadcast_to_channel(channel: str, data: dict):
                async with PocketOptionAsync(ssid) as client:
                    message = f'42["join",{{"channel":"{channel}"}}]'
                    await client.send_raw_message(message)
            ```
        """
        await self.client.send_raw_message(message)

    async def create_raw_order(self, message: str, validator: Validator) -> str:
        """Sends a raw message and waits for a matching response.

        This method sends a WebSocket message and blocks until a response is received
        that matches the provided validator. It is the basic request-response pattern
        for custom API interactions.

        Args:
            message (str): Raw WebSocket message to send, properly formatted as JSON
                or Socket.IO protocol (e.g., '42["getBalance"]')
            validator (Validator): Validator instance used to filter and identify
                the expected response. The validator determines which incoming
                messages are considered matching responses.

        Returns:
            str: The first response message that matches the validator, as a raw string.
                Typically this is a JSON string that can be parsed with `json.loads()`.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the message format is invalid or validator doesn't match
            TimeoutError: If no matching response is received within the default timeout

        Examples:
            Basic request-response:
            ```python
            from BinaryOptionsToolsV2.validator import Validator

            async def get_balance():
                async with PocketOptionAsync(ssid) as client:
                    validator = Validator.starts_with('42["balance"')
                    response = await client.create_raw_order('42["getBalance"]', validator)
                    balance_data = json.loads(response)
                    print(f"Balance: {balance_data}")
            ```

            Query specific trade:
            ```python
            async def get_trade_details(trade_id: str):
                async with PocketOptionAsync(ssid) as client:
                    msg = f'42["getTrade",{{"id":"{trade_id}"}}]'
                    validator = Validator.contains('"trade"')
                    response = await client.create_raw_order(msg, validator)
                    return json.loads(response)
            ```

        Note:
            The default timeout is determined by the client configuration. For more
            control over timeout behavior, use `create_raw_order_with_timeout()`.
        """
        return await self.client.create_raw_order(message, validator.raw_validator)

    async def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Sends a raw message and waits for a matching response with a custom timeout.

        This method is similar to `create_raw_order()` but allows specifying a
        custom timeout duration. It sends a WebSocket message and blocks until
        a response matching the validator is received or the timeout expires.

        Args:
            message (str): Raw WebSocket message to send, properly formatted as JSON
                or Socket.IO protocol (e.g., '42["getBalance"]')
            validator (Validator): Validator instance to filter and identify the
                expected response.
            timeout (timedelta): Maximum time to wait for a response. For example,
                `timedelta(seconds=30)` will wait up to 30 seconds.

        Returns:
            str: The first response message that matches the validator, as a raw string.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the message format is invalid or validator doesn't match
            TimeoutError: If no matching response is received within the specified timeout

        Examples:
            Short timeout for quick operations:
            ```python
            from datetime import timedelta

            async def quick_request():
                async with PocketOptionAsync(ssid) as client:
                    validator = Validator.starts_with('42["pong"')
                    try:
                        response = await client.create_raw_order_with_timeout(
                            '42["ping"]', validator, timedelta(seconds=5)
                        )
                        print(f"Pong: {response}")
                    except TimeoutError:
                        print("Server did not respond in time")
            ```

            Longer timeout for complex operations:
            ```python
            async def fetch_historical_data(asset: str, days: int):
                async with PocketOptionAsync(ssid) as client:
                    msg = f'42["history",{{"asset":"{asset}","days":{days}}}]'
                    validator = Validator.json_path("$.data")
                    # Allow up to 60 seconds for historical data fetch
                    response = await client.create_raw_order_with_timeout(
                        msg, validator, timedelta(seconds=60)
                    )
                    return json.loads(response)
            ```
        """
        return await self.client.create_raw_order_with_timeout(message, validator.raw_validator, timeout)

    async def create_raw_order_with_timeout_and_retry(
        self, message: str, validator: Validator, timeout: timedelta
    ) -> str:
        """Sends a raw message with timeout and automatic retry logic.

        This method extends `create_raw_order_with_timeout()` by adding automatic
        retry logic. If the request fails or times out, it will automatically
        retry the operation, providing enhanced reliability for flaky connections
        or temporary server issues.

        Args:
            message (str): Raw WebSocket message to send, properly formatted as JSON
                or Socket.IO protocol.
            validator (Validator): Validator instance to filter and identify the
                expected response.
            timeout (timedelta): Maximum time to wait for each attempt. For example,
                `timedelta(seconds=30)` sets a 30-second timeout per try.

        Returns:
            str: The first response message that matches the validator, as a raw string.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the message format is invalid or validator doesn't match
            TimeoutError: If all retry attempts fail to receive a matching response

        Examples:
            Reliable request with retries:
            ```python
            from datetime import timedelta

            async def reliable_fetch():
                async with PocketOptionAsync(ssid) as client:
                    validator = Validator.starts_with('42["data"')
                    try:
                        response = await client.create_raw_order_with_timeout_and_retry(
                            '42["fetch"]', validator, timedelta(seconds=30)
                        )
                        return json.loads(response)
                    except TimeoutError:
                        print("All retry attempts exhausted")
            ```

            Critical operation with guaranteed delivery:
            ```python
            async def place_critical_order(asset: str, amount: float):
                async with PocketOptionAsync(ssid) as client:
                    msg = f'42["order",{{"asset":"{asset}","amount":{amount}}}]'
                    validator = Validator.contains('"order_id"')
                    # Retry with 30s timeout per attempt
                    response = await client.create_raw_order_with_timeout_and_retry(
                        msg, validator, timedelta(seconds=30)
                    )
                    return json.loads(response)
            ```

        Note:
            The retry strategy (number of retries, backoff behavior) is determined
            by the underlying Rust client configuration. Check the client config for
            retry-related parameters.
        """
        return await self.client.create_raw_order_with_timeout_and_retry(message, validator.raw_validator, timeout)

    async def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        """Creates an async iterator for streaming responses.

        This method sends an initial message and returns an async iterator that yields
        all subsequent messages matching the validator. It is useful for subscribing
        to a stream of responses or for scenarios where multiple responses are expected
        to a single request.

        Args:
            message (str): Initial raw WebSocket message to send, properly formatted
                as JSON or Socket.IO protocol.
            validator (Validator): Validator instance to filter incoming messages.
                Only messages matching this validator will be yielded by the iterator.
            timeout (timedelta | None, optional): Optional timeout for the entire
                iterator session. If None, the iterator may continue indefinitely
                until closed or the connection ends. Defaults to None.

        Returns:
            AsyncIterator[str]: Async iterator yielding matching response messages
                as raw strings. Each item can be parsed with `json.loads()`.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the message format is invalid

        Examples:
            Stream multiple responses:
            ```python
            from BinaryOptionsToolsV2.validator import Validator

            async def stream_updates():
                async with PocketOptionAsync(ssid) as client:
                    validator = Validator.starts_with('42["update"')
                    iterator = await client.create_raw_iterator(
                        '42["subscribeUpdates"]', validator, timeout=timedelta(minutes=5)
                    )
                    async for response in iterator:
                        data = json.loads(response)
                        print(f"Update: {data}")
            ```

            Collect all items into a list:
            ```python
            async def collect_all():
                async with PocketOptionAsync(ssid) as client:
                    validator = Validator.contains('"item"')
                    iterator = await client.create_raw_iterator(
                        '42["getAll"]', validator
                    )
                    items = []
                    async for response in iterator:
                        items.append(json.loads(response))
                    return items
            ```

            Example:
            ```python
            async def bounded_stream():
                async with PocketOptionAsync(ssid) as client:
                    validator = Validator.regex(r'42\\["signal"')
                    stream = await client.create_raw_iterator(
                        '42["startSignals"]', validator
                    )
                    async for signal in stream:
                        process_signal(json.loads(signal))
            ```

        Note:
            The iterator will continue yielding messages until:
            - The connection is closed or times out
            - The client is shut down
            - An exception occurs
            - The optional timeout expires (if specified)

            Proper cleanup is handled automatically when using the iterator as an
            async context manager or when it is garbage collected.
        """
        return await self.client.create_raw_iterator(message, validator.raw_validator, timeout)


async def _timeout(future, timeout: int):
    if sys.version_info[:3] >= (3, 11):
        async with asyncio.timeout(timeout):
            return await future
    else:
        return await asyncio.wait_for(future, timeout)
