import asyncio
import json
import threading
from datetime import timedelta
from typing import Dict, List, Optional, Tuple, Union

from ..config import Config
from ..validator import Validator
from .asynchronous import PocketOptionAsync


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
    """
    Synchronous handler for advanced raw WebSocket message operations.

    Provides low-level access to send messages and receive filtered responses
    based on a validator. Each handler maintains its own message stream.
    """

    def __init__(self, async_handler, loop, lock=None):
        """
        Initialize RawHandlerSync with an async handler, event loop, and optional lock.

        Args:
            async_handler: The underlying async RawHandler instance
            loop: Event loop for running async operations
            lock: Optional threading lock to ensure thread safety when multiple threads use this handler
        """
        self._handler = async_handler
        self._loop = loop
        self._lock = lock

    def send_text(self, message: str) -> None:
        """
        Send a text message through this handler.

        Args:
            message: Text message to send

        Example:
            ```python
            handler.send_text('42["ping"]')
            ```
        """
        if self._lock:
            with self._lock:
                self._loop.run_until_complete(self._handler.send_text(message))
        else:
            self._loop.run_until_complete(self._handler.send_text(message))

    def send_binary(self, data: bytes) -> None:
        """
        Send a binary message through this handler.

        Args:
            data: Binary data to send

        Example:
            ```python
            handler.send_binary(b'\\x00\\x01\\x02')
            ```
        """
        if self._lock:
            with self._lock:
                self._loop.run_until_complete(self._handler.send_binary(data))
        else:
            self._loop.run_until_complete(self._handler.send_binary(data))

    def send_and_wait(self, message: str) -> str:
        """
        Send a message and wait for the next matching response.

        Args:
            message: Message to send

        Returns:
            str: The first response that matches this handler's validator

        Example:
            ```python
            response = handler.send_and_wait('42["getBalance"]')
            data = json.loads(response)
            ```
        """
        if self._lock:
            with self._lock:
                return self._loop.run_until_complete(self._handler.send_and_wait(message))
        else:
            return self._loop.run_until_complete(self._handler.send_and_wait(message))

    def wait_next(self) -> str:
        """
        Wait for the next message that matches this handler's validator.

        Returns:
            str: The next matching message

        Example:
            ```python
            message = handler.wait_next()
            print(f"Received: {message}")
            ```
        """
        if self._lock:
            with self._lock:
                return self._loop.run_until_complete(self._handler.wait_next())
        else:
            return self._loop.run_until_complete(self._handler.wait_next())

    def subscribe(self):
        """
        Subscribe to messages matching this handler's validator.

        Returns:
            Iterator[str]: Stream of matching messages

        Example:
            ```python
            stream = handler.subscribe()
            for message in stream:
                data = json.loads(message)
                print(f"Update: {data}")
            ```
        """
        # Get the async subscription
        if self._lock:
            with self._lock:
                async_subscription = self._loop.run_until_complete(self._handler.subscribe())
        else:
            async_subscription = self._loop.run_until_complete(self._handler.subscribe())
        return SyncRawSubscription(async_subscription)

    def id(self) -> str:
        """
        Get the unique ID of this handler.

        Returns:
            str: Handler UUID
        """
        return self._handler.id()

    def close(self) -> None:
        """
        Close this handler and clean up resources.
        Note: The handler is automatically cleaned up when it goes out of scope.
        """
        if self._lock:
            with self._lock:
                self._loop.run_until_complete(self._handler.close())
        else:
            self._loop.run_until_complete(self._handler.close())


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
            client = PocketOption("your-session-id")
            balance = client.balance()
            print(f"Current balance: {balance}")
            ```

            With custom WebSocket URL:
            ```python
            client = PocketOption("your-session-id", url="wss://custom-server.com/ws")
            ```


            Using the client for trading:
            ```python
            client = PocketOption("your-session-id")
            # Place a trade
            trade_id, trade_data = client.buy("EURUSD", 1.0, 60)
            print(f"Trade placed: {trade_id}")

            # Check trade result
            result = client.check_win(trade_id)
            print(f"Trade result: {result}")
            ```

        Note:
            - Creates a new event loop for handling async operations synchronously
            - The configuration becomes locked once initialized and cannot be modified afterwards
            - Custom URLs provided in the `url` parameter take precedence over URLs in the configuration
            - Invalid configuration values will raise appropriate exceptions
            - The event loop is automatically closed when the instance is deleted
            - All async operations are wrapped to provide a synchronous interface
        """
        self.loop = asyncio.new_event_loop()
        self._lock = threading.RLock()
        self._client = PocketOptionAsync(ssid, url=url, config=config)
        # Wait for assets to ensure connection is ready
        with self._lock:
            self.loop.run_until_complete(self._client.wait_for_assets())

    @property
    def client(self):
        """Returns the underlying PocketOptionAsync client."""
        return self._client

    @property
    def config(self):
        """Returns the configuration object."""
        return self._client.config

    def __enter__(self):
        """
        Context manager entry.
        """
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """
        Context manager exit. Shuts down the client and its runner.
        """
        self.close()

    def close(self) -> None:
        """
        Explicitly closes the client and its event loop.
        """
        with self._lock:
            self.shutdown()
            if self.loop is not None and not self.loop.is_closed():
                self.loop.close()

    def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """
        Takes the asset, and amount to place a buy trade that will expire in time (in seconds).
        If check_win is True then the function will return a tuple containing the trade id and a dictionary containing the trade data and the result of the trade ("win", "draw", "loss)
        If check_win is False then the function will return a tuple with the id of the trade and the trade as a dict
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.buy(asset, amount, time, check_win))

    def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """
        Takes the asset, and amount to place a sell trade that will expire in time (in seconds).
        If check_win is True then the function will return a tuple containing the trade id and a dictionary containing the trade data and the result of the trade ("win", "draw", "loss)
        If check_win is False then the function will return a tuple with the id of the trade and the trade as a dict
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.sell(asset, amount, time, check_win))

    def check_win(self, id: str) -> dict:
        """Returns a dictionary containing the trade data and the result of the trade ("win", "draw", "loss)"""
        with self._lock:
            return self.loop.run_until_complete(self._client.check_win(id))

    def get_deal_end_time(self, trade_id: str) -> Optional[int]:
        """
        Returns the expected close time of a deal as a Unix timestamp.
        Returns None if the deal is not found.
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.get_deal_end_time(trade_id))

    def get_candles(self, asset: str, period: int, offset: int) -> List[Dict]:
        """
        Takes the asset you want to get the candles and return a list of raw candles in dictionary format
        Each candle contains:
            * time: using the iso format
            * open: open price
            * close: close price
            * high: highest price
            * low: lowest price
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.get_candles(asset, period, offset))

    def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> List[Dict]:
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

        with self._lock:
            return self.loop.run_until_complete(self._client.get_candles_advanced(asset, period, offset, time))

    def candles(self, asset: str, period: int) -> List[Dict]:
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
        with self._lock:
            return self.loop.run_until_complete(self._client.candles(asset, period))

    def balance(self) -> float:
        "Returns the balance of the account"
        with self._lock:
            return self.loop.run_until_complete(self._client.balance())

    def opened_deals(self) -> List[Dict]:
        "Returns a list of all the opened deals as dictionaries"
        with self._lock:
            return self.loop.run_until_complete(self._client.opened_deals())

    def get_pending_deals(self) -> List[Dict]:
        """
        Retrieves a list of all currently pending trade orders.

        Returns:
            List[Dict]: List of pending orders, each containing order details.
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.get_pending_deals())

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
        with self._lock:
            return self.loop.run_until_complete(
                self._client.open_pending_order(
                    open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
                )
            )

    def cancel_pending_order(self, ticket: str) -> Dict:
        """Cancels a pending order by ticket UUID."""
        with self._lock:
            return self.loop.run_until_complete(self._client.cancel_pending_order(ticket))

    def cancel_pending_orders(self, tickets: List[str]) -> List[Dict]:
        """Cancels multiple pending orders by ticket UUID."""
        with self._lock:
            return self.loop.run_until_complete(self._client.cancel_pending_orders(tickets))

    def closed_deals(self) -> List[Dict]:
        "Returns a list of all the closed deals as dictionaries"
        with self._lock:
            return self.loop.run_until_complete(self._client.closed_deals())

    def clear_closed_deals(self) -> None:
        "Removes all the closed deals from memory, this function doesn't return anything"
        with self._lock:
            self.loop.run_until_complete(self._client.clear_closed_deals())

    def payout(
        self, asset: Optional[Union[str, List[str]]] = None
    ) -> Union[Dict[str, Optional[int]], List[Optional[int]], int, None]:
        "Returns a dict of asset | payout for each asset, if 'asset' is not None then it will return the payout of the asset or a list of the payouts for each asset it was passed"
        with self._lock:
            return self.loop.run_until_complete(self._client.payout(asset))

    def history(self, asset: str, period: int) -> List[Dict]:
        "Returns a list of dictionaries containing the latest data available for the specified asset starting from 'period', the data is in the same format as the returned data of the 'get_candles' function."
        with self._lock:
            return self.loop.run_until_complete(self._client.history(asset, period))

    def compile_candles(self, asset: str, custom_period: int, lookback_period: int) -> List[Dict]:
        """Compiles custom candlesticks from raw tick history.

        This method fetches raw tick data over the specified lookback period and
        aggregates it into custom-sized candles. This enables non-standard timeframes
        like 20 seconds, 40 seconds, 90 seconds, etc.

        Args:
            asset (str): Trading asset symbol (e.g., "EURUSD_otc")
            custom_period (int): Desired candle duration in seconds (e.g., 20, 40, 90)
            lookback_period (int): Number of seconds of tick history to fetch

        Returns:
            List[Dict]: A list of dictionaries, each representing a compiled candlestick:
                - time: Candle timestamp (Unix timestamp, aligned to period boundaries)
                - open: Opening price
                - high: Highest price during the period
                - low: Lowest price during the period
                - close: Closing price

        Raises:
            ValueError: If the asset is invalid or periods are zero/negative
            ConnectionError: If the client is not connected

        Example:
            ```python
            client = PocketOption(ssid)
            # Get 20-second candles from last 5 minutes
            candles = client.compile_candles("EURUSD_otc", 20, 300)
            for candle in candles:
                print(f"{candle['time']}: O={candle['open']}, C={candle['close']}")
            ```

        Note:
            - This is a compute-intensive operation.
            - For standard timeframes, use `get_candles()` for better efficiency.
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.compile_candles(asset, custom_period, lookback_period))

    def subscribe_symbol(self, asset: str) -> SyncSubscription:
        """Returns a sync iterator over the associated asset, it will return real time raw candles and will return new candles while the 'PocketOption' class is loaded if the class is droped then the iterator will fail"""
        with self._lock:
            return SyncSubscription(self.loop.run_until_complete(self._client._subscribe_symbol_inner(asset)))

    def subscribe_symbol_chuncked(self, asset: str, chunck_size: int) -> SyncSubscription:
        """Returns a sync iterator over the associated asset, it will return real time candles formed with the specified amount of raw candles and will return new candles while the 'PocketOption' class is loaded if the class is droped then the iterator will fail"""
        with self._lock:
            return SyncSubscription(
                self.loop.run_until_complete(self._client._subscribe_symbol_chuncked_inner(asset, chunck_size))
            )

    def subscribe_symbol_timed(self, asset: str, time: timedelta) -> SyncSubscription:
        """
        Returns a sync iterator over the associated asset, it will return real time candles formed with candles ranging from time `start_time` to `start_time` + `time` allowing users to get the latest candle of `time` duration and will return new candles while the 'PocketOption' class is loaded if the class is droped then the iterator will fail
        Please keep in mind the iterator won't return a new candle exactly each `time` duration, there could be a small delay and imperfect timestamps
        """
        with self._lock:
            return SyncSubscription(
                self.loop.run_until_complete(self._client._subscribe_symbol_timed_inner(asset, time))
            )

    def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> SyncSubscription:
        """
        Returns a sync iterator over the associated asset, it will return real time candles formed with candles ranging from time `start_time` to `start_time` + `time` allowing users to get the latest candle of `time` duration and will return new candles while the 'PocketOption' class is loaded if the class is droped then the iterator will fail
        Please keep in mind the iterator won't return a new candle exactly each `time` duration, there could be a small delay and imperfect timestamps
        """
        with self._lock:
            return SyncSubscription(
                self.loop.run_until_complete(self._client._subscribe_symbol_time_aligned_inner(asset, time))
            )

    def get_server_time(self) -> int:
        """Returns the current server time as a UNIX timestamp"""
        with self._lock:
            return self.loop.run_until_complete(self._client.get_server_time())

    def is_demo(self) -> bool:
        """
        Checks if the current account is a demo account.

        Returns:
            bool: True if using a demo account, False if using a real account

        Examples:
            ```python
            # Basic account type check
            client = PocketOption(ssid)
            is_demo = client.is_demo()
            print("Using", "demo" if is_demo else "real", "account")

            # Example with balance check
            def check_account():
                is_demo = client.is_demo()
                balance = client.balance()
                print(f"{'Demo' if is_demo else 'Real'} account balance: {balance}")

            # Example with trade validation
            def safe_trade(asset: str, amount: float, duration: int):
                is_demo = client.is_demo()
                if not is_demo and amount > 100:
                    raise ValueError("Large trades should be tested in demo first")
                return client.buy(asset, amount, duration)
            ```
        """
        return self._client.is_demo()

    def wait_for_assets(self, timeout: float = 60.0) -> None:
        """
        Waits for the assets to be loaded from the server.

        Args:
            timeout (float): The maximum time to wait in seconds. Default is 60.0.

        Raises:
            TimeoutError: If the assets are not loaded within the timeout period.
        """
        with self._lock:
            self.loop.run_until_complete(self._client.wait_for_assets(timeout))

    def disconnect(self) -> None:
        """
        Disconnects the client while keeping the configuration intact.
        The connection will automatically try to re-establish if max_allowed_loops > 0.
        To completely stop the client and its runner, use shutdown().

        Example:
            ```python
            client = PocketOption(ssid)
            # Use client...
            client.disconnect()
            # The client will try to reconnect in the background...
            ```
        """
        with self._lock:
            self.loop.run_until_complete(self._client.disconnect())

    def connect(self) -> None:
        """
        Establishes a connection after a manual disconnect.
        Uses the same configuration and credentials.

        Example:
            ```python
            client.disconnect()
            # Connection is closed
            client.connect()
            # Connection is re-established
            ```
        """
        with self._lock:
            self.loop.run_until_complete(self._client.connect())

    def reconnect(self) -> None:
        """
        Disconnects and reconnects the client.

        Example:
            ```python
            client.reconnect()
            ```
        """
        with self._lock:
            self.loop.run_until_complete(self._client.reconnect())

    def unsubscribe(self, asset: str) -> None:
        """
        Unsubscribes from an asset's stream by asset name.

        Args:
            asset (str): Asset name to unsubscribe from (e.g., "EURUSD_otc")

        Example:
            ```python
            # Subscribe to asset
            subscription = client.subscribe_symbol("EURUSD_otc")
            # ... use subscription ...
            # Unsubscribe when done
            client.unsubscribe("EURUSD_otc")
            ```
        """
        with self._lock:
            self.loop.run_until_complete(self._client.unsubscribe(asset))

    def shutdown(self) -> None:
        """
        Completely shuts down the client and its background runner.
        Once shut down, the client cannot be used anymore.
        """
        with self._lock:
            self.loop.run_until_complete(self._client.shutdown())

    def create_raw_handler(self, validator: Validator, keep_alive: Optional[str] = None) -> "RawHandlerSync":
        """
        Creates a raw handler for advanced WebSocket message handling.

        Args:
            validator: Validator instance to filter incoming messages
            keep_alive: Optional message to send on reconnection

        Returns:
            RawHandlerSync: Sync handler instance for sending/receiving messages

        Example:
            ```python
            from BinaryOptionsToolsV2.validator import Validator

            validator = Validator.starts_with('42["signals"')
            handler = client.create_raw_handler(validator)

            # Send and wait for response
            response = handler.send_and_wait('42["signals/subscribe"]')

            # Or subscribe to stream
            for message in handler.subscribe():
                print(message)
            ```
        """
        with self._lock:
            async_handler = self.loop.run_until_complete(self._client.create_raw_handler(validator, keep_alive))
        return RawHandlerSync(async_handler, self.loop, self._lock)

    def send_raw_message(self, message: str) -> None:
        """Sends a raw message through the websocket without waiting for a response"""
        with self._lock:
            self.loop.run_until_complete(self._client.send_raw_message(message))

    def create_raw_order(self, message: str, validator: Validator) -> str:
        """Sends a raw message and waits for a response that matches the validator"""
        with self._lock:
            return self.loop.run_until_complete(self._client.create_raw_order(message, validator))

    def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Sends a raw message and waits for a response that matches the validator with a timeout"""
        with self._lock:
            return self.loop.run_until_complete(self._client.create_raw_order_with_timeout(message, validator, timeout))

    def create_raw_order_with_timeout_and_retry(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """Sends a raw message and waits for a response that matches the validator with a timeout and retry logic"""
        with self._lock:
            return self.loop.run_until_complete(
                self._client.create_raw_order_with_timeout_and_retry(message, validator, timeout)
            )

    def create_raw_iterator(self, message: str, validator: Validator, timeout: Optional[timedelta] = None):
        """Returns a sync iterator that yields messages matching the validator after sending the initial message"""
        with self._lock:
            async_iterator = self.loop.run_until_complete(self._client.create_raw_iterator(message, validator, timeout))
        return SyncRawSubscription(async_iterator)

    def active_assets(self) -> List[Dict]:
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
            client = PocketOption(ssid)
            active = client.active_assets()
            for asset in active:
                print(f"{asset['symbol']}: {asset['name']} (payout: {asset['payout']}%)")
            ```
        """
        with self._lock:
            return self.loop.run_until_complete(self._client.active_assets())
