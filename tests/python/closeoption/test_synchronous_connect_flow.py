"""Regression tests for the CloseOption synchronous connect flow.

Covers:
- the eager constructor connection going through the bounded ``_connect()``
  path (``connection_initialization_timeout_secs`` is enforced),
- deferred connections (``connect_on_init=False``) being established through
  the same bounded path before an operation runs,
- operation futures (``buy``/``sell``) NOT being subject to the connection
  initialization timeout, so a slow order can never be abandoned mid-flight
  and re-issued by a retry.
"""

import asyncio
import time

import pytest

from BinaryOptionsToolsV2.closeoption.synchronous import CloseOption
from BinaryOptionsToolsV2.config import Config

import BinaryOptionsToolsV2.closeoption.synchronous as sync_module


class FakeCloseOptionAsync:
    """Stand-in for CloseOptionAsync driven entirely by the sync client's loop.

    Delays are read from class attributes so a test can configure behaviour
    before the client constructor instantiates the object.
    """

    connect_delay = 0.0
    op_delay = 0.0
    instances = []

    def __init__(self, ssid, url=None, config=None):
        self._ssid = ssid
        self._url = url
        self._config = config
        self.connect_delay = FakeCloseOptionAsync.connect_delay
        self.op_delay = FakeCloseOptionAsync.op_delay
        self._connected = False
        self.connect_calls = 0
        self.connected_when_buy = None
        FakeCloseOptionAsync.instances.append(self)

    @property
    def is_connected(self):
        return self._connected

    async def connect(self):
        self.connect_calls += 1
        if self.connect_delay:
            await asyncio.sleep(self.connect_delay)
        self._connected = True

    async def _order(self, name, delay):
        if name == "buy":
            self.connected_when_buy = self._connected
        if delay:
            await asyncio.sleep(delay)
        return {"ok": True, "side": name}

    async def buy(self, asset, amount, time):
        return await self._order("buy", self.op_delay)

    async def sell(self, asset, amount, time):
        return await self._order("sell", self.op_delay)

    async def shutdown(self):
        self._connected = False


@pytest.fixture(autouse=True)
def _fake_async_client(monkeypatch):
    FakeCloseOptionAsync.instances.clear()
    monkeypatch.setattr(sync_module, "CloseOptionAsync", FakeCloseOptionAsync)


def _make_client(monkeypatch, **kwargs):
    config = Config(connection_initialization_timeout_secs=1)
    return CloseOption("token|sid|demo|pub|hid", config=config, **kwargs)


def test_eager_connect_is_bounded_by_connection_timeout(monkeypatch):
    """connect_on_init=True must fail fast when the connection exceeds the
    configured connection_initialization_timeout_secs (i.e. it goes through the
    bounded _connect() path, not an unbounded _run())."""
    monkeypatch.setattr(FakeCloseOptionAsync, "connect_delay", 5.0)

    started = time.monotonic()
    with pytest.raises(TimeoutError, match="CloseOption connection timed out"):
        _make_client(monkeypatch, connect_on_init=True)
    elapsed = time.monotonic() - started

    assert elapsed < 4.0, "timeout was not enforced while establishing the connection"
    # The connection attempt was actually started before the timeout fired.
    assert len(FakeCloseOptionAsync.instances) == 1
    assert FakeCloseOptionAsync.instances[0].connect_calls == 1


def test_deferred_connect_runs_before_first_operation(monkeypatch):
    """With connect_on_init=False the first operation must establish the
    connection through the bounded path before the order is dispatched."""
    monkeypatch.setattr(FakeCloseOptionAsync, "connect_delay", 0.05)

    client = _make_client(monkeypatch, connect_on_init=False)
    try:
        fake = client._async_client
        assert fake.is_connected is False
        result = client.buy("EURUSD", 1.0, 1)
        assert result == {"ok": True, "side": "buy"}
        assert fake.connect_calls == 1
        assert fake.connected_when_buy is True
        # A subsequent operation reuses the established connection.
        assert client.sell("EURUSD", 1.0, 1) == {"ok": True, "side": "sell"}
        assert fake.connect_calls == 1
    finally:
        client.shutdown()


def test_operations_are_not_subject_to_connection_timeout(monkeypatch):
    """Buy/sell futures must not be bounded by connection_initialization_timeout_secs:
    a slow (but in-flight) order resolves normally instead of being abandoned,
    which would allow a retry to place a duplicate order."""
    client = _make_client(monkeypatch, connect_on_init=True)
    try:
        # Order takes longer than the 1s connection timeout: it must still complete.
        client._async_client.op_delay = 1.6
        started = time.monotonic()
        result = client.buy("EURUSD", 1.0, 1)
        elapsed = time.monotonic() - started
        assert result == {"ok": True, "side": "buy"}
        assert elapsed >= 1.4
    finally:
        client.shutdown()
