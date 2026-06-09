"""
Tests for the email/password login flow in pocketoption.tools.login.

Unit tests mock the HTTP layer so they run without network access.
Integration tests hit the real PocketOption endpoint and require
POCKET_OPTION_EMAIL and POCKET_OPTION_PASSWORD environment variables.

Run unit tests only:
    pytest tests/python/pocketoption/test_login.py -v -k "not integration"

Run integration tests (needs real credentials):
    POCKET_OPTION_EMAIL=you@example.com POCKET_OPTION_PASSWORD=secret \
    pytest tests/python/pocketoption/test_login.py -v -k integration
"""

from __future__ import annotations

import asyncio
import os
import sys
from io import BytesIO
from unittest.mock import MagicMock, patch

import pytest

# Make sure we can import from source when not installed.
_source = os.path.join(os.path.dirname(__file__), "../../../python")
if _source not in sys.path:
    sys.path.insert(0, _source)

from BinaryOptionsToolsV2.pocketoption.tools.login import (
    LoginError,
    _build_multipart,
    _extract_token,
    login,
    login_async,
)


# ── Fixtures / helpers ────────────────────────────────────────────────────────

FAKE_TOKEN = "FakeCSRFToken1234"
FAKE_SESSION = "fakesessioncookievalue9999"

LOGIN_PAGE_HTML = f"""
<html><body>
<form method="post">
  <input type="hidden" name="token" value="{FAKE_TOKEN}">
</form>
</body></html>
"""

LOGIN_PAGE_HTML_ALT_ORDER = f"""
<html><body>
<form method="post">
  <input type="hidden" value="{FAKE_TOKEN}" name="token">
</form>
</body></html>
"""


def _make_fake_response(body: str, cookies: dict[str, str] | None = None):
    """Return a mock that mimics urllib response used as a context manager."""
    resp = MagicMock()
    resp.read.return_value = body.encode()
    resp.__enter__ = lambda s: s
    resp.__exit__ = MagicMock(return_value=False)
    return resp


class _FakeOpener:
    """Simulates urllib opener, injects cookies manually after POST."""

    def __init__(self, jar, get_html: str, post_html: str, session: str | None):
        self._jar = jar
        self._get_html = get_html
        self._post_html = post_html
        self._session = session
        self._call_count = 0

    def open(self, req, timeout=30):
        self._call_count += 1
        if self._call_count == 1:
            # GET
            return _make_fake_response(self._get_html)
        else:
            # POST — inject the session cookie into the jar
            if self._session:
                cookie = MagicMock()
                cookie.name = "po_session"
                cookie.value = self._session
                self._jar._cookies = [cookie]  # monkey-patch for iteration
            return _make_fake_response(self._post_html)


# ── Unit tests ────────────────────────────────────────────────────────────────


class TestExtractToken:
    def test_normal_order(self):
        html = '<input type="hidden" name="token" value="abc123">'
        assert _extract_token(html) == "abc123"

    def test_reversed_order(self):
        html = '<input type="hidden" value="xyz789" name="token">'
        assert _extract_token(html) == "xyz789"

    def test_single_quotes(self):
        html = "<input type='hidden' name='token' value='tok'>"
        assert _extract_token(html) == "tok"

    def test_not_found(self):
        assert _extract_token("<html>no token here</html>") is None

    def test_full_login_page(self):
        assert _extract_token(LOGIN_PAGE_HTML) == FAKE_TOKEN

    def test_alt_order_login_page(self):
        assert _extract_token(LOGIN_PAGE_HTML_ALT_ORDER) == FAKE_TOKEN


class TestBuildMultipart:
    def test_contains_boundary(self):
        body = _build_multipart({"email": "a@b.com"}, "TESTBOUNDARY")
        assert b"--TESTBOUNDARY" in body
        assert b"--TESTBOUNDARY--" in body

    def test_contains_field(self):
        body = _build_multipart({"email": "a@b.com", "password": "secret"}, "B")
        assert b"a@b.com" in body
        assert b"secret" in body

    def test_content_disposition(self):
        body = _build_multipart({"myfield": "myvalue"}, "BOUND")
        assert b'name="myfield"' in body


