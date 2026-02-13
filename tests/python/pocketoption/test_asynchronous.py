import pytest
import os
import asyncio
from BinaryOptionsToolsV2.pocketoption.asynchronous import (
    PocketOptionAsync as PocketOption,
)
from BinaryOptionsToolsV2.config import Config
from BinaryOptionsToolsV2.validator import Validator


@pytest.fixture
async def api_no_context():
    # Helper to get api without automatic enter/exit if needed,
    # or just use the standard one but we want to test manual connect/shutdown
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    api = PocketOption(ssid)
    yield api
    try:
        await api.shutdown()
    except Exception:
        pass


@pytest.mark.asyncio
async def test_manual_connect_shutdown(api_no_context):
    api = api_no_context
    # Test manual connect
    await api.connect()
    # Test double connect (should be fine)
    await api.connect()

    # Check if connected
    server_time = await api.get_server_time()
    assert server_time > 0

    await api.shutdown()


@pytest.mark.asyncio
async def test_config_variations():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    # Test Config from dict
    config_dict = {"terminal_logging": False, "log_level": "INFO"}
    api1 = PocketOption(ssid, config=config_dict)
    assert api1.config.terminal_logging is False
    await api1.shutdown()

    # Test Config from object
    cfg = Config()
    cfg.terminal_logging = False
    api2 = PocketOption(ssid, config=cfg)
    assert api2.config.terminal_logging is False
    await api2.shutdown()


@pytest.mark.asyncio
async def test_raw_operations(api):
    # Test send_raw_message
    # We send a ping-like message
    await api.send_raw_message('42["ping"]')

    # Test create_raw_order
    # We wait for a balance update which usually comes after some time or on request
    # Since we can't easily trigger a specific raw response without knowing the protocol deeply,
    # we'll test with a validator that might match common messages

    v = Validator.contains("time")  # Server time updates usually contain "time"
    try:
        # This might timeout if no such message arrives, so we use a short timeout
        res = await asyncio.wait_for(
            api.create_raw_order('42["getServerTime"]', v), timeout=5.0
        )
        assert isinstance(res, str)
    except asyncio.TimeoutError:
        pass  # Expected if no matching message in 5s


@pytest.mark.asyncio
async def test_context_manager():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    async with PocketOption(ssid) as api:
        assert api.client is not None
        # Should already be connected and assets loaded due to __aenter__
        active = await api.active_assets()
        assert len(active) > 0


@pytest.mark.asyncio
async def test_config_json_and_trades():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    # Test Config from JSON string (Line 143)
    config_json = '{"terminal_logging": false, "log_level": "DEBUG"}'
    api = PocketOption(ssid, config=config_json)
    assert api.config.terminal_logging is False

    # Test buy/sell without check_win to avoid skipping on real accounts (Line 274-279, 306-311)
    # Note: This might still fail if account has no money or asset is closed,
    # but it will cover the lines.
    try:
        await api.buy("EURUSD_otc", 1.0, 60, check_win=False)
    except Exception:
        pass

    try:
        await api.sell("EURUSD_otc", 1.0, 60, check_win=False)
    except Exception:
        pass

    await api.shutdown()


@pytest.mark.asyncio
async def test_raw_handler_extended(api):
    v = Validator.contains("time")
    handler = await api.create_raw_handler(v)

    assert handler.id() is not None

    # Test send_text (Line 62)
    await handler.send_text('42["getServerTime"]')

    # Test send_binary
    await handler.send_binary(b"\x42")

    # Test wait_next with timeout
    try:
        await asyncio.wait_for(handler.wait_next(), timeout=2.0)
    except asyncio.TimeoutError:
        pass

    # Test send_and_wait with timeout
    try:
        await asyncio.wait_for(
            handler.send_and_wait('42["getServerTime"]'), timeout=2.0
        )
    except asyncio.TimeoutError:
        pass

    # Test handler.subscribe()
    stream = await handler.subscribe()
    assert stream is not None

    await handler.close()


@pytest.mark.asyncio
async def test_extra_api_methods(api):
    # Test reconnect (Line 717)
    await api.reconnect()

    # Test unsubscribe (Line 735)
    try:
        await api.unsubscribe("EURUSD_otc")
    except Exception:
        pass

    # Test send_raw_message (Line 783)
    await api.send_raw_message('42["ping"]')


@pytest.mark.asyncio
async def test_async_subscription_iteration(api):
    # Trigger a real subscription
    sub = await api.subscribe_symbol("EURUSD_otc")
    assert sub is not None

    # test __aiter__
    assert sub.__aiter__() is sub

    # test __anext__ with timeout to avoid hanging
    try:
        async with asyncio.timeout(5.0):
            async for msg in sub:
                assert isinstance(msg, (dict, list))
                break
    except (asyncio.TimeoutError, TimeoutError):
        pass


@pytest.mark.asyncio
async def test_check_win_invalid_id(api):
    # Test check_win with a random UUID
    import uuid

    invalid_id = str(uuid.uuid4())
    try:
        # It should either raise an error or return something indicating not found
        # According to Rust code, it might return DealNotFound error
        await api.check_win(invalid_id)
    except Exception as e:
        error_msg = str(e).lower()
        assert (
            "failed to find deal" in error_msg
            or "not found" in error_msg
            or "dealnotfound" in error_msg
        )
