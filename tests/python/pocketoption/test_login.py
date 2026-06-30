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

try:
    import playwright  # noqa: F401
except ImportError:
    # Playwright is not installed, mock it so that standard patch and calls don't crash
    from unittest.mock import MagicMock

    mock_playwright = MagicMock()
    mock_sync_api = MagicMock()

    class MockPWError(Exception):
        pass

    mock_sync_api.Error = MockPWError
    mock_sync_api.TimeoutError = MockPWError
    sys.modules["playwright"] = mock_playwright
    sys.modules["playwright.sync_api"] = mock_sync_api

from BinaryOptionsToolsV2.pocketoption.tools.login import (  # noqa: E402
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


def _make_playwright_mock(
    session: str | None = FAKE_SESSION,
    redirect_url: str = "https://pocketoption.com/en/cabinet/",
):
    """Build a mock playwright sync_playwright context manager."""

    mock_cookie = (
        _make_cookie("po_session", session) if session else _make_cookie("lang", "en")
    )
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

    firefox = MagicMock()
    firefox.launch.return_value = browser

    pw = MagicMock()
    pw.chromium = chromium
    pw.firefox = firefox
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

    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
        return_value=FAKE_SESSION,
    )
    def test_successful_login_real(self, _):
        result = login("user@example.com", "pass", demo=False, backend="playwright")
        assert '"isDemo":0' in result

    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
        side_effect=LoginError("credentials rejected"),
    )
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

    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
        return_value=FAKE_SESSION,
    )
    def test_session_cookie_extracted(self, _):
        result = login("u@e.com", "p", backend="playwright")
        assert FAKE_SESSION in result

    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
        side_effect=LoginError("po_session cookie was not found"),
    )
    def test_missing_session_cookie_raises(self, _):
        with pytest.raises(LoginError, match="po_session"):
            login("u@e.com", "p", backend="playwright")


# ── Async wrapper ─────────────────────────────────────────────────────────────


class TestLoginAsync:
    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_playwright",
        return_value=FAKE_SESSION,
    )
    def test_async_returns_ssid(self, _):
        result = asyncio.run(
            login_async("u@e.com", "p", demo=False, backend="playwright")
        )
        assert FAKE_SESSION in result
        assert result.startswith('42["auth",')


# ── 2captcha backend (mocked) ─────────────────────────────────────────────────


class TestLogin2CaptchaMock:
    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_captcha_solver",
        return_value=FAKE_SESSION,
    )
    def test_2captcha_backend_used(self, mock_solver):
        result = login("u@e.com", "p", backend="2captcha", api_key="testkey", demo=True)
        mock_solver.assert_called_once_with(
            "u@e.com", "p", api_key="testkey", service="2captcha", timeout=60
        )
        assert FAKE_SESSION in result
        assert '"isDemo":1' in result

    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_captcha_solver",
        return_value=FAKE_SESSION,
    )
    def test_capsolver_backend_used(self, mock_solver):
        result = login(
            "u@e.com", "p", backend="capsolver", api_key="cs_key", demo=False
        )
        mock_solver.assert_called_once_with(
            "u@e.com", "p", api_key="cs_key", service="capsolver", timeout=60
        )
        assert FAKE_SESSION in result
        assert '"isDemo":0' in result


# ── NoCaptchaAI backend (mocked) ─────────────────────────────────────────────


class TestLoginNoCaptchaAi:
    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_captcha_solver",
        return_value=FAKE_SESSION,
    )
    def test_nocaptchaai_backend_used(self, mock_solver):
        result = login(
            "u@e.com", "p", backend="nocaptchaai", api_key="nc_key", demo=True
        )
        mock_solver.assert_called_once_with(
            "u@e.com", "p", api_key="nc_key", service="nocaptchaai", timeout=60
        )
        assert FAKE_SESSION in result
        assert '"isDemo":1' in result

    @patch(
        "BinaryOptionsToolsV2.pocketoption.tools.login._login_captcha_solver",
        return_value=FAKE_SESSION,
    )
    def test_nocaptchaai_backend_demo_false(self, mock_solver):
        result = login(
            "u@e.com", "p", backend="nocaptchaai", api_key="nc_key", demo=False
        )
        mock_solver.assert_called_once_with(
            "u@e.com", "p", api_key="nc_key", service="nocaptchaai", timeout=60
        )
        assert FAKE_SESSION in result
        assert '"isDemo":0' in result


