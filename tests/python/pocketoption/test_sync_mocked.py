import asyncio
import sys
import threading
import types
from datetime import timedelta
from unittest.mock import AsyncMock, MagicMock

import pytest

from BinaryOptionsToolsV2.config import Config
from BinaryOptionsToolsV2.pocketoption.synchronous import PocketOption
from BinaryOptionsToolsV2.validator import Validator


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


class MockPocketOptionAsync:
    """Mock PocketOptionAsync client for testing."""

    _shared_state = {}

    def __init__(self, *args, **kwargs):
        self.__dict__ = self._shared_state
        # Initialize defaults if not present
        if "_closed" not in self.__dict__:
            self._closed = False
        if "_connected" not in self.__dict__:
            self._connected = True
        # Process config
        config = kwargs.get("config")
        url = kwargs.get("url")
        if config is None:
            self.config = Config()
        elif isinstance(config, dict):
            self.config = Config.from_dict(config)
        elif isinstance(config, str):
            self.config = Config.from_json(config)
        elif isinstance(config, Config):
            self.config = config
        else:
            raise ValueError("Config type mismatch")
        # Handle url insertion
        if url is not None:
            if not hasattr(self.config, "urls") or self.config.urls is None:
                self.config.urls = []
            self.config.urls.insert(0, url)

    async def buy(self, asset, amount, time, check_win=False):
        trade_id, trade = (
            "trade_123",
            {"asset": asset, "amount": amount, "time": time, "direction": "buy"},
        )
        if check_win:
            trade["result"] = "win"
            trade["profit"] = 1.5
        return trade_id, trade

    async def sell(self, asset, amount, time, check_win=False):
        trade_id, trade = (
            "trade_456",
            {"asset": asset, "amount": amount, "time": time, "direction": "sell"},
        )
        if check_win:
            trade["result"] = "win"
            trade["profit"] = 1.5
        return trade_id, trade

    async def check_win(self, trade_id):
        if trade_id == "not_found":
            raise Exception("Failed to find deal with ID: not_found")
        return {"id": trade_id, "profit": 1.5, "result": "win"}

    async def get_deal_end_time(self, trade_id):
        if trade_id == "invalid":
            return None
        return int(asyncio.get_event_loop().time()) + 60

    async def candles(self, asset, period):
        return [
            {"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15},
            {"time": 1060, "open": 1.15, "high": 1.25, "low": 1.1, "close": 1.2},
        ]

    async def get_candles(self, asset, period, offset):
        return [{"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]

    async def get_candles_advanced(self, asset, period, offset, time):
        return [{"time": time, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]

    async def balance(self):
        return 1000.50

    async def opened_deals(self):
        return [
            {"id": "deal1", "asset": "EURUSD_otc", "amount": 10.0, "status": "open"}
        ]

    async def get_pending_deals(self):
        return []

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
        return {"id": "pending_1", "status": "pending"}

    async def cancel_pending_order(self, ticket):
        return {"ticket": ticket, "status": "cancelled"}

    async def cancel_pending_orders(self, tickets):
        return [{"ticket": ticket, "status": "cancelled"} for ticket in tickets]

    async def closed_deals(self):
        return [
            {
                "id": "deal2",
                "asset": "GBPUSD_otc",
                "amount": 5.0,
                "profit": -1.0,
                "result": "loss",
            }
        ]

    async def clear_closed_deals(self):
        pass

    async def payout(self, asset=None):
        if asset is None:
            return {"EURUSD_otc": 85, "GBPUSD_otc": 82, "BTCUSD_otc": 78}
        elif isinstance(asset, str):
            return 85 if asset == "EURUSD_otc" else None
        elif isinstance(asset, list):
            return [85 if a == "EURUSD_otc" else 82 for a in asset]
        return None

    async def active_assets(self):
        return [
            {
                "symbol": "EURUSD_otc",
                "name": "EUR/USD OTC",
                "asset_type": "currency",
                "payout": 85,
                "is_otc": True,
                "is_active": True,
                "allowed_candles": [1, 5, 15, 30, 60],
            }
        ]

    async def history(self, asset, period):
        return [{"time": 1000, "open": 1.1, "high": 1.2, "low": 1.0, "close": 1.15}]

    async def subscribe_symbol(self, asset):
        async def subscription():
            yield {"symbol": asset, "price": 1.11}

        return subscription()

    async def subscribe_symbol_chuncked(self, asset, chunk_size):
        async def subscription():
            yield {"chunk": 1, "open": 1.1, "close": 1.2}

        return subscription()

    async def subscribe_symbol_timed(self, asset, time):
        async def subscription():
            yield {"time": 1000, "price": 1.11}

        return subscription()

    async def subscribe_symbol_time_aligned(self, asset, time):
        async def subscription():
            yield {"aligned_time": 1000, "price": 1.11}

        return subscription()

    async def _subscribe_symbol_inner(self, asset: str):
        return await self.subscribe_symbol(asset)

    async def _subscribe_symbol_chuncked_inner(self, asset: str, chunk_size: int):
        return await self.subscribe_symbol_chuncked(asset, chunk_size)

    async def _subscribe_symbol_timed_inner(self, asset: str, time):
        return await self.subscribe_symbol_timed(asset, time)

    async def _subscribe_symbol_time_aligned_inner(self, asset: str, time):
        return await self.subscribe_symbol_time_aligned(asset, time)

    async def get_server_time(self):
        return 1700000000

    async def wait_for_assets(self, timeout=60.0):
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
        mock_handler.subscribe = AsyncMock(return_value=AsyncMock())
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

    @classmethod
    def new_with_config(cls, *args, **kwargs):
        return MockPocketOptionAsync(*args, **kwargs)


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


@pytest.fixture(autouse=True)
def mock_pocketoption_async(monkeypatch):
    """Autouse fixture that replaces PocketOptionAsync and dependencies with mock classes."""
    # Ensure BinaryOptionsToolsV2 module exists
    try:
        import BinaryOptionsToolsV2
    except ImportError:
        BinaryOptionsToolsV2 = types.ModuleType("BinaryOptionsToolsV2")
        sys.modules["BinaryOptionsToolsV2"] = BinaryOptionsToolsV2
    # Reset shared state for isolation
    MockPocketOptionAsync._shared_state = {}
    # Patch PocketOptionAsync in asynchronous module
    monkeypatch.setattr(
        BinaryOptionsToolsV2.pocketoption.asynchronous,
        "PocketOptionAsync",
        MockPocketOptionAsync,
        raising=False,
    )
    # Also patch PocketOptionAsync in synchronous module if it exists
    try:
        import BinaryOptionsToolsV2.pocketoption.synchronous as sync_mod

        monkeypatch.setattr(
            sync_mod, "PocketOptionAsync", MockPocketOptionAsync, raising=False
        )
    except ImportError:
        pass
    # Mock Logger if not present at top-level
    if not hasattr(BinaryOptionsToolsV2, "Logger"):
        monkeypatch.setattr(BinaryOptionsToolsV2, "Logger", MockLogger, raising=False)
    # Also patch Logger in asynchronous and synchronous modules if they have it
    try:
        import BinaryOptionsToolsV2.pocketoption.asynchronous as async_mod

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
    try:
        import BinaryOptionsToolsV2.pocketoption.synchronous as sync_mod

        if hasattr(sync_mod, "PyConfig"):
            monkeypatch.setattr(sync_mod, "PyConfig", MockPyConfig, raising=False)
        if hasattr(sync_mod, "RawValidator"):
            monkeypatch.setattr(
                sync_mod, "RawValidator", MockRawValidator, raising=False
            )
    except ImportError:
        pass
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
    # Also patch the Rust submodule's RawValidator for Validator.custom()
    try:
        import BinaryOptionsToolsV2.BinaryOptionsToolsV2 as rust_submodule

        if hasattr(rust_submodule, "RawValidator"):
            monkeypatch.setattr(
                rust_submodule, "RawValidator", MockRawValidator, raising=False
            )
    except ImportError:
        pass
    # Create and return a mock instance
    instance = MockPocketOptionAsync()
    MockPocketOptionAsync.new_with_config = lambda *args, **kwargs: instance
    return instance


@pytest.fixture
def sync_client(mock_pocketoption_async):
    """Fixture that creates a PocketOption sync client with mocked backend."""
    client = PocketOption("test_ssid", config={"terminal_logging": False})
    yield client
    try:
        client.shutdown()
    except Exception:
        pass


class TestValidator:
    """Tests for Validator class methods."""

    def test_validator_starts_with(self):
        """Test Validator.starts_with creates valid validator."""
        validator = Validator.starts_with('42["test"')
        assert validator is not None

    def test_validator_contains(self):
        """Test Validator.contains creates valid validator."""
        validator = Validator.contains('"result"')
        assert validator is not None

    def test_validator_regex(self):
        """Test Validator.regex creates valid validator."""
        validator = Validator.regex(r'42\[".*"\]')
        assert validator is not None

    def test_validator_custom_callable(self):
        """Test Validator.custom accepts callable."""

        def custom_check(message):
            return "test" in message

        validator = Validator.custom(custom_check)
        assert validator is not None

    def test_validator_custom_non_callable_raises(self):
        """Test Validator.custom raises TypeError for non-callable."""
        with pytest.raises(TypeError, match="func must be callable"):
            Validator.custom("not a function")


class TestConcurrentOperations:
    """Tests for concurrent operations."""

    def test_concurrent_calls(self, sync_client):
        """Test multiple concurrent operations."""
        results = []

        def worker():
            trade_id, trade = sync_client.buy("EURUSD_otc", 1.0, 60)
            results.append(trade_id)

        threads = [threading.Thread(target=worker) for _ in range(3)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        assert len(results) == 3

    def test_concurrent_reads(self, sync_client):
        """Test concurrent read operations."""
        results = []

        def worker():
            balance = sync_client.balance()
            results.append(balance)

        threads = [threading.Thread(target=worker) for _ in range(3)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        assert len(results) == 3


class TestPocketOptionInit:
    """Tests for PocketOption initialization."""

    def test_init_with_ssid_only(self, mock_pocketoption_async):
        """Test initialization with just SSID."""
        client = PocketOption("test_ssid")
        assert client.client is not None
        client.shutdown()

    def test_init_with_config_dict(self, mock_pocketoption_async):
        """Test initialization with config dict."""
        config = {"terminal_logging": False, "log_level": "INFO"}
        client = PocketOption("test_ssid", config=config)
        assert client.config.terminal_logging is False
        client.shutdown()

    def test_init_with_config_json(self, mock_pocketoption_async):
        """Test initialization with config JSON string."""
        config_json = '{"terminal_logging": false, "log_level": "DEBUG"}'
        client = PocketOption("test_ssid", config=config_json)
        assert client.config.terminal_logging is False
        client.shutdown()

    def test_init_with_config_object(self, mock_pocketoption_async):
        """Test initialization with Config object."""
        cfg = Config()
        cfg.terminal_logging = False
        client = PocketOption("test_ssid", config=cfg)
        assert client.config.terminal_logging is False
        client.shutdown()

    def test_init_with_invalid_config_type(self):
        """Test initialization with invalid config type raises ValueError."""
        with pytest.raises(ValueError, match="Config type mismatch"):
            PocketOption("test_ssid", config=123)

    def test_init_with_custom_url(self, mock_pocketoption_async):
        """Test that custom URL is added to config."""
        client = PocketOption("test_ssid", url="wss://custom.com")
        assert "wss://custom.com" in client.config.urls
        client.shutdown()


class TestBuyAndSell:
    """Tests for buy and sell methods."""

    def test_buy_success(self, sync_client):
        """Test successful buy operation."""
        trade_id, trade = sync_client.buy("EURUSD_otc", 1.0, 60)
        assert trade_id == "trade_123"
        assert trade["asset"] == "EURUSD_otc"
        assert trade["direction"] == "buy"

    def test_buy_with_check_win(self, sync_client):
        """Test buy with check_win=True."""
        trade_id, trade = sync_client.buy("EURUSD_otc", 1.0, 60, check_win=True)
        assert trade["result"] == "win"
        assert trade["profit"] == 1.5

    def test_sell_success(self, sync_client):
        """Test successful sell operation."""
        trade_id, trade = sync_client.sell("EURUSD_otc", 1.0, 60)
        assert trade_id == "trade_456"
        assert trade["direction"] == "sell"

    def test_sell_with_check_win(self, sync_client):
        """Test sell with check_win=True."""
        trade_id, trade = sync_client.sell("EURUSD_otc", 1.0, 60, check_win=True)
        assert trade["result"] == "win"
        assert trade["profit"] == 1.5

    def test_buy_client_error(self, sync_client, mock_pocketoption_async):
        """Test buy when client raises exception."""
        mock_pocketoption_async.buy = AsyncMock(
            side_effect=Exception("Connection lost")
        )
        with pytest.raises(Exception, match="Connection lost"):
            sync_client.buy("EURUSD_otc", 1.0, 60)


class TestCheckWin:
    """Tests for check_win method."""

    def test_check_win_success(self, sync_client):
        """Test check_win with valid trade ID."""
        result = sync_client.check_win("trade_123")
        assert result["id"] == "trade_123"
        assert result["result"] == "win"
        assert result["profit"] == 1.5

    def test_check_win_invalid_id(self, sync_client):
        """Test check_win with invalid trade ID."""
        with pytest.raises(Exception):
            sync_client.check_win("not_found")

    def test_check_win_timeout(self, sync_client, mock_pocketoption_async):
        """Test check_win timeout protection."""
        mock_pocketoption_async.check_win = AsyncMock(side_effect=TimeoutError)
        with pytest.raises(Exception):
            sync_client.check_win("trade_123")


class TestGetDealEndTime:
    """Tests for get_deal_end_time method."""

    def test_get_deal_end_time_success(self, sync_client):
        """Test getting deal end time."""
        end_time = sync_client.get_deal_end_time("trade_123")
        assert end_time is not None
        assert isinstance(end_time, int)

    def test_get_deal_end_time_not_found(self, sync_client):
        """Test get_deal_end_time with invalid ID returns None."""
        end_time = sync_client.get_deal_end_time("invalid")
        assert end_time is None


class TestCandles:
    """Tests for candles and get_candles methods."""

    def test_candles_success(self, sync_client):
        """Test candles retrieval."""
        candles = sync_client.candles("EURUSD_otc", 60)
        assert isinstance(candles, list)
        assert len(candles) > 0
        assert "open" in candles[0]
        assert "close" in candles[0]

    def test_get_candles_success(self, sync_client):
        """Test get_candles with offset."""
        candles = sync_client.get_candles("EURUSD_otc", 60, 10)
        assert isinstance(candles, list)
        assert len(candles) > 0

    def test_get_candles_advanced_success(self, sync_client):
        """Test get_candles_advanced with time parameter."""
        candles = sync_client.get_candles_advanced("EURUSD_otc", 60, 10, 1700000000)
        assert isinstance(candles, list)
        assert len(candles) > 0


class TestBalance:
    """Tests for balance method."""

    def test_balance_success(self, sync_client):
        """Test balance retrieval."""
        balance = sync_client.balance()
        assert isinstance(balance, float)
        assert balance >= 0


class TestOpenedDeals:
    """Tests for opened_deals method."""

    def test_opened_deals_success(self, sync_client):
        """Test opened_deals retrieval."""
        deals = sync_client.opened_deals()
        assert isinstance(deals, list)
        if deals:
            assert "id" in deals[0]

    def test_opened_deals_empty(self, sync_client, mock_pocketoption_async):
        """Test opened_deals when no open deals."""
        mock_pocketoption_async.opened_deals = AsyncMock(return_value=[])
        deals = sync_client.opened_deals()
        assert deals == []


class TestGetPendingDeals:
    """Tests for get_pending_deals method."""

    def test_get_pending_deals_success(self, sync_client):
        """Test get_pending_deals retrieval."""
        pending = sync_client.get_pending_deals()
        assert isinstance(pending, list)


class TestOpenPendingOrder:
    """Tests for open_pending_order method."""

    def test_open_pending_order_success(self, sync_client):
        """Test successful pending order creation."""
        order = sync_client.open_pending_order(
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

    def test_open_pending_order_invalid_params(
        self, sync_client, mock_pocketoption_async
    ):
        """Test open_pending_order with invalid parameters."""
        mock_pocketoption_async.open_pending_order = AsyncMock(
            side_effect=ValueError("Invalid amount")
        )
        with pytest.raises(ValueError, match="Invalid amount"):
            sync_client.open_pending_order(
                0, -1.0, "EURUSD_otc", 1700000000, 1.1, 60, 80, 0
            )


class TestCancelPendingOrder:
    """Tests for pending order cancellation methods."""

    def test_cancel_pending_order_success(self, sync_client):
        result = sync_client.cancel_pending_order(
            "11111111-1111-1111-1111-111111111111"
        )
        assert result["status"] == "cancelled"

    def test_cancel_pending_orders_success(self, sync_client):
        results = sync_client.cancel_pending_orders(
            [
                "11111111-1111-1111-1111-111111111111",
                "22222222-2222-2222-2222-222222222222",
            ]
        )
        assert len(results) == 2
        assert all(result["status"] == "cancelled" for result in results)


class TestClosedDeals:
    """Tests for closed_deals method."""

    def test_closed_deals_success(self, sync_client):
        """Test closed_deals retrieval."""
        deals = sync_client.closed_deals()
        assert isinstance(deals, list)
        if deals:
            assert "result" in deals[0] or "profit" in deals[0]

    def test_closed_deals_empty(self, sync_client, mock_pocketoption_async):
        """Test closed_deals when no closed deals."""
        mock_pocketoption_async.closed_deals = AsyncMock(return_value=[])
        deals = sync_client.closed_deals()
        assert deals == []


class TestClearClosedDeals:
    """Tests for clear_closed_deals method."""

    def test_clear_closed_deals_success(self, sync_client):
        """Test clearing closed deals."""
        sync_client.clear_closed_deals()

    def test_clear_closed_deals_error(self, sync_client, mock_pocketoption_async):
        """Test clear_closed_deals when operation fails."""
        mock_pocketoption_async.clear_closed_deals = AsyncMock(
            side_effect=Exception("Clear failed")
        )
        with pytest.raises(Exception, match="Clear failed"):
            sync_client.clear_closed_deals()


class TestPayout:
    """Tests for payout method."""

    def test_payout_all(self, sync_client):
        """Test payout with no asset parameter (all assets)."""
        payouts = sync_client.payout()
        assert isinstance(payouts, dict)
        assert "EURUSD_otc" in payouts

    def test_payout_single_asset(self, sync_client):
        """Test payout with single asset string."""
        payout = sync_client.payout("EURUSD_otc")
        assert isinstance(payout, int)
        assert payout == 85

    def test_payout_list_of_assets(self, sync_client):
        """Test payout with list of assets."""
        payouts = sync_client.payout(["EURUSD_otc", "GBPUSD_otc"])
        assert isinstance(payouts, list)
        assert len(payouts) == 2
        assert payouts[0] == 85

    def test_payout_invalid_asset(self, sync_client):
        """Test payout with invalid asset returns None."""
        payout = sync_client.payout("INVALID_ASSET")
        assert payout is None

    def test_payout_empty_list(self, sync_client):
        """Test payout with empty list."""
        payouts = sync_client.payout([])
        assert payouts == []


class TestActiveAssets:
    """Tests for active_assets method."""

    def test_active_assets_success(self, sync_client):
        """Test active_assets retrieval."""
        assets = sync_client.active_assets()
        assert isinstance(assets, list)
        if assets:
            assert "symbol" in assets[0]
            assert "payout" in assets[0]

    def test_active_assets_empty(self, sync_client, mock_pocketoption_async):
        """Test active_assets when no assets available."""
        mock_pocketoption_async.active_assets = AsyncMock(return_value=[])
        assets = sync_client.active_assets()
        assert assets == []


class TestHistory:
    """Tests for history method."""

    def test_history_success(self, sync_client):
        """Test history retrieval."""
        candles = sync_client.history("EURUSD_otc", 60)
        assert isinstance(candles, list)
        assert len(candles) > 0
        assert "time" in candles[0]

    def test_history_empty(self, sync_client, mock_pocketoption_async):
        """Test history when no data available."""
        mock_pocketoption_async.history = AsyncMock(return_value=[])
        candles = sync_client.history("EURUSD_otc", 60)
        assert candles == []


class TestSubscriptions:
    """Tests for subscription methods."""

    def test_subscribe_symbol_success(self, sync_client):
        """Test subscribe_symbol creates valid subscription."""
        sub = sync_client.subscribe_symbol("EURUSD_otc")
        assert sub is not None
        assert hasattr(sub, "__aiter__")

    def test_subscribe_symbol_chuncked_success(self, sync_client):
        """Test subscribe_symbol_chuncked with valid chunk size."""
        sub = sync_client.subscribe_symbol_chuncked("EURUSD_otc", 10)
        assert sub is not None
        assert hasattr(sub, "__aiter__")

    def test_subscribe_symbol_chuncked_invalid_chunk(self, sync_client):
        """Test subscribe_symbol_chuncked with invalid chunk size."""
        sub = sync_client.subscribe_symbol_chuncked("EURUSD_otc", 0)
        assert sub is not None

    def test_subscribe_symbol_timed_success(self, sync_client):
        """Test subscribe_symbol_timed with timedelta."""
        sub = sync_client.subscribe_symbol_timed("EURUSD_otc", timedelta(seconds=5))
        assert sub is not None
        assert hasattr(sub, "__aiter__")

    def test_subscribe_symbol_time_aligned_success(self, sync_client):
        """Test subscribe_symbol_time_aligned with timedelta."""
        sub = sync_client.subscribe_symbol_time_aligned(
            "EURUSD_otc", timedelta(seconds=60)
        )
        assert sub is not None
        assert hasattr(sub, "__aiter__")


class TestGetServerTime:
    """Tests for get_server_time method."""

    def test_get_server_time_success(self, sync_client):
        """Test server time retrieval."""
        time = sync_client.get_server_time()
        assert isinstance(time, int)
        assert time > 0

    def test_get_server_time_error(self, sync_client, mock_pocketoption_async):
        """Test get_server_time when client fails."""
        mock_pocketoption_async.get_server_time = AsyncMock(
            side_effect=Exception("Connection error")
        )
        with pytest.raises(Exception, match="Connection error"):
            sync_client.get_server_time()


class TestWaitForAssets:
    """Tests for wait_for_assets method."""

    def test_wait_for_assets_success(self, sync_client):
        """Test wait_for_assets completes quickly."""
        sync_client.wait_for_assets(timeout=1.0)

    def test_wait_for_assets_timeout(self, sync_client, mock_pocketoption_async):
        """Test wait_for_assets timeout."""
        mock_pocketoption_async.wait_for_assets = AsyncMock(side_effect=TimeoutError)
        with pytest.raises(Exception):
            sync_client.wait_for_assets(timeout=1.0)


class TestIsDemo:
    """Tests for is_demo method."""

    def test_is_demo_success(self, sync_client):
        """Test is_demo returns boolean."""
        result = sync_client.is_demo()
        assert isinstance(result, bool)


class TestConnectionMethods:
    """Tests for disconnect, connect, reconnect methods."""

    def test_disconnect_success(self, sync_client):
        """Test disconnect."""
        sync_client.disconnect()
        assert sync_client.client._connected is False

    def test_connect_success(self, sync_client):
        """Test connect after disconnect."""
        sync_client.disconnect()
        sync_client.connect()
        assert sync_client.client._connected is True

    def test_reconnect_success(self, sync_client):
        """Test reconnect."""
        sync_client.reconnect()
        assert sync_client.client._connected is True


class TestUnsubscribe:
    """Tests for unsubscribe method."""

    def test_unsubscribe_success(self, sync_client):
        """Test unsubscribe from asset."""
        sync_client.unsubscribe("EURUSD_otc")


class TestShutdown:
    """Tests for shutdown method."""

    def test_shutdown_success(self, sync_client):
        """Test shutdown."""
        sync_client.shutdown()
        assert sync_client.client._closed is True


class TestCreateRawHandler:
    """Tests for create_raw_handler method."""

    def test_create_raw_handler_success(self, sync_client):
        """Test creating raw handler."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        assert handler is not None
        assert handler.id() is not None

    def test_raw_handler_send_text(self, sync_client):
        """Test raw handler send_text."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        handler.send_text('42["ping"]')

    def test_raw_handler_send_binary(self, sync_client):
        """Test raw handler send_binary."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        handler.send_binary(b"\x00\x01")

    def test_raw_handler_send_and_wait(self, sync_client):
        """Test raw handler send_and_wait."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        response = handler.send_and_wait('42["getServerTime"]')
        assert isinstance(response, str)

    def test_raw_handler_wait_next(self, sync_client):
        """Test raw handler wait_next."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        message = handler.wait_next()
        assert isinstance(message, str)

    def test_raw_handler_subscribe(self, sync_client):
        """Test raw handler subscribe."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        stream = handler.subscribe()
        assert stream is not None

    def test_raw_handler_close(self, sync_client):
        """Test raw handler close."""
        validator = Validator.starts_with('42["test"')
        handler = sync_client.create_raw_handler(validator)
        handler.close()


class TestSendRawMessage:
    """Tests for send_raw_message method."""

    def test_send_raw_message_success(self, sync_client):
        """Test sending raw message."""
        sync_client.send_raw_message('42["ping"]')

    def test_send_raw_message_error(self, sync_client, mock_pocketoption_async):
        """Test send_raw_message when client fails."""
        mock_pocketoption_async.send_raw_message = AsyncMock(
            side_effect=Exception("Send failed")
        )
        with pytest.raises(Exception, match="Send failed"):
            sync_client.send_raw_message('42["ping"]')


class TestCreateRawOrder:
    """Tests for create_raw_order and variants."""

    def test_create_raw_order_success(self, sync_client):
        """Test create_raw_order with validator."""
        validator = Validator.contains("response")
        response = sync_client.create_raw_order('42["test"]', validator)
        assert isinstance(response, str)

    def test_create_raw_order_timeout(self, sync_client):
        """Test create_raw_order with timeout."""
        validator = Validator.contains("response")
        timeout = timedelta(seconds=5)
        response = sync_client.create_raw_order_with_timeout(
            '42["test"]', validator, timeout
        )
        assert isinstance(response, str)

    def test_create_raw_order_with_timeout_success(self, sync_client):
        """Test create_raw_order_with_timeout."""
        validator = Validator.contains("response")
        timeout = timedelta(seconds=5)
        response = sync_client.create_raw_order_with_timeout(
            '42["test"]', validator, timeout
        )
        assert isinstance(response, str)

    def test_create_raw_order_with_timeout_and_retry_success(self, sync_client):
        """Test create_raw_order_with_timeout_and_retry."""
        validator = Validator.contains("response")
        timeout = timedelta(seconds=5)
        response = sync_client.create_raw_order_with_timeout_and_retry(
            '42["test"]', validator, timeout
        )
        assert isinstance(response, str)

    def test_create_raw_iterator_success(self, sync_client):
        """Test create_raw_iterator returns async iterator."""
        validator = Validator.contains("event")
        iterator = sync_client.create_raw_iterator('42["subscribe"]', validator)
        assert iterator is not None
        assert hasattr(iterator, "__aiter__")

    def test_create_raw_iterator_with_timeout(self, sync_client):
        """Test create_raw_iterator with timeout."""
        validator = Validator.contains("event")
        timeout = timedelta(seconds=30)
        iterator = sync_client.create_raw_iterator(
            '42["subscribe"]', validator, timeout
        )
        assert iterator is not None


class TestContextManager:
    """Tests for sync context manager."""

    def test_sync_context_manager(self):
        """Test sync context manager enter and exit."""
        with PocketOption("test_ssid") as client:
            assert client.client is not None
