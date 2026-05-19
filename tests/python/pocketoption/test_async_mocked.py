import asyncio
import json
import sys
import types
from datetime import timedelta
from unittest.mock import AsyncMock, MagicMock

import pytest

from BinaryOptionsToolsV2.config import Config
from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync
from BinaryOptionsToolsV2.validator import Validator


class MockRawClient:
    """Mock RawPocketOption client for testing."""

    def __init__(self, *args, **kwargs):
        self._closed = False
        self._connected = True

    async def buy(self, asset, amount, time):
        return "trade_123", json.dumps(
            {"asset": asset, "amount": amount, "time": time, "direction": "buy"}
        )

    async def sell(self, asset, amount, time):
        return "trade_456", json.dumps(
            {"asset": asset, "amount": amount, "time": time, "direction": "sell"}
        )

    async def check_win(self, trade_id):
        if trade_id == "not_found":
            raise Exception("Failed to find deal with ID: not_found")
        return json.dumps({"id": trade_id, "profit": 1.5, "result": "win"})

    async def get_deal_end_time(self, trade_id):
        if trade_id == "invalid":
            return None
        return int(asyncio.get_event_loop().time()) + 60

    async def candles(self, asset, period):
        return json.dumps(
            [
                {"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15},
                {"time": 1060, "open": 1.15, "high": 1.25, "low": 1.1, "close": 1.2},
            ]
        )

    async def get_candles(self, asset, period, offset):
        return json.dumps(
            [{"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]
        )

    async def get_candles_advanced(self, asset, period, offset, time):
        return json.dumps(
            [{"time": time, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]
        )

    async def balance(self):
        return 1000.50

    async def opened_deals(self):
        return json.dumps(
            [{"id": "deal1", "asset": "EURUSD_otc", "amount": 10.0, "status": "open"}]
        )

    async def get_pending_deals(self):
        return json.dumps([])

    async def open_pending_order(
        self,
        open_type,
        amount,
        asset,
        open_time,
        open_price,
        timeframe,
        min_payout,
        command,
    ):
        return json.dumps({"id": "pending_1", "status": "pending"})

    async def cancel_pending_order(self, ticket):
        return json.dumps({"ticket": ticket, "status": "cancelled"})

    async def cancel_pending_orders(self, tickets):
        return json.dumps([{"ticket": ticket, "status": "cancelled"} for ticket in tickets])

    async def closed_deals(self):
        return json.dumps(
            [
                {
                    "id": "deal2",
                    "asset": "GBPUSD_otc",
                    "amount": 5.0,
                    "profit": -1.0,
                    "result": "loss",
                }
            ]
        )

    async def clear_closed_deals(self):
        pass

    async def payout(self):
        return json.dumps({"EURUSD_otc": 85, "GBPUSD_otc": 82, "BTCUSD_otc": 78})

    async def active_assets(self):
        return json.dumps(
            [
                {
                    "id": 1,
                    "symbol": "EURUSD_otc",
                    "name": "EUR/USD OTC",
                    "asset_type": "currency",
                    "payout": 85,
                    "is_otc": True,
                    "is_active": True,
                    "allowed_candles": [1, 5, 15, 30, 60],
                }
            ]
        )

    async def history(self, asset, period):
        return json.dumps(
            [{"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]
        )

    async def subscribe_symbol(self, asset):
        async def subscription():
            yield json.dumps({"symbol": asset, "price": 1.11})

        return subscription()

    async def subscribe_symbol_chuncked(self, asset, chunk_size):
        async def subscription():
            yield json.dumps({"chunk": 1, "open": 1.1, "close": 1.2})

        return subscription()

    async def subscribe_symbol_timed(self, asset, time):
        async def subscription():
            yield json.dumps({"time": 1000, "price": 1.11})

        return subscription()

    async def subscribe_symbol_time_aligned(self, asset, time):
        async def subscription():
            yield json.dumps({"aligned_time": 1000, "price": 1.11})

        return subscription()

    async def get_server_time(self):
        return 1700000000

    async def wait_for_assets(self, timeout):
        pass

    def is_demo(self):
        return True

    async def disconnect(self):
        self._connected = False

    async def connect(self):
        self._connected = True

    async def reconnect(self):
        self._connected = True

    async def unsubscribe(self, asset):
        pass

    async def shutdown(self):
        self._closed = True

    async def create_raw_handler(self, validator, keep_alive=None):
        mock_handler = MagicMock()
        mock_handler.id.return_value = "handler_123"
        mock_handler.send_text = AsyncMock()
        mock_handler.send_binary = AsyncMock()
        mock_handler.send_and_wait = AsyncMock(return_value='42["response"]')
        mock_handler.wait_next = AsyncMock(return_value='42["message"]')

        async def mock_subscribe():
            yield '42["stream_data"]'

        mock_handler.subscribe = MagicMock(return_value=mock_subscribe())
        mock_handler.close = AsyncMock()
        return mock_handler

    async def send_raw_message(self, message):
        pass

    async def create_raw_order(self, message, validator):
        return '42["response"]'

    async def create_raw_order_with_timeout(self, message, validator, timeout):
        return '42["response"]'

    async def create_raw_order_with_timeout_and_retry(
        self, message, validator, timeout
    ):
        return '42["response"]'

    async def create_raw_iterator(self, message, validator, timeout=None):
        async def iterator():
            yield '42["event1"]'
            yield '42["event2"]'

        return iterator()


class MockRawPocketOption:
    """Mock class to replace RawPocketOption."""

    @classmethod
    def new_with_config(cls, *args, **kwargs):
        return MockRawClient()


class MockPyConfig:
    """Mock PyConfig class."""

    def __init__(self, *args, **kwargs):
        self._config = kwargs
        self.urls = kwargs.get("urls", ["wss://api.pocketoption.com"])
        self.terminal_logging = kwargs.get("terminal_logging", True)
        self.log_level = kwargs.get("log_level", "INFO")
        self.websocket_config = kwargs.get("websocket_config", {})
        self.keepalive_config = kwargs.get("keepalive_config", {})
        self.validator = kwargs.get("validator", None)
        self.retries = kwargs.get("retries", 3)
        self.timeout = kwargs.get("timeout", 30)
        self.connection_timeout = kwargs.get("connection_timeout", 30)
        self.ping_interval = kwargs.get("ping_interval", 5)
        self.ping_timeout = kwargs.get("ping_timeout", 5)
        self.close_timeout = kwargs.get("close_timeout", 5)
        self.max_size = kwargs.get("max_size", 2**20)
        self.max_queue = kwargs.get("max_queue", 2**10)
        self.read_limit = kwargs.get("read_limit", 2**16)
        self.write_limit = kwargs.get("write_limit", 2**16)

    def __call__(self):
        return self


class MockRawValidator:
    """Mock RawValidator class."""

    def __init__(self, condition=None):
        self.condition = condition

    def __call__(self, message):
        return True

    def __repr__(self):
        return f"MockRawValidator(condition={self.condition})"

    @classmethod
    def starts_with(cls, prefix):
        """Mock starts_with validator factory."""
        return cls(condition=f"starts_with:{prefix}")

    @classmethod
    def contains(cls, substring):
        """Mock contains validator factory."""
        return cls(condition=f"contains:{substring}")

    @classmethod
    def regex(cls, pattern):
        """Mock regex validator factory."""
        return cls(condition=f"regex:{pattern}")

    @classmethod
    def custom(cls, func):
        """Mock custom validator factory."""
        if not callable(func):
            raise TypeError("func must be callable")
        return cls(condition=f"custom:{func}")


class MockLogger:
    """Mock Logger class."""

    def __init__(self):
        pass

    def info(self, *args, **kwargs):
        pass

    def error(self, *args, **kwargs):
        pass

    def debug(self, *args, **kwargs):
        pass

    def warn(self, *args, **kwargs):
        pass


@pytest.fixture(autouse=True)
def mock_raw_pocketoption(monkeypatch):
    """Autouse fixture that replaces RawPocketOption and other dependencies with mock classes."""
    # Ensure BinaryOptionsToolsV2 module exists
    try:
        import BinaryOptionsToolsV2
    except ImportError:
        BinaryOptionsToolsV2 = types.ModuleType("BinaryOptionsToolsV2")
        sys.modules["BinaryOptionsToolsV2"] = BinaryOptionsToolsV2
    # Set our mock class as RawPocketOption at top-level
    monkeypatch.setattr(
        BinaryOptionsToolsV2, "RawPocketOption", MockRawPocketOption, raising=False
    )
    # Also patch RawPocketOption in the asynchronous module (where PocketOptionAsync uses it)
    try:
        import BinaryOptionsToolsV2.pocketoption.asynchronous as async_mod

        monkeypatch.setattr(
            async_mod, "RawPocketOption", MockRawPocketOption, raising=False
        )
        # Also patch Logger, PyConfig, RawValidator in asynchronous module if they exist
        if hasattr(async_mod, "Logger"):
            monkeypatch.setattr(async_mod, "Logger", MockLogger, raising=False)
        if hasattr(async_mod, "PyConfig"):
            monkeypatch.setattr(async_mod, "PyConfig", MockPyConfig, raising=False)
        if hasattr(async_mod, "RawValidator"):
            monkeypatch.setattr(
                async_mod, "RawValidator", MockRawValidator, raising=False
            )
    except ImportError:
        pass
    # Mock Logger if not present at top-level
    if not hasattr(BinaryOptionsToolsV2, "Logger"):
        monkeypatch.setattr(BinaryOptionsToolsV2, "Logger", MockLogger, raising=False)
    # Mock PyConfig if not present at top-level
    if not hasattr(BinaryOptionsToolsV2, "PyConfig"):
        monkeypatch.setattr(
            BinaryOptionsToolsV2, "PyConfig", MockPyConfig, raising=False
        )
    # Mock RawValidator if not present at top-level
    if not hasattr(BinaryOptionsToolsV2, "RawValidator"):
        monkeypatch.setattr(
            BinaryOptionsToolsV2, "RawValidator", MockRawValidator, raising=False
        )
    # Also patch the Rust submodule's RawPocketOption (the actual class imported by asynchronous.__init__)
    try:
        import BinaryOptionsToolsV2.BinaryOptionsToolsV2 as rust_submodule

        monkeypatch.setattr(
            rust_submodule, "RawPocketOption", MockRawPocketOption, raising=False
        )
        # Also patch RawValidator in the Rust submodule for Validator.custom()
        if hasattr(rust_submodule, "RawValidator"):
            monkeypatch.setattr(
                rust_submodule, "RawValidator", MockRawValidator, raising=False
            )
    except ImportError:
        pass
    # Create and return a mock instance for tests to customize
    instance = MockRawClient()
    # Make the mock class's new_with_config return this specific instance
    MockRawPocketOption.new_with_config = lambda *args, **kwargs: instance
    return instance


@pytest.fixture
async def async_client(mock_raw_pocketoption):
    """Fixture that creates a PocketOptionAsync client with mocked backend."""
    client = PocketOptionAsync("test_ssid", config={"terminal_logging": False})
    yield client
    await client.shutdown()


class TestPocketOptionAsyncInit:
    """Tests for PocketOptionAsync initialization."""

    def test_init_with_ssid_only(self, mock_raw_pocketoption):
        """Test initialization with just SSID."""
        client = PocketOptionAsync("test_ssid")
        assert client.client is mock_raw_pocketoption
        asyncio.get_event_loop().run_until_complete(client.shutdown())

    def test_init_with_config_dict(self, mock_raw_pocketoption):
        """Test initialization with config dict."""
        config = {"terminal_logging": False, "log_level": "INFO"}
        client = PocketOptionAsync("test_ssid", config=config)
        assert client.config.terminal_logging is False
        asyncio.get_event_loop().run_until_complete(client.shutdown())

    def test_init_with_config_json(self, mock_raw_pocketoption):
        """Test initialization with config JSON string."""
        config_json = '{"terminal_logging": false, "log_level": "DEBUG"}'
        client = PocketOptionAsync("test_ssid", config=config_json)
        assert client.config.terminal_logging is False
        asyncio.get_event_loop().run_until_complete(client.shutdown())

    def test_init_with_config_object(self, mock_raw_pocketoption):
        """Test initialization with Config object."""
        cfg = Config()
        cfg.terminal_logging = False
        client = PocketOptionAsync("test_ssid", config=cfg)
        assert client.config.terminal_logging is False
        asyncio.get_event_loop().run_until_complete(client.shutdown())

    def test_init_with_invalid_config_type(self):
        """Test initialization with invalid config type raises ValueError."""
        with pytest.raises(ValueError, match="Config type mismatch"):
            PocketOptionAsync("test_ssid", config=123)

    def test_init_with_custom_url(self, mock_raw_pocketoption):
        """Test that custom URL is added to config."""
        client = PocketOptionAsync("test_ssid", url="wss://custom.com")
        assert "wss://custom.com" in client.config.urls
        asyncio.get_event_loop().run_until_complete(client.shutdown())


class TestBuyAndSell:
    """Tests for buy and sell methods."""

    @pytest.mark.asyncio
    async def test_buy_success(self, async_client):
        """Test successful buy operation."""
        trade_id, trade = await async_client.buy("EURUSD_otc", 1.0, 60)
        assert trade_id == "trade_123"
        assert trade["asset"] == "EURUSD_otc"
        assert trade["direction"] == "buy"

    @pytest.mark.asyncio
    async def test_buy_with_check_win(self, async_client):
        """Test buy with check_win=True."""
        trade_id, trade = await async_client.buy("EURUSD_otc", 1.0, 60, check_win=True)
        assert trade["result"] == "win"
        assert trade["profit"] == 1.5

    @pytest.mark.asyncio
    async def test_sell_success(self, async_client):
        """Test successful sell operation."""
        trade_id, trade = await async_client.sell("EURUSD_otc", 1.0, 60)
        assert trade_id == "trade_456"
        assert trade["direction"] == "sell"

    @pytest.mark.asyncio
    async def test_sell_with_check_win(self, async_client):
        """Test sell with check_win=True."""
        trade_id, trade = await async_client.sell("EURUSD_otc", 1.0, 60, check_win=True)
        assert trade["result"] == "win"
        assert trade["profit"] == 1.5

    @pytest.mark.asyncio
    async def test_buy_client_error(self, async_client, mock_raw_pocketoption):
        """Test buy when client raises exception."""
        mock_raw_pocketoption.buy = AsyncMock(side_effect=Exception("Connection lost"))
        with pytest.raises(Exception, match="Connection lost"):
            await async_client.buy("EURUSD_otc", 1.0, 60)


class TestCheckWin:
    """Tests for check_win method."""

    @pytest.mark.asyncio
    async def test_check_win_success(self, async_client):
        """Test check_win with valid trade ID."""
        result = await async_client.check_win("trade_123")
        assert result["id"] == "trade_123"
        assert result["result"] == "win"
        assert result["profit"] == 1.5

    @pytest.mark.asyncio
    async def test_check_win_invalid_id(self, async_client):
        """Test check_win with invalid trade ID."""
        with pytest.raises(Exception):
            await async_client.check_win("not_found")

    @pytest.mark.asyncio
    async def test_check_win_timeout(self, async_client, mock_raw_pocketoption):
        """Test check_win timeout protection."""
        mock_raw_pocketoption.check_win = AsyncMock(side_effect=asyncio.TimeoutError)
        with pytest.raises(Exception):
            await async_client.check_win("trade_123")


class TestGetDealEndTime:
    """Tests for get_deal_end_time method."""

    @pytest.mark.asyncio
    async def test_get_deal_end_time_success(self, async_client):
        """Test getting deal end time."""
        end_time = await async_client.get_deal_end_time("trade_123")
        assert end_time is not None
        assert isinstance(end_time, int)

    @pytest.mark.asyncio
    async def test_get_deal_end_time_not_found(self, async_client):
        """Test get_deal_end_time with invalid ID returns None."""
        end_time = await async_client.get_deal_end_time("invalid")
        assert end_time is None


class TestCandles:
    """Tests for candles and get_candles methods."""

    @pytest.mark.asyncio
    async def test_candles_success(self, async_client):
        """Test candles retrieval."""
        candles = await async_client.candles("EURUSD_otc", 60)
        assert isinstance(candles, list)
        assert len(candles) > 0
        assert "open" in candles[0]
        assert "close" in candles[0]

    @pytest.mark.asyncio
    async def test_get_candles_success(self, async_client):
        """Test get_candles with offset."""
        candles = await async_client.get_candles("EURUSD_otc", 60, 10)
        assert isinstance(candles, list)
        assert len(candles) > 0

    @pytest.mark.asyncio
    async def test_get_candles_advanced_success(self, async_client):
        """Test get_candles_advanced with time parameter."""
        candles = await async_client.get_candles_advanced(
            "EURUSD_otc", 60, 10, 1700000000
        )
        assert isinstance(candles, list)
        assert len(candles) > 0

    @pytest.mark.asyncio
    async def test_compile_candles_success(self, async_client, mock_raw_pocketoption):
        """Test compile_candles with custom periods."""
        # Setup mock to return expected compiled candles shape
        mock_raw_pocketoption.compile_candles = AsyncMock(
            return_value=json.dumps(
                [{"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]
            )
        )
        candles = await async_client.compile_candles("EURUSD_otc", 20, 300)
        assert isinstance(candles, list)
        assert len(candles) == 1
        assert "open" in candles[0]
        assert "time" in candles[0]
        mock_raw_pocketoption.compile_candles.assert_called_with("EURUSD_otc", 20, 300)

    @pytest.mark.asyncio
    async def test_compile_candles_validation_error(self, async_client):
        """Test compile_candles validation for non-positive periods."""
        with pytest.raises(
            ValueError, match="custom_period must be a positive integer"
        ):
            await async_client.compile_candles("EURUSD_otc", 0, 300)
        with pytest.raises(
            ValueError, match="lookback_period must be a positive integer"
        ):
            await async_client.compile_candles("EURUSD_otc", 20, -1)


class TestBalance:
    """Tests for balance method."""

    @pytest.mark.asyncio
    async def test_balance_success(self, async_client):
        """Test balance retrieval."""
        balance = await async_client.balance()
        assert isinstance(balance, float)
        assert balance >= 0


class TestOpenedDeals:
    """Tests for opened_deals method."""

    @pytest.mark.asyncio
    async def test_opened_deals_success(self, async_client):
        """Test opened_deals retrieval."""
        deals = await async_client.opened_deals()
        assert isinstance(deals, list)
        if deals:
            assert "id" in deals[0]

    @pytest.mark.asyncio
    async def test_opened_deals_empty(self, async_client, mock_raw_pocketoption):
        """Test opened_deals when no open deals."""
        mock_raw_pocketoption.opened_deals = AsyncMock(return_value=json.dumps([]))
        deals = await async_client.opened_deals()
        assert deals == []


class TestGetPendingDeals:
    """Tests for get_pending_deals method."""

    @pytest.mark.asyncio
    async def test_get_pending_deals_success(self, async_client):
        """Test get_pending_deals retrieval."""
        pending = await async_client.get_pending_deals()
        assert isinstance(pending, list)


class TestOpenPendingOrder:
    """Tests for open_pending_order method."""

    @pytest.mark.asyncio
    async def test_open_pending_order_success(self, async_client):
        """Test successful pending order creation."""
        order = await async_client.open_pending_order(
            open_type=0,
            amount=10.0,
            asset="EURUSD_otc",
            open_time=1700000000,
            open_price=1.1,
            timeframe=60,
            min_payout=80,
            command=0,
        )
        assert isinstance(order, dict)
        assert "id" in order

    @pytest.mark.asyncio
    async def test_open_pending_order_invalid_params(
        self, async_client, mock_raw_pocketoption
    ):
        """Test open_pending_order with invalid parameters."""
        mock_raw_pocketoption.open_pending_order = AsyncMock(
            side_effect=ValueError("Invalid amount")
        )
        with pytest.raises(ValueError, match="Invalid amount"):
            await async_client.open_pending_order(
                0, -1.0, "EURUSD_otc", 1700000000, 1.1, 60, 80, 0
            )


class TestCancelPendingOrder:
    """Tests for pending order cancellation methods."""

    @pytest.mark.asyncio
    async def test_cancel_pending_order_success(self, async_client):
        result = await async_client.cancel_pending_order(
            "11111111-1111-1111-1111-111111111111"
        )
        assert result["status"] == "cancelled"

    @pytest.mark.asyncio
    async def test_cancel_pending_orders_success(self, async_client):
        results = await async_client.cancel_pending_orders(
            [
                "11111111-1111-1111-1111-111111111111",
                "22222222-2222-2222-2222-222222222222",
            ]
        )
        assert len(results) == 2
        assert all(result["status"] == "cancelled" for result in results)


class TestClosedDeals:
    """Tests for closed_deals method."""

    @pytest.mark.asyncio
    async def test_closed_deals_success(self, async_client):
        """Test closed_deals retrieval."""
        deals = await async_client.closed_deals()
        assert isinstance(deals, list)
        if deals:
            assert "result" in deals[0] or "profit" in deals[0]

    @pytest.mark.asyncio
    async def test_closed_deals_empty(self, async_client, mock_raw_pocketoption):
        """Test closed_deals when no closed deals."""
        mock_raw_pocketoption.closed_deals = AsyncMock(return_value=json.dumps([]))
        deals = await async_client.closed_deals()
        assert deals == []


class TestClearClosedDeals:
    """Tests for clear_closed_deals method."""

    @pytest.mark.asyncio
    async def test_clear_closed_deals_success(self, async_client):
        """Test clearing closed deals."""
        await async_client.clear_closed_deals()

    @pytest.mark.asyncio
    async def test_clear_closed_deals_error(self, async_client, mock_raw_pocketoption):
        """Test clear_closed_deals when operation fails."""
        mock_raw_pocketoption.clear_closed_deals = AsyncMock(
            side_effect=Exception("Clear failed")
        )
        with pytest.raises(Exception, match="Clear failed"):
            await async_client.clear_closed_deals()


class TestPayout:
    """Tests for payout method."""

    @pytest.mark.asyncio
    async def test_payout_all(self, async_client):
        """Test payout with no asset parameter (all assets)."""
        payouts = await async_client.payout()
        assert isinstance(payouts, dict)
        assert "EURUSD_otc" in payouts

    @pytest.mark.asyncio
    async def test_payout_single_asset(self, async_client):
        """Test payout with single asset string."""
        payout = await async_client.payout("EURUSD_otc")
        assert isinstance(payout, int)
        assert payout == 85

    @pytest.mark.asyncio
    async def test_payout_list_of_assets(self, async_client):
        """Test payout with list of assets."""
        payouts = await async_client.payout(["EURUSD_otc", "GBPUSD_otc"])
        assert isinstance(payouts, list)
        assert len(payouts) == 2
        assert payouts[0] == 85

    @pytest.mark.asyncio
    async def test_payout_invalid_asset(self, async_client):
        """Test payout with invalid asset returns None."""
        payout = await async_client.payout("INVALID_ASSET")
        assert payout is None

    @pytest.mark.asyncio
    async def test_payout_empty_list(self, async_client):
        """Test payout with empty list."""
        payouts = await async_client.payout([])
        assert payouts == []


class TestActiveAssets:
    """Tests for active_assets method."""

    @pytest.mark.asyncio
    async def test_active_assets_success(self, async_client):
        """Test active_assets retrieval."""
        assets = await async_client.active_assets()
        assert isinstance(assets, list)
        if assets:
            assert "symbol" in assets[0]
            assert "payout" in assets[0]

    @pytest.mark.asyncio
    async def test_active_assets_empty(self, async_client, mock_raw_pocketoption):
        """Test active_assets when no assets available."""
        mock_raw_pocketoption.active_assets = AsyncMock(return_value=json.dumps([]))
        assets = await async_client.active_assets()
        assert assets == []


class TestHistory:
    """Tests for history method."""

    @pytest.mark.asyncio
    async def test_history_success(self, async_client):
        """Test history retrieval."""
        candles = await async_client.history("EURUSD_otc", 60)
        assert isinstance(candles, list)
        assert len(candles) > 0
        assert "time" in candles[0]

    @pytest.mark.asyncio
    async def test_history_empty(self, async_client, mock_raw_pocketoption):
        """Test history when no data available."""
        mock_raw_pocketoption.history = AsyncMock(return_value=json.dumps([]))
        candles = await async_client.history("EURUSD_otc", 60)
        assert candles == []


class TestSubscriptions:
    """Tests for subscription methods."""

    @pytest.mark.asyncio
    async def test_subscribe_symbol_success(self, async_client):
        """Test subscribe_symbol creates valid subscription."""
        sub = await async_client.subscribe_symbol("EURUSD_otc")
        assert sub is not None
        assert hasattr(sub, "__aiter__")

    @pytest.mark.asyncio
    async def test_subscribe_symbol_chuncked_success(self, async_client):
        """Test subscribe_symbol_chuncked with valid chunk size."""
        sub = await async_client.subscribe_symbol_chuncked("EURUSD_otc", 10)
        assert sub is not None
        assert hasattr(sub, "__aiter__")

    @pytest.mark.asyncio
    async def test_subscribe_symbol_chuncked_invalid_chunk(self, async_client):
        """Test subscribe_symbol_chuncked with invalid chunk size."""
        sub = await async_client.subscribe_symbol_chuncked("EURUSD_otc", 0)
        assert sub is not None

    @pytest.mark.asyncio
    async def test_subscribe_symbol_timed_success(self, async_client):
        """Test subscribe_symbol_timed with timedelta."""
        sub = await async_client.subscribe_symbol_timed(
            "EURUSD_otc", timedelta(seconds=5)
        )
        assert sub is not None
        assert hasattr(sub, "__aiter__")

    @pytest.mark.asyncio
    async def test_subscribe_symbol_time_aligned_success(self, async_client):
        """Test subscribe_symbol_time_aligned with timedelta."""
        sub = await async_client.subscribe_symbol_time_aligned(
            "EURUSD_otc", timedelta(seconds=60)
        )
        assert sub is not None
        assert hasattr(sub, "__aiter__")


class TestGetServerTime:
    """Tests for get_server_time method."""

    @pytest.mark.asyncio
    async def test_get_server_time_success(self, async_client):
        """Test server time retrieval."""
        time = await async_client.get_server_time()
        assert isinstance(time, int)
        assert time > 0

    @pytest.mark.asyncio
    async def test_get_server_time_error(self, async_client, mock_raw_pocketoption):
        """Test get_server_time when client fails."""
        mock_raw_pocketoption.get_server_time = AsyncMock(
            side_effect=Exception("Connection error")
        )
        with pytest.raises(Exception, match="Connection error"):
            await async_client.get_server_time()


class TestWaitForAssets:
    """Tests for wait_for_assets method."""

    @pytest.mark.asyncio
    async def test_wait_for_assets_success(self, async_client):
        """Test wait_for_assets completes quickly."""
        await async_client.wait_for_assets(timeout=1.0)

    @pytest.mark.asyncio
    async def test_wait_for_assets_timeout(self, async_client, mock_raw_pocketoption):
        """Test wait_for_assets timeout."""
        mock_raw_pocketoption.wait_for_assets = AsyncMock(
            side_effect=asyncio.TimeoutError
        )
        with pytest.raises(Exception):
            await async_client.wait_for_assets(timeout=1.0)


class TestIsDemo:
    """Tests for is_demo method."""

    def test_is_demo_success(self, async_client):
        """Test is_demo returns boolean."""
        result = async_client.is_demo()
        assert isinstance(result, bool)


class TestConnectionMethods:
    """Tests for disconnect, connect, reconnect methods."""

    @pytest.mark.asyncio
    async def test_disconnect_success(self, async_client):
        """Test disconnect."""
        await async_client.disconnect()
        assert async_client.client._connected is False

    @pytest.mark.asyncio
    async def test_connect_success(self, async_client):
        """Test connect after disconnect."""
        await async_client.disconnect()
        await async_client.connect()
        assert async_client.client._connected is True

    @pytest.mark.asyncio
    async def test_reconnect_success(self, async_client):
        """Test reconnect."""
        await async_client.reconnect()
        assert async_client.client._connected is True


class TestUnsubscribe:
    """Tests for unsubscribe method."""

    @pytest.mark.asyncio
    async def test_unsubscribe_success(self, async_client):
        """Test unsubscribe from asset."""
        await async_client.unsubscribe("EURUSD_otc")


class TestShutdown:
    """Tests for shutdown method."""

    @pytest.mark.asyncio
    async def test_shutdown_success(self, async_client):
        """Test shutdown."""
        await async_client.shutdown()
        assert async_client.client._closed is True


class TestCreateRawHandler:
    """Tests for create_raw_handler method."""

    @pytest.mark.asyncio
    async def test_create_raw_handler_success(self, async_client):
        """Test creating raw handler."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        assert handler is not None
        assert handler.id() is not None

    @pytest.mark.asyncio
    async def test_raw_handler_send_text(self, async_client):
        """Test raw handler send_text."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        await handler.send_text('42["ping"]')

    @pytest.mark.asyncio
    async def test_raw_handler_send_binary(self, async_client):
        """Test raw handler send_binary."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        await handler.send_binary(b"\x00\x01")

    @pytest.mark.asyncio
    async def test_raw_handler_send_and_wait(self, async_client):
        """Test raw handler send_and_wait."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        response = await handler.send_and_wait('42["getServerTime"]')
        assert isinstance(response, str)

    @pytest.mark.asyncio
    async def test_raw_handler_wait_next(self, async_client):
        """Test raw handler wait_next."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        message = await handler.wait_next()
        assert isinstance(message, str)

    @pytest.mark.asyncio
    async def test_raw_handler_subscribe(self, async_client):
        """Test raw handler subscribe."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        stream = await handler.subscribe()
        assert stream is not None

    @pytest.mark.asyncio
    async def test_raw_handler_close(self, async_client):
        """Test raw handler close."""
        validator = Validator.starts_with('42["test"')
        handler = await async_client.create_raw_handler(validator)
        await handler.close()


class TestSendRawMessage:
    """Tests for send_raw_message method."""

    @pytest.mark.asyncio
    async def test_send_raw_message_success(self, async_client):
        """Test sending raw message."""
        await async_client.send_raw_message('42["ping"]')

    @pytest.mark.asyncio
    async def test_send_raw_message_error(self, async_client, mock_raw_pocketoption):
        """Test send_raw_message when client fails."""
        mock_raw_pocketoption.send_raw_message = AsyncMock(
            side_effect=Exception("Send failed")
        )
        with pytest.raises(Exception, match="Send failed"):
            await async_client.send_raw_message('42["ping"]')


class TestCreateRawOrder:
    """Tests for create_raw_order and variants."""

    @pytest.mark.asyncio
    async def test_create_raw_order_success(self, async_client):
        """Test create_raw_order with validator."""
        validator = Validator.contains("response")
        response = await async_client.create_raw_order('42["test"]', validator)
        assert isinstance(response, str)

    @pytest.mark.asyncio
    async def test_create_raw_order_timeout(self, async_client):
        """Test create_raw_order with timeout."""
        validator = Validator.contains("response")
        timeout = timedelta(seconds=5)
        response = await async_client.create_raw_order_with_timeout(
            '42["test"]', validator, timeout
        )
        assert isinstance(response, str)

    @pytest.mark.asyncio
    async def test_create_raw_order_with_timeout_success(self, async_client):
        """Test create_raw_order_with_timeout."""
        validator = Validator.contains("response")
        timeout = timedelta(seconds=5)
        response = await async_client.create_raw_order_with_timeout(
            '42["test"]', validator, timeout
        )
        assert isinstance(response, str)

    @pytest.mark.asyncio
    async def test_create_raw_order_with_timeout_and_retry_success(self, async_client):
        """Test create_raw_order_with_timeout_and_retry."""
        validator = Validator.contains("response")
        timeout = timedelta(seconds=5)
        response = await async_client.create_raw_order_with_timeout_and_retry(
            '42["test"]', validator, timeout
        )
        assert isinstance(response, str)

    @pytest.mark.asyncio
    async def test_create_raw_iterator_success(self, async_client):
        """Test create_raw_iterator returns async iterator."""
        validator = Validator.contains("event")
        iterator = await async_client.create_raw_iterator('42["subscribe"]', validator)
        assert iterator is not None
        assert hasattr(iterator, "__aiter__")

    @pytest.mark.asyncio
    async def test_create_raw_iterator_with_timeout(self, async_client):
        """Test create_raw_iterator with timeout."""
        validator = Validator.contains("event")
        timeout = timedelta(seconds=30)
        iterator = await async_client.create_raw_iterator(
            '42["subscribe"]', validator, timeout
        )
        assert iterator is not None


class TestContextManager:
    """Tests for async context manager."""

    @pytest.mark.asyncio
    async def test_async_context_manager(self):
        """Test async context manager enter and exit."""
        async with PocketOptionAsync("test_ssid") as client:
            assert client.client is not None


class TestValidator:
    """Tests for Validator class."""

    def test_validator_custom_with_invalid_function(self):
        """Test Validator.custom with non-callable raises error."""
        with pytest.raises(TypeError):
            Validator.custom("not a function")

    def test_validator_custom_with_callable(self):
        """Test Validator.custom with callable works."""

        def my_validator(msg):
            return True

        validator = Validator.custom(my_validator)
        assert validator.raw_validator is not None


class TestConcurrentOperations:
    """Tests for concurrent operations."""

    @pytest.mark.asyncio
    async def test_concurrent_multiple_calls(self, async_client):
        """Test that multiple async calls can run concurrently."""
        # Run balance, active_assets, and history concurrently
        results = await asyncio.gather(
            async_client.balance(),
            async_client.active_assets(),
            async_client.history("EURUSD_otc", 60),
        )
        balance, assets, candles = results
        assert isinstance(balance, float)
        assert isinstance(assets, list)
        assert isinstance(candles, list)
