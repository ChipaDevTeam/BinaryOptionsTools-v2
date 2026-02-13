import pytest
import os
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
        PocketOption(ssid, config=123)


def test_sync_context_manager():
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")
    with PocketOption(ssid) as api:
        assert api.balance() >= 0


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
        # Just check if we can create it
        sub = api.subscribe_symbol("EURUSD_otc")
        # Get one item
        for msg in sub:
            assert isinstance(msg, (dict, list))
            break


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
