import asyncio
import json
import re
import sys
import warnings
from datetime import timedelta
from typing import TYPE_CHECKING, Dict, List, Optional, Tuple, Union, AsyncGenerator

from ..config import Config
from ..validator import Validator

if TYPE_CHECKING:
    from ..BinaryOptionsToolsV2 import Logger, RawCloseOption

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
        """Asynchronous Iterator over raw messages"""
        self.subscription = subscription

    def __aiter__(self):
        return self

    async def __anext__(self):
        return await anext(self.subscription)


class RawHandler:
    """
    Asynchronous handler for advanced raw WebSocket message operations.
    Provides low-level access to send custom messages and wait for specific responses.
    """

    def __init__(self, handler):
        self._handler = handler

    async def send(self, message: str) -> None:
        """Send a raw message through the WebSocket connection."""
        await self._handler.send(message)

    async def wait_for(self, event: str, timeout: float = 30.0) -> dict:
        """Wait for a specific event from the server."""
        return await self._handler.wait_for(event, timeout)

    async def subscribe(self, event: str) -> AsyncRawSubscription:
        """Subscribe to a specific event type."""
        sub = await self._handler.subscribe(event)
        return AsyncRawSubscription(sub)

    async def close(self) -> None:
        """Close the raw handler and release resources."""
        if self._handler:
            await self._handler.close()
            self._handler = None  # Release reference to allow Rust Drop

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()


def sanitize_and_validate_ssid(ssid: str, logger: "Logger") -> str:
    """Sanitize SSID format and validate session payload semantics."""
    if not ssid or not isinstance(ssid, str):
        raise ValueError("SSID must be a non-empty string")

    ssid = ssid.strip()

    # Try to parse as JSON first
    if ssid.startswith('{') and ssid.endswith('}'):
        try:
            data = json.loads(ssid)
            if isinstance(data, dict):
                required = ['token', 'sid']
                if all(k in data for k in required):
                    logger.debug("SSID parsed as JSON with token and sid")
                    return ssid
        except json.JSONDecodeError:
            pass

    # Try pipe-delimited format: token|sid|demo|public_code|hidden_code
    parts = ssid.split('|')
    if len(parts) >= 3:
        logger.debug(f"SSID parsed as pipe-delimited with {len(parts)} parts")
        return ssid

    # If it's just a token, assume it's a simple token
    logger.warning("SSID format not recognized, using as-is")
    return ssid


def parse_ssid(ssid: str) -> Tuple[str, str, bool, str, str]:
    """Parse SSID into token, sid, demo, public_code, hidden_code."""
    # Try JSON first
    if ssid.startswith('{') and ssid.endswith('}'):
        try:
            data = json.loads(ssid)
            if isinstance(data, dict):
                token = data.get('token', '')
                sid = data.get('sid', '')
                demo = data.get('demo', False)
                public_code = data.get('public_code', '')
                hidden_code = data.get('hidden_code', '')
                return token, sid, demo, public_code, hidden_code
        except json.JSONDecodeError:
            pass

    # Try pipe-delimited
    parts = ssid.split('|')
    if len(parts) >= 3:
        token = parts[0]
        sid = parts[1]
        demo = parts[2].lower() in ('true', '1', 'yes', 'demo') if len(parts) > 2 else False
        public_code = parts[3] if len(parts) > 3 else ''
        hidden_code = parts[4] if len(parts) > 4 else ''
        return token, sid, demo, public_code, hidden_code

    # Fallback: treat as token only
    return ssid, '', False, '', ''


