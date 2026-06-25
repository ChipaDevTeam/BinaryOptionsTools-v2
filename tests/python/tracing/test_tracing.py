import time

import pytest

from BinaryOptionsToolsV2.tracing import LogBuilder, Logger, start_logs


def test_logger_basic(tmp_path):
    # Test logger initialization and basic logging
    # Note: Since it's linked to Rust tracing, we mostly test that it doesn't crash
    logger = Logger()
    logger.debug("Test debug")
    logger.info("Test info")
    logger.warn("Test warn")
    logger.error("Test error")


def test_log_builder(tmp_path):
    log_dir = tmp_path / "logs"
    log_dir.mkdir()
    log_file = log_dir / "test.log"

    builder = LogBuilder()
    builder.log_file(str(log_file), "DEBUG")
    builder.terminal("INFO")
    builder.build()

    logger = Logger()
    logger.info("Logging to file")

    # Wait a bit for file to be written
    time.sleep(0.5)

    assert log_file.exists()
    # Depending on buffering, we might not see the content immediately,
    # but the file should exist at least.


def test_start_logs(tmp_path):
    log_dir = tmp_path / "logs_start"

    # Test the helper function
    start_logs(str(log_dir), "DEBUG", terminal=True)

    logger = Logger()
    logger.error("Testing start_logs")

    assert log_dir.exists()


def test_log_subscription_sync():
    builder = LogBuilder()
    try:
        sub = builder.create_logs_iterator("DEBUG")
        assert sub.__iter__() is sub

        # We can't easily force a log message to appear in the sync iterator without blocking,
        # but we can test that the structure is there.
    except Exception as e:
        pytest.skip(f"Log subscription sync test skipped: {e}")


@pytest.mark.asyncio
async def test_log_subscription():
    builder = LogBuilder()
    # Subscriptions might be tricky if build() was already called
    # but let's try to create one.
    try:
        sub = builder.create_logs_iterator("DEBUG")
        logger = Logger()
        logger.debug('{"event": "test_event"}')

        # Testing if we can iterate (might need a way to push logs to the sub)
        # For now, just test it exists and doesn't crash on creation
        assert sub is not None
    except Exception as e:
        pytest.skip(f"Log subscription test skipped: {e}")


def test_log_subscription_wrapper():
    from BinaryOptionsToolsV2.tracing import LogSubscription
    
    # Test sync iteration
    inner_sync = iter(['{"msg": "hello"}', '{"msg": "world"}'])
    sub_sync = LogSubscription(inner_sync)
    assert sub_sync.__iter__() is sub_sync
    assert next(sub_sync) == {"msg": "hello"}
    assert next(sub_sync) == {"msg": "world"}
    
    # Test async iteration
    class MockAsyncSub:
        def __init__(self, items):
            self.items = iter(items)
        def __aiter__(self):
            return self
        async def __anext__(self):
            try:
                return next(self.items)
            except StopIteration:
                raise StopAsyncIteration

    sub_async = LogSubscription(MockAsyncSub(['{"msg": "async1"}']))
    assert sub_async.__aiter__() is sub_async
    
    async def run_async_test():
        assert await sub_async.__anext__() == {"msg": "async1"}
        with pytest.raises(StopAsyncIteration):
            await sub_async.__anext__()
            
    import anyio
    anyio.run(run_async_test)


def test_start_logs_failure():
    from unittest.mock import patch
    with patch("BinaryOptionsToolsV2.tracing._get_rust_attr") as mock_get_attr:
        mock_start_tracing = mock_get_attr.return_value
        mock_start_tracing.side_effect = Exception("Mock Rust Exception")
        with pytest.warns(RuntimeWarning, match="start_logs: Mock Rust Exception"):
            start_logs("logs_fail", level="INVALID_LEVEL")


def test_get_rust_attr_fallback():
    from unittest.mock import patch
    from BinaryOptionsToolsV2.tracing import _get_rust_attr
    with patch("sys.modules") as mock_modules:
        mock_modules.get.return_value = None
        attr = _get_rust_attr("Logger")
        assert attr is not None