@patch("requests.post")
class TestSolveViaNoCaptchaAi:
    """Unit tests for _solve_via_nocaptchaai matching capsolver coverage."""

    def test_success(self, mock_post):
        create_resp = MagicMock()
        create_resp.json.return_value = {"errorId": 0, "taskId": "nc_task_123"}
        poll_resp = MagicMock()
        poll_resp.json.return_value = {
            "errorId": 0,
            "status": "ready",
            "solution": {"gRecaptchaResponse": "nc_token_value"},
        }
        mock_post.side_effect = [create_resp, poll_resp]

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_nocaptchaai

        token = _solve_via_nocaptchaai("api_key", timeout=10)
        assert token == "nc_token_value"

    def test_creation_error(self, mock_post):
        resp = MagicMock()
        resp.json.return_value = {"errorId": 1, "errorDescription": "Invalid API key"}
        mock_post.return_value = resp

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_nocaptchaai

        with pytest.raises(
            LoginError, match="NoCaptchaAI task creation failed: Invalid API key"
        ):
            _solve_via_nocaptchaai("api_key", timeout=10)

    def test_polling_error(self, mock_post):
        resp1 = MagicMock()
        resp1.json.return_value = {"errorId": 0, "taskId": "nc_task_456"}
        resp2 = MagicMock()
        resp2.json.return_value = {"errorId": 9, "errorDescription": "Task expired"}
        mock_post.side_effect = [resp1, resp2]

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_nocaptchaai

        with pytest.raises(LoginError, match="NoCaptchaAI error: Task expired"):
            _solve_via_nocaptchaai("api_key", timeout=10)

    def test_timeout(self, mock_post):
        resp1 = MagicMock()
        resp1.json.return_value = {"errorId": 0, "taskId": "nc_task_789"}
        resp2 = MagicMock()
        resp2.json.return_value = {"errorId": 0, "status": "processing"}
        mock_post.side_effect = [resp1, resp2, resp2, resp2, resp2, resp2]

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_nocaptchaai

        with patch("time.time", side_effect=[0, 1, 15]):
            with pytest.raises(LoginError, match="did not return a token within"):
                _solve_via_nocaptchaai("api_key", timeout=10)


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


