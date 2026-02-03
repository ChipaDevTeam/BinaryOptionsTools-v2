import pytest
import sys
from unittest.mock import MagicMock

# Mock the Rust extension module BEFORE importing the package
mock_rust_ext = MagicMock()
# Mock the classes expected from the Rust extension
mock_rust_ext.RawPocketOption = MagicMock()
mock_rust_ext.LogBuilder = MagicMock()
mock_rust_ext.Logger = MagicMock()
mock_rust_ext.start_tracing = MagicMock()

# We need to handle how it's imported.
# It seems the code tries `from BinaryOptionsToolsV2.BinaryOptionsToolsV2 import ...`
# AND `from BinaryOptionsToolsV2 import ...`

# Mock `BinaryOptionsToolsV2.BinaryOptionsToolsV2`
sys.modules["BinaryOptionsToolsV2.BinaryOptionsToolsV2"] = mock_rust_ext

# Also mock `BinaryOptionsToolsV2` if strictly needed, but `BinaryOptionsToolsV2` is the package we are testing.
# The package `BinaryOptionsToolsV2` tries to import from itself (the extension).

# Let's try to patch the specific imports inside the modules after import?
# No, imports happen at top level.

# If we look at `tracing.py`:
# try:
#     from BinaryOptionsToolsV2.BinaryOptionsToolsV2 import LogBuilder as RustLogBuilder
# except ImportError:
#     from BinaryOptionsToolsV2 import LogBuilder as RustLogBuilder

# If we run with PYTHONPATH=.../BinaryOptionsToolsV2 (the parent of the inner BinaryOptionsToolsV2),
# then `import BinaryOptionsToolsV2` imports the package.
# Inside that package, it tries to import the extension.

# If we set sys.modules['BinaryOptionsToolsV2'] = mock_rust_ext, we block the actual package.
# But we want to test the actual package code (`PocketOptionAsync` etc).

# The inner `BinaryOptionsToolsV2` used in imports refers to the compiled extension.
# If we are in the source tree, `BinaryOptionsToolsV2` is the package directory.
# The code expects the extension to be importable.

# Strategy:
# 1. Allow `BinaryOptionsToolsV2` (package) to be imported.
# 2. Mock `BinaryOptionsToolsV2.BinaryOptionsToolsV2` (extension).
# 3. But `tracing.py` also tries `from BinaryOptionsToolsV2 import LogBuilder`.
#    This implies `LogBuilder` should be available directly under `BinaryOptionsToolsV2`?
#    This usually happens if `__init__.py` exposes it, or if it is a flat module.

# Let's mock `BinaryOptionsToolsV2.BinaryOptionsToolsV2` which satisfies the first import attempt in `tracing.py`.

import os
from unittest.mock import AsyncMock, patch
from datetime import timedelta

# Set up the mock for the extension module
sys.modules["BinaryOptionsToolsV2.BinaryOptionsToolsV2"] = mock_rust_ext

# Now we can import the package
from BinaryOptionsToolsV2 import PocketOptionAsync
# Also need to make sure RawPocketOption is available where PocketOptionAsync looks for it.
# It looks in `..BinaryOptionsToolsV2` (relative) or `BinaryOptionsToolsV2` (absolute).

# Mock responses
MOCK_BALANCE = 1000.0
MOCK_SSID = "test_ssid"

@pytest.fixture
def mock_client():
    # We need to patch where RawPocketOption is used.
    # In `asynchronous.py`: `from ..BinaryOptionsToolsV2 import RawPocketOption`
    # or `from BinaryOptionsToolsV2 import RawPocketOption`

    # Since we mocked `BinaryOptionsToolsV2.BinaryOptionsToolsV2`, let's ensure it has what we need.
    mock_rust_client = MagicMock()
    mock_rust_client.balance = AsyncMock(return_value=MOCK_BALANCE)
    mock_rust_client.buy = AsyncMock(return_value=("trade_id_123", '{"id": "trade_id_123", "profit": 0, "amount": 10}'))
    mock_rust_client.check_win = AsyncMock(return_value='{"id": "trade_id_123", "profit": 8.5, "result": "win"}')
    mock_rust_client.get_server_time = AsyncMock(return_value=1620000000)

    mock_rust_ext.RawPocketOption.new_with_config.return_value = mock_rust_client

    yield mock_rust_client

@pytest.fixture
def api(mock_client):
    return PocketOptionAsync(MOCK_SSID)

@pytest.mark.asyncio
async def test_balance(api, mock_client):
    balance = await api.balance()
    assert balance == MOCK_BALANCE
    mock_client.balance.assert_called_once()

@pytest.mark.asyncio
async def test_buy(api, mock_client):
    trade_id, trade_info = await api.buy("EURUSD_otc", 10.0, 60, check_win=False)
    assert trade_id == "trade_id_123"
    assert trade_info["amount"] == 10
    mock_client.buy.assert_called_with("EURUSD_otc", 10.0, 60)

@pytest.mark.asyncio
async def test_check_win(api, mock_client):
    # Mock get_deal_end_time to return a time in the past so check_win proceeds
    mock_client.get_deal_end_time = AsyncMock(return_value=1620000000)

    with patch("time.time", return_value=1620000010): # current time > end time
        result = await api.check_win("trade_id_123")
        assert result["result"] == "win"
        assert result["profit"] == 8.5

@pytest.mark.asyncio
async def test_server_time(api, mock_client):
    time = await api.get_server_time()
    assert time == 1620000000
    mock_client.get_server_time.assert_called_once()
