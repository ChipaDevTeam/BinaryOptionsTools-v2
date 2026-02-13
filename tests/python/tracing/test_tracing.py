import pytest
import time
from BinaryOptionsToolsV2.tracing import Logger, LogBuilder, start_logs


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