class TestLoginInternalDetails:
    def test_capsolver_without_api_key_raises(self):
        with pytest.raises(ValueError, match="api_key is required"):
            login("u@e.com", "p", backend="capsolver")

    def test_browser_configs(self):
        from BinaryOptionsToolsV2.pocketoption.tools.login import _browser_configs

        mock_pw = MagicMock()
        configs = list(_browser_configs(mock_pw, True))
        assert len(configs) >= 2

    # Unit tests for _login_playwright
    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_success(self, mock_sync_pw):
        mock_pw = _make_playwright_mock(session=FAKE_SESSION)()
        mock_sync_pw.return_value = mock_pw
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        session = _login_playwright("u@e.com", "p", headless=True, timeout=10)
        assert session == FAKE_SESSION

    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_cookie_missing(self, mock_sync_pw):
        mock_pw = _make_playwright_mock(session=None)()
        mock_sync_pw.return_value = mock_pw
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        with pytest.raises(LoginError, match="cookie was not found"):
            _login_playwright("u@e.com", "p", headless=True, timeout=10)

    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_all_browsers_fail(self, mock_sync_pw):
        # Force browser launch to fail with PWError to cover the PWError branch
        from playwright.sync_api import Error as PWError

        pw_mock = MagicMock()
        pw_mock.firefox.launch.side_effect = PWError("firefox launch fail")
        pw_mock.chromium.launch.side_effect = PWError("chromium launch fail")
        pw_mock.__enter__ = lambda s: s
        pw_mock.__exit__ = MagicMock(return_value=False)
        mock_sync_pw.return_value = pw_mock
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        with pytest.raises(LoginError, match="All browser backends failed"):
            _login_playwright("u@e.com", "p", headless=True, timeout=10)

    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_pw_error_during_navigation(self, mock_sync_pw):
        from playwright.sync_api import Error as PWError

        mock_pw = _make_playwright_mock(session=FAKE_SESSION)()
        page = mock_pw.chromium.launch.return_value.new_context.return_value.new_page.return_value
        page.goto.side_effect = PWError("mock playwright navigation error")
        mock_pw.firefox.launch.side_effect = Exception("force firefox fallback")
        mock_sync_pw.return_value = mock_pw
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        with pytest.raises(LoginError, match="All browser backends failed"):
            _login_playwright("u@e.com", "p", headless=True, timeout=10)

    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_remember_checkbox_fail(self, mock_sync_pw):
        # Test that remember checkbox page.check raising exception is ignored (pass)
        mock_pw = _make_playwright_mock(session=FAKE_SESSION)()
        mock_pw.chromium.launch.return_value.new_context.return_value.new_page.return_value.check.side_effect = Exception(
            "checkbox not found"
        )
        # Ensure we fall back to chromium and trigger page.check error
        mock_pw.firefox.launch.side_effect = Exception("force firefox fallback")
        mock_sync_pw.return_value = mock_pw
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        session = _login_playwright("u@e.com", "p", headless=True, timeout=10)
        assert session == FAKE_SESSION

    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_wait_url_timeout(self, mock_sync_pw):
        from playwright.sync_api import TimeoutError as PWTimeout

        mock_pw = _make_playwright_mock(session=FAKE_SESSION)()
        page = mock_pw.chromium.launch.return_value.new_context.return_value.new_page.return_value
        page.wait_for_url.side_effect = PWTimeout("mock timeout")
        # Ensure error element count > 0 to test error text retrieval
        err_els = MagicMock()
        err_els.count.return_value = 1
        err_els.first.text_content.return_value = "Mock Page Error Alert"
        page.locator.return_value = err_els

        # Force firefox fallback to chromium
        mock_pw.firefox.launch.side_effect = Exception("force firefox fallback")
        mock_sync_pw.return_value = mock_pw

        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        with pytest.raises(LoginError, match="Page says: Mock Page Error Alert"):
            _login_playwright("u@e.com", "p", headless=True, timeout=10)

    @patch("playwright.sync_api.sync_playwright", create=True)
    def test_login_playwright_internal_wait_url_timeout_no_alert(self, mock_sync_pw):
        from playwright.sync_api import TimeoutError as PWTimeout

        mock_pw = _make_playwright_mock(session=FAKE_SESSION)()
        page = mock_pw.chromium.launch.return_value.new_context.return_value.new_page.return_value
        page.wait_for_url.side_effect = PWTimeout("mock timeout")
        page.locator.return_value.count.return_value = 0

        mock_pw.firefox.launch.side_effect = Exception("force firefox fallback")
        mock_sync_pw.return_value = mock_pw

        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

        with pytest.raises(LoginError) as exc_info:
            _login_playwright("u@e.com", "p", headless=True, timeout=10)
        assert "Page says:" not in str(exc_info.value)

    # Unit tests for requests capsolver/2captcha backend
    @patch("requests.Session")
    def test_login_captcha_solver_success_capsolver(self, mock_session_class):
        mock_sess = MagicMock()
        mock_session_class.return_value = mock_sess

        # GET response
        get_resp = MagicMock()
        get_resp.text = 'register_page = "123"'
        mock_sess.get.return_value = get_resp

        # POST response
        post_resp = MagicMock()
        post_resp.json.return_value = {"status": True}
        mock_sess.post.return_value = post_resp
        mock_sess.cookies.get.return_value = FAKE_SESSION

        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_captcha_solver

        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._solve_via_capsolver",
            return_value="mock_token",
        ) as mock_solve:
            session = _login_captcha_solver(
                "u@e.com", "p", api_key="k", service="capsolver", timeout=10
            )
            assert session == FAKE_SESSION
            mock_solve.assert_called_once_with("k", timeout=10)

    @patch("requests.Session")
    def test_login_captcha_solver_success_2captcha(self, mock_session_class):
        mock_sess = MagicMock()
        mock_session_class.return_value = mock_sess
        get_resp = MagicMock()
        get_resp.text = 'register_page = "123"'
        mock_sess.get.return_value = get_resp
        post_resp = MagicMock()
        post_resp.json.return_value = {"status": True}
        mock_sess.post.return_value = post_resp
        mock_sess.cookies.get.return_value = FAKE_SESSION
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_captcha_solver

        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._solve_via_2captcha",
            return_value="mock_token",
        ) as mock_solve:
            session = _login_captcha_solver(
                "u@e.com", "p", api_key="k", service="2captcha", timeout=10
            )
            assert session == FAKE_SESSION
            mock_solve.assert_called_once_with("k", timeout=10)

    @patch("requests.Session")
    def test_login_captcha_solver_no_session_cookie_generic(self, mock_session_class):
        mock_sess = MagicMock()
        mock_session_class.return_value = mock_sess
        get_resp = MagicMock()
        get_resp.text = ""
        mock_sess.get.return_value = get_resp
        post_resp = MagicMock()
        post_resp.json.side_effect = ValueError("no json")
        post_resp.status_code = 200
        post_resp.text = "Generic response without error keywords"
        mock_sess.post.return_value = post_resp
        mock_sess.cookies.get.return_value = None
        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_captcha_solver

        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._solve_via_capsolver",
            return_value="mock_token",
        ):
            with pytest.raises(LoginError, match="cookie was not set"):
                _login_captcha_solver(
                    "u@e.com", "p", api_key="k", service="capsolver", timeout=10
                )

    @patch("requests.Session")
    def test_login_captcha_solver_server_error(self, mock_session_class):
        mock_sess = MagicMock()
        mock_session_class.return_value = mock_sess

        # GET response
        get_resp = MagicMock()
        get_resp.text = 'register_page = "123"'
        mock_sess.get.return_value = get_resp

        # POST response (returns status False with error message)
        post_resp = MagicMock()
        post_resp.json.return_value = {"status": False, "error": "Invalid email format"}
        mock_sess.post.return_value = post_resp
        mock_sess.cookies.get.return_value = None

        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_captcha_solver

        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._solve_via_capsolver",
            return_value="mock_token",
        ):
            with pytest.raises(
                LoginError, match="Server rejected login: Invalid email format"
            ):
                _login_captcha_solver(
                    "u@e.com", "p", api_key="k", service="capsolver", timeout=10
                )

    @patch("requests.Session")
    def test_login_captcha_solver_no_session_cookie(self, mock_session_class):
        mock_sess = MagicMock()
        mock_session_class.return_value = mock_sess

        get_resp = MagicMock()
        get_resp.text = ""
        mock_sess.get.return_value = get_resp

        post_resp = MagicMock()
        # Non-JSON or AttributeError
        post_resp.json.side_effect = ValueError("no json")
        post_resp.status_code = 403
        post_resp.text = "Incorrect password"
        mock_sess.post.return_value = post_resp
        mock_sess.cookies.get.return_value = None

        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_captcha_solver

        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._solve_via_capsolver",
            return_value="mock_token",
        ):
            with pytest.raises(
                LoginError, match="Invalid credentials: server rejected"
            ):
                _login_captcha_solver(
                    "u@e.com", "p", api_key="k", service="capsolver", timeout=10
                )

    @patch("requests.Session")
    def test_login_captcha_solver_captcha_error_response(self, mock_session_class):
        mock_sess = MagicMock()
        mock_session_class.return_value = mock_sess

        get_resp = MagicMock()
        get_resp.text = ""
        mock_sess.get.return_value = get_resp

        post_resp = MagicMock()
        post_resp.json.side_effect = ValueError("no json")
        post_resp.status_code = 200
        post_resp.text = "Captcha verification failed"
        mock_sess.post.return_value = post_resp
        mock_sess.cookies.get.return_value = None

        from BinaryOptionsToolsV2.pocketoption.tools.login import _login_captcha_solver

        with patch(
            "BinaryOptionsToolsV2.pocketoption.tools.login._solve_via_capsolver",
            return_value="mock_token",
        ):
            with pytest.raises(LoginError, match="Login blocked by CAPTCHA"):
                _login_captcha_solver(
                    "u@e.com", "p", api_key="k", service="capsolver", timeout=10
                )

    @patch("requests.post")
    def test_solve_via_capsolver_success(self, mock_post):
        # CreateTask response
        resp1 = MagicMock()
        resp1.json.return_value = {"errorId": 0, "taskId": "task_123"}
        # GetTaskResult response
        resp2 = MagicMock()
        resp2.json.return_value = {
            "errorId": 0,
            "status": "ready",
            "solution": {"gRecaptchaResponse": "capsolver_token_value"},
        }
        mock_post.side_effect = [resp1, resp2]

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_capsolver

        token = _solve_via_capsolver("api_key", timeout=10)
        assert token == "capsolver_token_value"

    @patch("requests.post")
    def test_solve_via_capsolver_creation_error(self, mock_post):
        resp = MagicMock()
        resp.json.return_value = {"errorId": 1, "errorDescription": "Invalid API key"}
        mock_post.return_value = resp

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_capsolver

        with pytest.raises(
            LoginError, match="CapSolver task creation failed: Invalid API key"
        ):
            _solve_via_capsolver("api_key", timeout=10)

    @patch("requests.post")
    def test_solve_via_capsolver_polling_error(self, mock_post):
        resp1 = MagicMock()
        resp1.json.return_value = {"errorId": 0, "taskId": "task_123"}
        resp2 = MagicMock()
        resp2.json.return_value = {"errorId": 9, "errorDescription": "Task expired"}
        mock_post.side_effect = [resp1, resp2]

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_capsolver

        with pytest.raises(LoginError, match="CapSolver error: Task expired"):
            _solve_via_capsolver("api_key", timeout=10)

    @patch("requests.post")
    def test_solve_via_capsolver_timeout(self, mock_post):
        resp1 = MagicMock()
        resp1.json.return_value = {"errorId": 0, "taskId": "task_123"}
        resp2 = MagicMock()
        resp2.json.return_value = {"errorId": 0, "status": "processing"}
        mock_post.side_effect = [resp1, resp2, resp2, resp2, resp2, resp2]

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_capsolver

        with patch("time.time", side_effect=[0, 1, 15]):
            with pytest.raises(LoginError, match="did not return a token within"):
                _solve_via_capsolver("api_key", timeout=10)

    @patch("requests.post")
    @patch("requests.get")
    def test_solve_via_2captcha_success(self, mock_get, mock_post):
        # in.php response
        resp1 = MagicMock()
        resp1.json.return_value = {"status": 1, "request": "task_456"}
        mock_post.return_value = resp1
        # res.php response
        resp2 = MagicMock()
        resp2.json.return_value = {"status": 1, "request": "2captcha_token_value"}
        mock_get.return_value = resp2

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_2captcha

        token = _solve_via_2captcha("api_key", timeout=10)
        assert token == "2captcha_token_value"

    @patch("requests.post")
    def test_solve_via_2captcha_submission_error(self, mock_post):
        resp = MagicMock()
        resp.json.return_value = {"status": 0, "request": "ERROR_KEY_DOES_NOT_EXIST"}
        mock_post.return_value = resp

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_2captcha

        with pytest.raises(LoginError, match="2captcha submission failed"):
            _solve_via_2captcha("api_key", timeout=10)

    @patch("requests.post")
    @patch("requests.get")
    def test_solve_via_2captcha_polling_error(self, mock_get, mock_post):
        resp1 = MagicMock()
        resp1.json.return_value = {"status": 1, "request": "task_456"}
        mock_post.return_value = resp1
        resp2 = MagicMock()
        resp2.json.return_value = {"status": 0, "request": "ERROR_WRONG_USER_KEY"}
        mock_get.return_value = resp2

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_2captcha

        with pytest.raises(LoginError, match="2captcha error"):
            _solve_via_2captcha("api_key", timeout=10)

    @patch("requests.post")
    @patch("requests.get")
    def test_solve_via_2captcha_timeout(self, mock_get, mock_post):
        resp1 = MagicMock()
        resp1.json.return_value = {"status": 1, "request": "task_456"}
        mock_post.return_value = resp1
        resp2 = MagicMock()
        resp2.json.return_value = {"status": 0, "request": "CAPTCHA_NOT_READY"}
        mock_get.return_value = resp2

        from BinaryOptionsToolsV2.pocketoption.tools.login import _solve_via_2captcha

        with patch("time.time", side_effect=[0, 1, 15]):
            with pytest.raises(LoginError, match="did not return a token within"):
                _solve_via_2captcha("api_key", timeout=10)

    # Test requests/playwright library missing
    def test_requests_import_error(self):
        from unittest.mock import patch

        with patch.dict("sys.modules", {"requests": None}):
            from BinaryOptionsToolsV2.pocketoption.tools.login import (
                _login_captcha_solver,
            )

            with pytest.raises(ImportError, match="requests is required"):
                _login_captcha_solver(
                    "u@e.com", "p", api_key="k", service="capsolver", timeout=10
                )

    def test_playwright_import_error(self):
        from unittest.mock import patch

        with patch.dict("sys.modules", {"playwright.sync_api": None}):
            from BinaryOptionsToolsV2.pocketoption.tools.login import _login_playwright

            with pytest.raises(ImportError, match="playwright is required"):
                _login_playwright("u@e.com", "p", headless=True, timeout=10)


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s", "-k", "not integration"])
