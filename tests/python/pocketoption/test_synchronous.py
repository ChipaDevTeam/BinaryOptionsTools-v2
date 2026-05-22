import os
import time

import pytest

from BinaryOptionsToolsV2.pocketoption.synchronous import PocketOption


def test_sync_manual_connect_shutdown():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    api = PocketOption(ssid)
    api.connect()

    server_time = api.get_server_time()
    assert server_time > 0

    api.shutdown()


def test_sync_config_variations():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    # Test Config from dict
    config_dict = {"terminal_logging": False}
    api1 = PocketOption(ssid, config=config_dict)
    assert api1._client.config.terminal_logging is False
    api1.shutdown()

    # Test invalid config type
    with pytest.raises(ValueError, match="Config type mismatch"):
        PocketOption(ssid, config=123)  # type: ignore[arg-type]


def test_sync_context_manager():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    with PocketOption(ssid) as api:
        # Demo accounts may return -1.0 if balance is not yet available
        balance = api.balance()
        if balance < 0:
            print(f"  Note: Demo account balance is {balance} (may not be available yet)")


def test_sync_raw_operations():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    with PocketOption(ssid) as api:
        api.send_raw_message('42["ping"]')

        try:
            # We don't want to wait too long in tests
            # But SyncPocketOption might not have a direct timeout for create_raw_order
            # so we just test send_raw_message for now to avoid hanging
            pass
        except Exception:
            pass


def test_sync_subscription():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    with PocketOption(ssid) as api:
        sub = api.subscribe_symbol("EURUSD_otc")
        try:
            msg = next(sub)
            assert isinstance(msg, (dict, list))
        except StopIteration:
            pytest.skip("Subscription did not yield data (server may not be streaming)")


def test_sync_payout_invalid(api_sync):
    assert (
        api_sync.payout("INVALID_ASSET") is None
        or api_sync.payout("INVALID_ASSET") == 0
    )


def test_sync_check_win_invalid(api_sync):
    import uuid

    invalid_id = str(uuid.uuid4())
    try:
        api_sync.check_win(invalid_id)
    except Exception as e:
        assert "failed to find deal" in str(e).lower()


def test_sync_close_resilience():
    """Verify close() does not hang when the event loop is blocked."""
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    api = PocketOption(ssid)
    # Simulate a busy loop by scheduling a long-running coroutine
    import asyncio

    async def blocker():
        await asyncio.sleep(999)

    asyncio.run_coroutine_threadsafe(blocker(), api.loop)
    # close() should complete within a reasonable time
    start = time.time()
    api.close()
    elapsed = time.time() - start
    assert elapsed < 20, f"close() took {elapsed:.1f}s, expected < 20s"


def test_sync_subscription_cancel():
    """Verify SyncSubscription can be stopped via unsubscribe."""
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    with PocketOption(ssid) as api:
        sub = api.subscribe_symbol("EURUSD_otc")
        assert sub is not None
        api.unsubscribe("EURUSD_otc")


def test_sync_del_cleanup():
    """Verify __del__ cleans up the event loop thread if close() was not called."""
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    import threading

    api = PocketOption(ssid)
    loop_thread = api._loop_thread
    assert loop_thread.is_alive()
    # Trigger __del__ without calling close()
    del api
    import gc

    gc.collect()
    # Give the thread time to stop
    time.sleep(0.5)
    # The thread should no longer be alive after __del__ cleanup
    assert not loop_thread.is_alive(), (
        "Event loop thread should be stopped after __del__"
    )