# This file contains all the async code for the CloseOption Module
class CloseOptionAsync:
    def __init__(self, ssid: str, url: Optional[str] = None, config: Optional[Union[Config, dict, str]] = None, **_):
        """
        Initialize CloseOption async client.

        Args:
            ssid: Session ID in format "token|sid|demo|public_code|hidden_code" or JSON
            url: WebSocket URL (optional, defaults to CloseOption)
            config: Configuration object (optional)
        """
        self._ssid = ssid
        self._url = url
        self._config = config if isinstance(config, Config) else Config(config) if config else Config()
        self._validator = Validator()
        self._client = None
        self._raw_handler = None
        self._connected = False

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.shutdown()

    async def connect(self) -> None:
        """Establish connection to CloseOption."""
        if self._connected:
            return

        from ..BinaryOptionsToolsV2 import RawCloseOption, Logger

        logger = Logger()
        ssid = sanitize_and_validate_ssid(self._ssid, logger)
        token, sid, demo, public_code, hidden_code = parse_ssid(ssid)

        if not token or not sid:
            raise ValueError("SSID must contain token and sid")

        if not public_code or not hidden_code:
            raise ValueError("SSID must contain public_code and hidden_code (format: token|sid|demo|public_code|hidden_code)")

        # Build URL with sid
        if self._url:
            ws_url = self._url
        else:
            ws_url = f"wss://www.closeoption.com:8443/socket.io/?EIO=3&transport=websocket&sid={sid}"

        logger.info("CloseOption: connecting to WebSocket")
        self._client = RawCloseOption(token, sid, public_code, hidden_code, demo, ws_url, self._config.pyconfig)
        logger.info("CloseOption: WebSocket connection established")
        self._connected = True

    async def _ensure_connected(self):
        if not self._connected:
            await self.connect()

    async def buy(self, asset: str, amount: float, time: int) -> dict:
        """Place a BUY (CALL) order."""
        await self._ensure_connected()
        result = await self._client.buy(asset, amount, time)
        return json.loads(result)

    async def sell(self, asset: str, amount: float, time: int) -> dict:
        """Place a SELL (PUT) order."""
        await self._ensure_connected()
        result = await self._client.sell(asset, amount, time)
        return json.loads(result)

    async def check_win(self, order_id: str) -> dict:
        """Check the result of a trade."""
        await self._ensure_connected()
        result = await self._client.check_win(order_id)
        return json.loads(result)

    async def balance(self) -> float:
        """Get current balance."""
        await self._ensure_connected()
        result = await self._client.balance()
        return float(result) if result else 0.0

    async def candles(self, asset: str, period: int) -> List[dict]:
        """Get historical candles."""
        await self._ensure_connected()
        result = await self._client.candles(asset, period)
        return json.loads(result)

    async def get_candles(self, asset: str, period: int, count: int = 100) -> List[dict]:
        """Get historical candles with count."""
        await self._ensure_connected()
        result = await self._client.get_candles(asset, period, count)
        return json.loads(result)
    async def get_ticks(self, asset: str) -> List[dict]:
        """Get tick series for an asset."""
        await self._ensure_connected()
        result = await self._client.get_ticks(asset)
        return json.loads(result)


    async def get_candles_live(self, asset: str, period: int) -> AsyncGenerator[dict, None]:
        """Get live candle updates."""
        await self._ensure_connected()
        async for candle in self._client.get_candles_live(asset, period):
            yield json.loads(candle)

    async def subscribe_symbol(self, symbol: str) -> AsyncSubscription:
        """Subscribe to price updates for a symbol."""
        await self._ensure_connected()
        sub = await self._client.subscribe_symbol(symbol)
        return AsyncSubscription(sub)

    async def subscribe_raw(self) -> AsyncRawSubscription:
        """Subscribe to all raw messages."""
        await self._ensure_connected()
        sub = await self._client.subscribe_raw()
        return AsyncRawSubscription(sub)

    async def send_raw(self, message: str) -> None:
        """Send a raw message."""
        await self._ensure_connected()
        await self._client.send_raw(message)

    async def active_assets(self) -> List[dict]:
        """Get list of active assets."""
        await self._ensure_connected()
        result = await self._client.active_assets()
        return json.loads(result)

    async def payout(self, asset: str) -> float:
        """Get payout for an asset."""
        await self._ensure_connected()
        result = await self._client.payout(asset)
        return float(result)

    async def history(self, limit: int = 100) -> List[dict]:
        """Get trade history."""
        await self._ensure_connected()
        result = await self._client.history(limit)
        return json.loads(result)

    async def opened_deals(self) -> List[dict]:
        """Get opened deals."""
        await self._ensure_connected()
        result = await self._client.opened_deals()
        return json.loads(result)

    async def closed_deals(self) -> List[dict]:
        """Get closed deals."""
        await self._ensure_connected()
        result = await self._client.closed_deals()
        return json.loads(result)

    async def get_server_time(self) -> int:
        """Get server time."""
        await self._ensure_connected()
        result = await self._client.get_server_time()
        return int(result)

    async def raw_handler(self) -> RawHandler:
        """Get raw handler for advanced operations."""
        await self._ensure_connected()
        if self._raw_handler is None:
            handler = await self._client.raw_handler()
            self._raw_handler = RawHandler(handler)
        return self._raw_handler

    async def shutdown(self) -> None:
        """Close the connection and cleanup."""
        if self._raw_handler:
            await self._raw_handler.close()
            self._raw_handler = None
        if self._client:
            await self._client.shutdown()
            self._client = None
        self._connected = False

    async def reconnect(self) -> None:
        """Reconnect to the server."""
        await self._client.reconnect()
