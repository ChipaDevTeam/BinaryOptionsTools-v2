"""
Tests for the email/password login flow in pocketoption.tools.login.

Unit tests mock the HTTP/browser layer — no network required.
Integration tests hit the real PocketOption site using Playwright.

Run unit tests only (default):
    pytest tests/python/pocketoption/test_login.py -v -k "not integration"

Run integration tests (needs real credentials + playwright install chromium):
    $env:POCKET_OPTION_EMAIL="you@example.com"
    $env:POCKET_OPTION_PASSWORD="yourpassword"
    pytest tests/python/pocketoption/test_login.py -v -k integration -s
"""

from __future__ import annotations

import asyncio
import os
import sys
from unittest.mock import MagicMock, patch

import pytest

_source = os.path.join(os.path.dirname(__file__), "../../../python")
if _source not in sys.path:
    sys.path.insert(0, _source)

from BinaryOptionsToolsV2.pocketoption.tools.login import (
    LoginError,
    _build_multipart,
    _find_session_cookie,
    login,
    login_async,
)


# ── Unit helpers ──────────────────────────────────────────────────────────────

FAKE_SESSION = "fakesessioncookievalue9999"


def _make_cookie(name: str, value: str) -> dict:
    return {"name": name, "value": value, "domain": ".pocketoption.com"}


# ── _build_multipart ──────────────────────────────────────────────────────────


class TestBuildMultipart:
    def test_contains_boundary(self):
        body = _build_multipart({"email": "a@b.com"}, "TESTBOUNDARY")
        assert b"--TESTBOUNDARY" in body
        assert b"--TESTBOUNDARY--" in body

    def test_contains_field_values(self):
        body = _build_multipart({"email": "a@b.com", "password": "secret"}, "B")
        assert b"a@b.com" in body
        assert b"secret" in body

    def test_content_disposition(self):
        body = _build_multipart({"myfield": "myvalue"}, "BOUND")
        assert b'name="myfield"' in body


# ── _find_session_cookie ──────────────────────────────────────────────────────


class TestFindSessionCookie:
    def test_finds_po_session(self):
        cookies = [
            _make_cookie("lang", "en"),
            _make_cookie("po_session", FAKE_SESSION),
            _make_cookie("other", "x"),
        ]
        assert _find_session_cookie(cookies) == FAKE_SESSION

    def test_returns_none_when_missing(self):
        assert _find_session_cookie([_make_cookie("lang", "en")]) is None

    def test_empty_list(self):
        assert _find_session_cookie([]) is None


# ── Playwright backend (mocked) ───────────────────────────────────────────────


def _make_playwright_mock(session: str | None = FAKE_SESSION, redirect_url: str = "https://pocketoption.com/en/cabinet/"):
    """Build a mock playwright sync_playwright context manager."""

    mock_cookie = _make_cookie("po_session", session) if session else _make_cookie("lang", "en")
    cookies_list = [mock_cookie]

    page = MagicMock()
    page.goto = MagicMock()
    page.fill = MagicMock()
    page.check = MagicMock()
    page.click = MagicMock()
    page.wait_for_url = MagicMock()
    page.locator.return_value.count.return_value = 0

    ctx = MagicMock()
    ctx.new_page.return_value = page
    ctx.cookies.return_value = cookies_list

    browser = MagicMock()
    browser.new_context.return_value = ctx
    browser.__enter__ = lambda s: s
    browser.__exit__ = MagicMock(return_value=False)

    chromium = MagicMock()
    chromium.launch.return_value = browser

    pw = MagicMock()
    pw.chromium = chromium
    pw.__enter__ = lambda s: s
    pw.__exit__ = MagicMock(return_value=False)

    sync_playwright_mock = MagicMock()
    sync_playwright_mock.return_value = pw
    return sync_playwright_mock


class TestLoginPlaywrightUnit:
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login.sync_playwright", create=True)
    def test_successful_login_demo(self, _):
        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
            return_value=FAKE_SESSION,
        ):
            result = login("user@example.com", "pass", demo=True, backend="playwright")
        assert result.startswith('42["auth",')
        assert FAKE_SESSION in result
        assert '"isDemo":1' in result

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
           return_value=FAKE_SESSION)
    def test_successful_login_real(self, _):
        result = login("user@example.com", "pass", demo=False, backend="playwright")
        assert '"isDemo":0' in result

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
           side_effect=LoginError("credentials rejected"))
    def test_login_error_propagates(self, _):
        with pytest.raises(LoginError, match="credentials rejected"):
            login("user@example.com", "wrongpass", backend="playwright")

    def test_unknown_backend_raises(self):
        with pytest.raises(ValueError, match="Unknown backend"):
            login("u@e.com", "p", backend="unknown")  # type: ignore

    def test_2captcha_without_api_key_raises(self):
        with pytest.raises(ValueError, match="api_key is required"):
            login("u@e.com", "p", backend="2captcha")


