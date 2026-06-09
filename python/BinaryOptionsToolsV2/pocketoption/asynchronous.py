import asyncio
import json
import re
import sys
from datetime import timedelta
from typing import TYPE_CHECKING, Dict, List, Optional, Tuple, Union

from ..config import Config
from ..validator import Validator

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

    warnings_list = []

    required_fields = ["session", "uid"]
    for field in required_fields:
        if field not in auth_data:
            warnings_list.append(f"missing required field '{field}'")

    session = auth_data.get("session", "")
    if session and not re.match(r'^[a-zA-Z0-9_\-]{10,}$', str(session)):
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
            "Ensure your SSID follows the format: 42['auth',{{'session':'...','uid':123,...}}]")

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

        if ssid is not None:
            ssid = sanitize_and_validate_ssid(ssid, self.logger)
            if not ssid.startswith("42["):
                self._ssid_valid = False
            else:
                try:
                    payload = json.loads(ssid[2:])
                    if not isinstance(payload, list) or len(payload) < 2:
                        self._ssid_valid = False
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

    async def _place_trade(self, method, asset: str, amount: float, time: int, check_win: bool) -> Tuple[str, Dict]:
        """Internal helper to place a trade and optionally wait for the result."""
        trade_id, trade = await method(asset, amount, time)
        if check_win:
            return trade_id, await self.check_win(trade_id, timeout_seconds=time + 30)
        trade = json.loads(trade)
        return trade_id, trade

    async def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """Places a buy (call) order."""
        return await self._place_trade(self.client.buy, asset, amount, time, check_win)

    async def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> Tuple[str, Dict]:
        """Places a sell (put) order."""
        return await self._place_trade(self.client.sell, asset, amount, time, check_win)

    async def check_win(self, id: str, timeout_seconds: Optional[int] = None) -> dict:
        """
        Checks the result of a specific trade.

        Args:
            id (str): ID of the trade to check.
            timeout_seconds (Optional[int]): Maximum time in seconds to wait for the trade result.
                If None, uses the configured default (default: 300s).
                When called from buy()/sell() with check_win=True, this is automatically
                set to trade_duration + 15 seconds to account for server processing.

        Returns:
            dict: Trade result containing:
                - result: "win", "loss", or "draw"
                - profit: Profit/loss amount
                - details: Additional trade details
                - timestamp: Result timestamp

        Raises:
            ValueError: If trade_id is invalid
            TimeoutError: If result check times out

        Example:
            ```python
            # For a 60-second trade, use a 75-second timeout
            result = await client.check_win(trade_id, timeout_seconds=75)
            ```
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
        """
        Returns the expected close time of a deal as a Unix timestamp.
        Returns None if the deal is not found.
        """
        return await self.client.get_deal_end_time(trade_id)

    async def _get_trade_result(self, id: str) -> dict:
        """Internal method to retrieve and classify trade result with timeout protection.

        Fetches the trade result from the Rust backend, parses the JSON response,
        and classifies the outcome as 'win', 'loss', or 'draw' based on the profit value.

        Args:
            id (str): The unique trade identifier to look up.

        Returns:
            dict: Trade result dictionary containing:
                - id (str): The trade identifier
                - profit (float): The profit/loss amount
                - result (str): Classified outcome ("win", "loss", or "draw")
                - Additional fields from the server response

        Raises:
            Exception: Wraps any error from the Rust client with context about the trade ID.
            ValueError: If the profit field cannot be converted to float.
            KeyError: If the response dict is missing required fields.
            json.JSONDecodeError: If the server response is not valid JSON.
        """
        try:
            trade = await self.client.check_win(id)
            trade = json.loads(trade)
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
            period (int): Historical period in seconds to fetch
            offset (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)

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
            period (int): Historical period in seconds to fetch
            offset (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
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

    async def opened_deals(self) -> List[str]:
        """Retrieves a list of all currently open (active) deals.

        This method returns all deals ids that are currently active/open on the account,
        including both pending and executed trades that have not yet closed.

        Returns:
            List[str]: List of currently opened deals IDs in UUID format.

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the response format is invalid

        Examples:
            Basic usage:
            ```python
            async with PocketOptionAsync(ssid) as client:
                open_deals_ids = await client.opened_deals()
                open_deals = [await client.get_opened_deal(deal_id) for deal_id in open_deals_ids]
                for deal in open_deals:
                    print(f"Deal {deal['id']}: {deal['asset']} {deal['direction']}")
            ```

            Filtering active deals:
            ```python
            async def monitor_open_deals(client):
                deals_ids = await client.opened_deals()
                deals = [await client.get_opened_deal(deal_id) for deal_id in deals_ids]
                total_value = sum(d['amount'] for d in deals)
                print(f"Open deals: {len(deals)}, Total exposure: {total_value}")
            ```
        """
        return json.loads(await self.client.opened_deals())

    async def get_opened_deal(self, id: str) -> Optional[Dict]:
        """
        Retrieves details of a specific opened deal by its ID.

        Args:
            id (str): The unique identifier of the deal to retrieve

        Returns:
            Optional[Dict]: A dictionary containing deal details if found, otherwise None.
            Deal details include:
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
        deal_json = await self.client.get_opened_deal(id)
        if deal_json is None:
            return None
        return json.loads(deal_json)
    
    async def open_pending_order(
        self,
        open_type: int,
        amount: float,
        asset: str,
        open_time: Union[int, str],
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
            open_time (int | str): The server time to open the trade. 
                Can be a Unix timestamp (int) or a formatted string "YYYY-MM-DD HH:MM:SS".
            open_price (float): The price to open the trade at.
            timeframe (int): The duration of the trade in seconds.
            min_payout (int): The minimum payout percentage required.
            command (int): The trade direction (0 for Call, 1 for Put).

        Returns:
            Dict: The created pending order details.
        """
        # Backward compatibility: If the underlying Rust client still expects an integer
        # but we received a string, try to convert it if it's numeric, or fallback to 0.
        # This handles cases where the binary extension hasn't been updated to support strings.
        actual_open_time = open_time
        try:
            # We try to call it with the original value first
            order = await self.client.open_pending_order(
                open_type, amount, asset, actual_open_time, open_price, timeframe, min_payout, command
            )
        except TypeError as e:
            if "object cannot be interpreted as an integer" in str(e) and isinstance(open_time, str):
                # Fallback: if it's a string like "0", convert to 0
                if open_time == "0":
                    actual_open_time = 0
                else:
                    # Try to parse Unix timestamp from string if it's just a number
                    try:
                        actual_open_time = int(open_time)
                    except ValueError:
                        # It's a formatted date string, but the binary wants an int.
                        # We can't easily convert "YYYY-MM-DD" to timestamp without more info,
                        # but for the sake of not crashing, we'll try to parse it or use 0.
                        from datetime import datetime
                        try:
                            # PocketOption strings are usually UTC
                            dt = datetime.strptime(open_time, '%Y-%m-%d %H:%M:%S')
                            actual_open_time = int(dt.timestamp())
                        except Exception:
                            actual_open_time = 0
                
                # Retry with converted integer
                order = await self.client.open_pending_order(
                    open_type, amount, asset, actual_open_time, open_price, timeframe, min_payout, command
                )
            else:
                raise

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

    async def closed_deals(self) -> List[str]:
        """Retrieves a list of all closed/completed deals.

        This method returns the ID of all deals that have been completed, including trades
        that have expired and reached a final outcome (win, loss, or draw).

        Returns:
            List[str]: A list of IDs, each representing a closed deal with details obtainable with the `get_closed_deal` method.:

        Raises:
            ConnectionError: If the client is not connected to the platform
            ValueError: If the response format is invalid

        Examples:
            Basic usage:
            ```python
            async with PocketOptionAsync(ssid) as client:
                closed = await client.closed_deals()
                closed = [await client.get_closed_deal(deal_id) for deal_id in closed]
                for deal in closed:
                    print(f"Deal {deal['id']}: {deal['result']} (profit: {deal['profit']})")
            ```

            Calculate total profit/loss:
            ```python
            async def calculate_pnl():
                async with PocketOptionAsync(ssid) as client:
                    closed_ids = await client.closed_deals()
                    closed = [await client.get_closed_deal(deal_id) for deal_id in closed_ids]
                    total_pnl = sum(d['profit'] for d in closed)
                    wins = sum(1 for d in closed if d['result'] == 'win')
                    print(f"Total P/L: {total_pnl}, Win rate: {wins}/{len(closed)}")
            ```
        """
        return json.loads(await self.client.closed_deals())

    async def get_closed_deal(self, id: str) -> Optional[Dict]:
        """
        Retrieves details of a specific closed deal by its ID.

        Args:
            id (str): The unique identifier of the closed deal to retrieve
        Returns:
            Optional[Dict]: The details of the closed deal if found, otherwise None
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
        deal_json = await self.client.get_closed_deal(id)
        if deal_json is None:
            return None
        return json.loads(deal_json)


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

    async def subscribe_symbol(self, asset: str) -> AsyncSubscription:
        """Subscribe to real-time raw price updates for an asset.

        Returns an async iterator yielding JSON-parsed price updates.
        """
        return AsyncSubscription(await self.client.subscribe_symbol(asset))

    async def subscribe_symbol_chunked(self, asset: str, chunk_size: int) -> AsyncSubscription:
        """Subscribe with chunked candle aggregation (n raw ticks per candle)."""
        return AsyncSubscription(await self.client.subscribe_symbol_chunked(asset, chunk_size))

    async def subscribe_symbol_timed(self, asset: str, time: timedelta) -> AsyncSubscription:
        """Subscribe with a fixed time-interval candle window."""
        return AsyncSubscription(await self.client.subscribe_symbol_timed(asset, time))

    async def subscribe_symbol_time_aligned(self, asset: str, time: timedelta) -> AsyncSubscription:
        """Subscribe with candles aligned to clock boundaries."""
        return AsyncSubscription(await self.client.subscribe_symbol_time_aligned(asset, time))

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

    def max_subscriptions(self) -> int:
        """
        Returns the configured maximum number of concurrent subscriptions.

        Returns:
            int: Maximum number of concurrent asset subscriptions allowed
        """
        return self.client.max_subscriptions()

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