class TestLoginUnit:
    """Mocked HTTP layer — no network required."""

    def _patch_opener(self, jar, session: str | None = FAKE_SESSION):
        fake = _FakeOpener(jar, LOGIN_PAGE_HTML, '{"status":"ok"}', session)
        return fake

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.CookieJar")
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.build_opener")
    def test_successful_login_real_account(self, mock_build_opener, mock_jar_cls):
        jar = MagicMock()
        jar.__iter__ = MagicMock(
            return_value=iter([_cookie("po_session", FAKE_SESSION)])
        )
        mock_jar_cls.return_value = jar
        mock_build_opener.return_value = _FakeOpener(
            jar, LOGIN_PAGE_HTML, '{"status":"ok"}', None
        )

        result = login("user@example.com", "pass123", demo=False)

        assert result.startswith('42["auth",')
        assert FAKE_SESSION in result
        assert '"isDemo":0' in result

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.CookieJar")
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.build_opener")
    def test_successful_login_demo_account(self, mock_build_opener, mock_jar_cls):
        jar = MagicMock()
        jar.__iter__ = MagicMock(
            return_value=iter([_cookie("po_session", FAKE_SESSION)])
        )
        mock_jar_cls.return_value = jar
        mock_build_opener.return_value = _FakeOpener(
            jar, LOGIN_PAGE_HTML, '{"status":"ok"}', None
        )

        result = login("user@example.com", "pass123", demo=True)

        assert '"isDemo":1' in result

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.CookieJar")
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.build_opener")
    def test_missing_token_raises(self, mock_build_opener, mock_jar_cls):
        jar = MagicMock()
        jar.__iter__ = MagicMock(return_value=iter([]))
        mock_jar_cls.return_value = jar
        mock_build_opener.return_value = _FakeOpener(
            jar, "<html>no token</html>", "", None
        )

        with pytest.raises(ValueError, match="CSRF token"):
            login("user@example.com", "pass")

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.CookieJar")
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.build_opener")
    def test_missing_session_cookie_raises(self, mock_build_opener, mock_jar_cls):
        jar = MagicMock()
        jar.__iter__ = MagicMock(return_value=iter([]))
        mock_jar_cls.return_value = jar
        mock_build_opener.return_value = _FakeOpener(
            jar, LOGIN_PAGE_HTML, '{"status":"ok"}', None
        )

        with pytest.raises(LoginError, match="cookie was not found"):
            login("user@example.com", "pass")

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.CookieJar")
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.build_opener")
    def test_wrong_credentials_detected(self, mock_build_opener, mock_jar_cls):
        jar = MagicMock()
        jar.__iter__ = MagicMock(return_value=iter([]))
        mock_jar_cls.return_value = jar
        mock_build_opener.return_value = _FakeOpener(
            jar, LOGIN_PAGE_HTML, "Invalid email or password", None
        )

        with pytest.raises(LoginError, match="Invalid credentials"):
            login("user@example.com", "wrongpass")


class TestLoginAsync:
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.CookieJar")
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.build_opener")
    def test_login_async_returns_ssid(self, mock_build_opener, mock_jar_cls):
        jar = MagicMock()
        jar.__iter__ = MagicMock(
            return_value=iter([_cookie("po_session", FAKE_SESSION)])
        )
        mock_jar_cls.return_value = jar
        mock_build_opener.return_value = _FakeOpener(
            jar, LOGIN_PAGE_HTML, '{"status":"ok"}', None
        )

        result = asyncio.run(login_async("user@example.com", "pass", demo=False))
        assert FAKE_SESSION in result
        assert result.startswith('42["auth",')


# ── Integration tests (need real env vars) ────────────────────────────────────


@pytest.mark.integration
class TestLoginIntegration:
    """
    These tests perform a real HTTP request to pocketoption.com.

    Set environment variables before running:
        POCKET_OPTION_EMAIL=your@email.com
        POCKET_OPTION_PASSWORD=yourpassword
    """

    @pytest.fixture(autouse=True)
    def _require_credentials(self):
        email = os.getenv("POCKET_OPTION_EMAIL")
        password = os.getenv("POCKET_OPTION_PASSWORD")
        if not email or not password:
            pytest.skip(
                "POCKET_OPTION_EMAIL and POCKET_OPTION_PASSWORD must be set for integration tests"
            )
        self.email = email
        self.password = password

    def test_login_returns_ssid_string(self):
        ssid = login(self.email, self.password, demo=True)
        print(f"\n[integration] SSID (first 60 chars): {ssid[:60]}...")
        assert ssid.startswith('42["auth",')
        assert '"session"' in ssid
        assert '"isDemo":1' in ssid

    def test_login_real_account(self):
        ssid = login(self.email, self.password, demo=False)
        assert '"isDemo":0' in ssid

    @pytest.mark.asyncio
    async def test_login_async(self):
        ssid = await login_async(self.email, self.password, demo=True)
        assert ssid.startswith('42["auth",')

    def test_login_ssid_can_connect(self):
        """Verify the obtained SSID actually connects to PocketOption WS."""
        try:
            from BinaryOptionsToolsV2.pocketoption import PocketOption
        except ImportError:
            pytest.skip("BinaryOptionsToolsV2 Rust extension not available")

        ssid = login(self.email, self.password, demo=True)
        config = {
            "connection_initialization_timeout_secs": 30,
            "max_allowed_loops": 0,
            "timeout_secs": 60,
            "terminal_logging": False,
            "log_level": "WARN",
        }
        import time

        with PocketOption(ssid, config=config) as client:
            time.sleep(5)
            connected = client.is_connected()
            print(f"\n[integration] is_connected after login: {connected}")
            assert connected, "Expected client to be connected with the new SSID"

    @pytest.mark.asyncio
    async def test_login_async_ssid_can_connect(self):
        """Async variant: verify the obtained SSID actually connects."""
        try:
            from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
        except ImportError:
            pytest.skip("BinaryOptionsToolsV2 Rust extension not available")

        ssid = await login_async(self.email, self.password, demo=True)
        config = {
            "connection_initialization_timeout_secs": 30,
            "max_allowed_loops": 0,
            "timeout_secs": 60,
            "terminal_logging": False,
            "log_level": "WARN",
        }
        async with PocketOptionAsync(ssid, config=config) as client:
            await asyncio.sleep(5)
            connected = client.is_connected()
            print(f"\n[integration] async is_connected after login: {connected}")
            assert connected


# ── Helpers ───────────────────────────────────────────────────────────────────


def _cookie(name: str, value: str):
    c = MagicMock()
    c.name = name
    c.value = value
    return c


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s", "-k", "not integration"])