class TestLoginPlaywrightBrowserMock:
    """Verify _login_playwright wiring via module-level patching."""

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
           return_value=FAKE_SESSION)
    def test_session_cookie_extracted(self, _):
        result = login("u@e.com", "p", backend="playwright")
        assert FAKE_SESSION in result

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
           side_effect=LoginError("po_session cookie was not found"))
    def test_missing_session_cookie_raises(self, _):
        with pytest.raises(LoginError, match="po_session"):
            login("u@e.com", "p", backend="playwright")


# ── Async wrapper ─────────────────────────────────────────────────────────────


class TestLoginAsync:
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
           return_value=FAKE_SESSION)
    def test_async_returns_ssid(self, _):
        result = asyncio.run(login_async("u@e.com", "p", demo=False, backend="playwright"))
        assert FAKE_SESSION in result
        assert result.startswith('42["auth",')


# ── 2captcha backend (mocked) ─────────────────────────────────────────────────


class TestLogin2CaptchaMock:
    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_captcha_solver",
           return_value=FAKE_SESSION)
    def test_2captcha_backend_used(self, mock_solver):
        result = login("u@e.com", "p", backend="2captcha", api_key="testkey", demo=True)
        mock_solver.assert_called_once_with(
            "u@e.com", "p", api_key="testkey", service="2captcha", timeout=60
        )
        assert FAKE_SESSION in result
        assert '"isDemo":1' in result

    @patch("BinaryOptionsToolsV2.pocketoption.tools.login._login_captcha_solver",
           return_value=FAKE_SESSION)
    def test_capsolver_backend_used(self, mock_solver):
        result = login("u@e.com", "p", backend="capsolver", api_key="cs_key", demo=False)
        mock_solver.assert_called_once_with(
            "u@e.com", "p", api_key="cs_key", service="capsolver", timeout=60
        )
        assert FAKE_SESSION in result
        assert '"isDemo":0' in result


# ── Integration tests ─────────────────────────────────────────────────────────


@pytest.mark.integration
class TestLoginIntegration:
    """
    Real network tests against pocketoption.com using Playwright.

    Requirements:
        pip install playwright && playwright install chromium
        $env:POCKET_OPTION_EMAIL="your@email.com"
        $env:POCKET_OPTION_PASSWORD="yourpassword"
    """

    @pytest.fixture(autouse=True)
    def _require_credentials(self):
        email = os.getenv("POCKET_OPTION_EMAIL")
        password = os.getenv("POCKET_OPTION_PASSWORD")
        if not email or not password:
            pytest.skip("POCKET_OPTION_EMAIL and POCKET_OPTION_PASSWORD must be set")
        self.email = email
        self.password = password

    def test_login_returns_ssid_demo(self):
        ssid = login(self.email, self.password, demo=True)
        print(f"\n[integration] SSID[:80]: {ssid[:80]}...")
        assert ssid.startswith('42["auth",')
        assert '"session"' in ssid
        assert '"isDemo":1' in ssid

    def test_login_returns_ssid_real(self):
        ssid = login(self.email, self.password, demo=False)
        assert '"isDemo":0' in ssid

    @pytest.mark.asyncio
    async def test_login_async(self):
        ssid = await login_async(self.email, self.password, demo=True)
        assert ssid.startswith('42["auth",')

    def test_login_ssid_can_connect(self):
        """Verify the obtained SSID actually connects to the PocketOption WS."""
        try:
            from BinaryOptionsToolsV2.pocketoption import PocketOption
        except ImportError:
            pytest.skip("BinaryOptionsToolsV2 Rust extension not available")

        import time

        ssid = login(self.email, self.password, demo=True)
        config = {
            "connection_initialization_timeout_secs": 30,
            "max_allowed_loops": 0,
            "timeout_secs": 60,
            "terminal_logging": False,
            "log_level": "WARN",
        }
        with PocketOption(ssid, config=config) as client:
            time.sleep(5)
            connected = client.is_connected()
            print(f"\n[integration] is_connected: {connected}")
            assert connected

    @pytest.mark.asyncio
    async def test_login_async_ssid_can_connect(self):
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
            print(f"\n[integration] async is_connected: {connected}")
            assert connected


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s", "-k", "not integration"])
