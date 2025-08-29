from ..validator import Validator
from ..config import Config
from .. import RawPocketOption, Logger
from datetime import timedelta


import asyncio
import json
import sys 


class AsyncSubscription:
    def __init__(self, subscription):
        """Asyncronous Iterator over json objects"""
        self.subscription = subscription
        
    def __aiter__(self):
        return self
        
    async def __anext__(self):
        return json.loads(await anext(self.subscription))
    
# This file contains all the async code for the PocketOption Module
class PocketOptionAsync:
    def __init__(self, ssid: str, url: str | None = None, config: Config | dict | str = None, **_):
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
            This version doesn't support the `Config` class.
        Note:
            - The configuration becomes locked once initialized and cannot be modified afterwards
            - Custom URLs provided in the `url` parameter take precedence over URLs in the configuration
            - Invalid configuration values will raise appropriate exceptions
        """
        if config is not None:
            if isinstance(config, dict):
                self.config = Config.from_dict(config)
            elif isinstance(config, str):
                self.config = Config.from_json(config)
            elif isinstance(config, Config):
                self.config = config
            else:
                raise ValueError("Config must be either a Config object, dictionary, or JSON string")

            if url is not None:
                self.client = RawPocketOption.new_with_url(ssid, url)
            else:
                self.client = RawPocketOption(ssid, config)
        else: 
            self.config = Config()
            if url is not None:
                self.client = RawPocketOption.new_with_url(ssid, url)
            else:
                self.client = RawPocketOption(ssid)
        self.logger = Logger()
    
    
    async def buy(self, asset: str, amount: float, time: int, check_win: bool = False) -> tuple[str, dict]:
        """
        Places a buy (call) order for the specified asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc", "EURUSD")
            amount (float): Trade amount in account currency
            time (int): Expiry time in seconds (e.g., 60 for 1 minute)
            check_win (bool): If True, waits for trade result. Defaults to True.

        Returns:
            tuple[str, dict]: Tuple containing (trade_id, trade_details)
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
       
    async def sell(self, asset: str, amount: float, time: int, check_win: bool = False) -> tuple[str, dict]:
        """
        Places a sell (put) order for the specified asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc", "EURUSD")
            amount (float): Trade amount in account currency
            time (int): Expiry time in seconds (e.g., 60 for 1 minute)
            check_win (bool): If True, waits for trade result. Defaults to True.

        Returns:
            tuple[str, dict]: Tuple containing (trade_id, trade_details)
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
        # end_time = await self.client.get_deal_end_time(id)
        
        # if end_time is not None:
        #     duration = end_time - int(time.time())
        #     if duration <= 0:
        #         duration = 5 # If duration is less than 0 then the trade is closed and the function should take less than 5 seconds to run
        # else:
        #     duration = 5
        # duration += self.config.extra_duration
        
        # self.logger.debug(f"Timeout set to: {duration} (6 extra seconds)")
        async def check(id):
            trade = await self.client.check_win(id)
            trade = json.loads(trade)
            win = trade["profit"]
            if win > 0:
                trade["result"] = "win"
            elif win == 0:
                trade["result"] = "draw"
            else:
                trade["result"] = "loss"
            return trade
        return await check(id)
        
        
    async def get_candles(self, asset: str, period: int, offset: int) -> list[dict]:  
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            timeframe (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            period (int): Historical period in seconds to fetch

        Returns:
            list[dict]: List of candles, each containing:
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
        # raise NotImplementedError(
        #     "The get_candles method is not implemented in the PocketOptionAsync class. "
        # )
    
    async def get_candles_advanced(self, asset: str, period: int, offset: int, time: int) -> list[dict]:  
        """
        Retrieves historical candle data for an asset.

        Args:
            asset (str): Trading asset (e.g., "EURUSD_otc")
            timeframe (int): Candle timeframe in seconds (e.g., 60 for 1-minute candles)
            period (int): Historical period in seconds to fetch
            time (int): Time to fetch candles from

        Returns:
            list[dict]: List of candles, each containing:
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
        # raise NotImplementedError(
        #     "The get_candles_advanced method is not implemented in the PocketOptionAsync class. "
        # )
        
    async def balance(self) -> float:
        """
        Retrieves current account balance.

        Returns:
            float: Account balance in account currency

        Note:
            Updates in real-time as trades are completed
        """
        return await self.client.balance()
    
    async def opened_deals(self) -> list[dict]:
        "Returns a list of all the opened deals as dictionaries"
        return json.loads(await self.client.opened_deals())
        # raise NotImplementedError(
        #     "The opened_deals method is not implemented in the PocketOptionAsync class. "
        # )
    
    async def closed_deals(self) -> list[dict]:
        "Returns a list of all the closed deals as dictionaries"
        return json.loads(await self.client.closed_deals())
        # raise NotImplementedError(
        #     "The closed_deals method is not implemented in the PocketOptionAsync class. "
        # )
    
    async def clear_closed_deals(self) -> None:
        "Removes all the closed deals from memory, this function doesn't return anything"
        await self.client.clear_closed_deals()

    async def payout(self, asset: None | str | list[str] = None) -> dict | list[int] | int:
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
        return payout
    
    async def history(self, asset: str, period: int) -> list[dict]:
        "Returns a list of dictionaries containing the latest data available for the specified asset starting from 'period', the data is in the same format as the returned data of the 'get_candles' function."
        return json.loads(await self.client.history(asset, period))
    
    
    async def _subscribe_symbol_inner(self, asset: str) :
        return await self.client.subscribe_symbol(asset)
    
    async def _subscribe_symbol_chuncked_inner(self, asset: str, chunck_size: int):
        return await self.client.subscribe_symbol_chuncked(asset, chunck_size)
    
    async def _subscribe_symbol_timed_inner(self, asset: str, time: timedelta):
        return await self.client.subscribe_symbol_timed(asset, time)
    
    async def _subscribe_symbol_time_aligned_inner(self, asset: str, time: timedelta):
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
    
    async def send_raw_message(self, message: str) -> None:
        """
        Sends a raw WebSocket message without waiting for a response.
        
        Args:
            message: Raw WebSocket message to send (e.g., '42["ping"]')
        """
        # await self.client.send_raw_message(message)
        raise NotImplementedError(
            "The send_raw_message method is not implemented in the PocketOptionAsync class. "
        )
        
    async def create_raw_order(self, message: str, validator: Validator) -> str:
        """
        Sends a raw WebSocket message and waits for a validated response.
        
        Args:
            message: Raw WebSocket message to send
            validator: Validator instance to validate the response
            
        Returns:
            str: The first message that matches the validator's conditions
            
        Example:
            ```python
            validator = Validator.starts_with('451-["signals/load"')
            response = await client.create_raw_order(
                '42["signals/subscribe"]',
                validator
            )
            ```
        """
        # return await self.client.create_raw_order(message, validator.raw_validator)
        raise NotImplementedError(
            "The create_raw_order method is not implemented in the PocketOptionAsync class. "
        )
        
    async def create_raw_order_with_timeout(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """
        Similar to create_raw_order but with a timeout.
        
        Args:
            message: Raw WebSocket message to send
            validator: Validator instance to validate the response
            timeout: Maximum time to wait for a valid response
            
        Returns:
            str: The first message that matches the validator's conditions
            
        Raises:
            TimeoutError: If no valid response is received within the timeout period
        """

        # return await self.client.create_raw_order_with_timeout(message, validator.raw_validator, timeout)
        raise NotImplementedError(
            "The create_raw_order_with_timout method is not implemented in the PocketOptionAsync class. "
        )
    
    async def create_raw_order_with_timeout_and_retry(self, message: str, validator: Validator, timeout: timedelta) -> str:
        """
        Similar to create_raw_order_with_timout but with automatic retry on failure.
        
        Args:
            message: Raw WebSocket message to send
            validator: Validator instance to validate the response
            timeout: Maximum time to wait for each attempt
            
        Returns:
            str: The first message that matches the validator's conditions
        """

        # return await self.client.create_raw_order_with_timeout_and_retry(message, validator.raw_validator, timeout)
        raise NotImplementedError(
            "The create_raw_order_with_timeout_and_retry method is not implemented in the PocketOptionAsync class. "
        )
 
    async def create_raw_iterator(self, message: str, validator: Validator, timeout: timedelta | None = None):
        """
        Creates an async iterator that yields validated WebSocket messages.
        
        Args:
            message: Initial WebSocket message to send
            validator: Validator instance to filter incoming messages
            timeout: Optional timeout for the entire stream
            
        Returns:
            AsyncIterator yielding validated messages
            
        Example:
            ```python
            validator = Validator.starts_with('{"signals":')
            async for message in client.create_raw_iterator(
                '42["signals/subscribe"]',
                validator,
                timeout=timedelta(minutes=5)
            ):
                print(f"Received: {message}")
            ```
        """
        # return await self.client.create_raw_iterator(message, validator, timeout)
        raise NotImplementedError(
            "The create_raw_iterator method is not implemented in the PocketOptionAsync class. "
        )
    
    async def get_server_time(self) -> int:
        """Returns the current server time as a UNIX timestamp"""
        return await self.client.get_server_time()
    
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

async def _timeout(future, timeout: int):
    if sys.version_info[:3] >= (3,11): 
        async with asyncio.timeout(timeout):
            return await future
    else:
        return await asyncio.wait_for(future, timeout)
