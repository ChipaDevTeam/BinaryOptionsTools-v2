"""
Login module for PocketOption — obtain a session SSID from email/password.

Three backends are available:

* ``"capsolver"`` — uses the CapSolver API (free tier at capsolver.com) to solve
  reCAPTCHA v3, then submits the form via plain HTTP requests.  Best choice when
  browser processes are blocked by a firewall.  Requires ``api_key`` and the
  ``requests`` package.

* ``"2captcha"`` — same approach but uses the 2captcha.com service instead of
  CapSolver.  Requires ``api_key`` and the ``requests`` package.

* ``"playwright"`` — launches a headless browser (Firefox → Chromium → system
  Chrome) that fills the form and handles reCAPTCHA v3 automatically.  Requires
  ``pip install playwright && playwright install firefox chromium``.  Useful when
  a captcha solver API key is not available.

* ``"auto"`` (default) — tries ``playwright`` first; if every browser backend
  fails with a network error, raises ``LoginError`` with instructions to use
  the captcha-solver backends.

Usage::

    # With CapSolver (recommended when browsers are firewall-blocked)
    from BinaryOptionsToolsV2.pocketoption.tools.login import login
    ssid = login("you@example.com", "password", demo=True,
                 backend="capsolver", api_key="YOUR_CAPSOLVER_KEY")

    # With Playwright headless browser
    ssid = login("you@example.com", "password", demo=True)
"""

from __future__ import annotations

import re
import time
import uuid
from typing import Literal, Optional

BASE_URL = "https://pocketoption.com"
LOGIN_URL = BASE_URL + "/en/login/"
RECAPTCHA_SITEKEY = "6LeJDkwpAAAAAFUuiKS66HQe6Jz-Z-uPp5Dl6q5B"

_DEFAULT_UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/146.0.0.0 Safari/537.36"
)

_REGISTER_PAGE_RE = re.compile(
    r'name=["\']register_page["\'][^>]+value=["\']([^"\']+)["\']'
    r'|value=["\']([^"\']+)["\'][^>]+name=["\']register_page["\']',
    re.IGNORECASE,
)


# ── Public API ─────────────────────────────────────────────────────────────────


def login(
    email: str,
    password: str,
    *,
    demo: bool = False,
    backend: Literal["auto", "playwright", "capsolver", "2captcha"] = "auto",
    api_key: Optional[str] = None,
    headless: bool = True,
    timeout: int = 60,
) -> str:
    """Login to PocketOption and return the SSID string.

    Args:
        email: Account e-mail address.
        password: Account password.
        demo: If True, the SSID targets the demo account.
        backend: Which login method to use (see module docstring).
            ``"auto"`` tries playwright and gives a clear error if it fails.
        api_key: CapSolver or 2captcha API key (required for those backends).
        headless: Run the browser in headless mode (playwright only).
        timeout: Overall timeout in seconds.

    Returns:
        SSID string ``42["auth",{...}]`` ready for PocketOptionAsync.

    Raises:
        LoginError: Credentials rejected or session cookie not found.
        ValueError: Missing required argument (e.g. api_key).
        ImportError: Required backend library not installed.
    """
    if backend in ("auto", "playwright"):
        session = _login_playwright(email, password, headless=headless, timeout=timeout)
    elif backend == "capsolver":
        if not api_key:
            raise ValueError("api_key is required when backend='capsolver'")
        session = _login_captcha_solver(
            email, password, api_key=api_key, service="capsolver", timeout=timeout
        )
    elif backend == "2captcha":
        if not api_key:
            raise ValueError("api_key is required when backend='2captcha'")
        session = _login_captcha_solver(
            email, password, api_key=api_key, service="2captcha", timeout=timeout
        )
    else:
        raise ValueError(f"Unknown backend: {backend!r}")

    is_demo_int = 1 if demo else 0
    return (
        f'42["auth",{{"session":"{session}",'
        f'"isDemo":{is_demo_int},"uid":0,"platform":2}}]'
    )


async def login_async(
    email: str,
    password: str,
    *,
    demo: bool = False,
    backend: Literal["auto", "playwright", "capsolver", "2captcha"] = "auto",
    api_key: Optional[str] = None,
    headless: bool = True,
    timeout: int = 60,
) -> str:
    """Async version of :func:`login` — runs blocking I/O in a thread executor."""
    import asyncio
    import functools

    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(
        None,
        functools.partial(
            login,
            email,
            password,
            demo=demo,
            backend=backend,
            api_key=api_key,
            headless=headless,
            timeout=timeout,
        ),
    )


# ── Playwright backend ─────────────────────────────────────────────────────────


def _login_playwright(email: str, password: str, *, headless: bool, timeout: int) -> str:
    """Use a real browser to log in and return the po_session cookie value.

    Tries Firefox → Chromium → system Chrome in order.
    """
    try:
        from playwright.sync_api import Error as PWError
        from playwright.sync_api import TimeoutError as PWTimeout
        from playwright.sync_api import sync_playwright
    except ImportError as exc:
        raise ImportError(
            "playwright is required for the 'playwright' backend.\n"
            "Install it with:  pip install playwright\n"
            "Then:             py -3 -m playwright install firefox chromium"
        ) from exc

    last_error: Exception = RuntimeError("no browser attempted")
    with sync_playwright() as pw:
        for browser_type, launch_kwargs, ctx_kwargs in _browser_configs(pw, headless):
            try:
                browser = browser_type.launch(**launch_kwargs)
            except Exception as exc:
                last_error = exc
                continue

            ctx = browser.new_context(**ctx_kwargs)
            ctx.add_init_script(
                "Object.defineProperty(navigator, 'webdriver', {get: () => undefined})"
            )
            page = ctx.new_page()
            try:
                page.goto(LOGIN_URL, wait_until="domcontentloaded", timeout=timeout * 1000)
                page.fill('input[name="email"]', email)
                page.fill('input[name="password"]', password)
                try:
                    page.check('input[name="remember"]', timeout=2000)
                except Exception:
                    pass

                page.click('button[type="submit"], input[type="submit"]')

                try:
                    page.wait_for_url(
                        lambda url: "/login/" not in url,
                        timeout=timeout * 1000,
                    )
                except PWTimeout:
                    err_els = page.locator(".error, .alert, .form-error")
                    err_text = (
                        err_els.first.text_content(timeout=2000)
                        if err_els.count() > 0
                        else ""
                    )
                    raise LoginError(
                        "Login did not redirect — credentials may be wrong or CAPTCHA blocked."
                        + (f" Page says: {err_text}" if err_text else "")
                    )

                session_value = _find_session_cookie(ctx.cookies())
                if not session_value:
                    raise LoginError(
                        "Login redirected but 'po_session' cookie was not found."
                    )
                return session_value

            except LoginError:
                raise
            except PWError as exc:
                last_error = exc
                continue
            finally:
                browser.close()

    raise LoginError(
        f"All browser backends failed to reach {LOGIN_URL}.\n"
        f"Last error: {last_error}\n\n"
        "Your browser processes appear to be blocked by a firewall or security\n"
        "software.  Use a captcha-solver backend instead:\n\n"
        "  1. Get a FREE CapSolver key at https://capsolver.com  (no credit card)\n"
        "  2. Call:  login(email, password, backend='capsolver', api_key='YOUR_KEY')\n\n"
        "Or use 2captcha.com (paid) with backend='2captcha'."
    )


def _browser_configs(pw, headless: bool):
    """Yield (browser_type, launch_kwargs, context_kwargs) in order to try."""
    common_ctx = {
        "user_agent": _DEFAULT_UA,
        "locale": "en-US",
        "timezone_id": "America/New_York",
        "viewport": {"width": 1366, "height": 768},
        "extra_http_headers": {"Accept-Language": "en-US,en;q=0.9"},
    }
    yield (
        pw.firefox,
        {"headless": headless},
        common_ctx,
    )
    yield (
        pw.chromium,
        {
            "headless": headless,
            "args": [
                "--disable-blink-features=AutomationControlled",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--lang=en-US,en",
            ],
        },
        {
            **common_ctx,
            "extra_http_headers": {
                **common_ctx["extra_http_headers"],
                "sec-ch-ua": '"Not-A.Brand";v="24", "Chromium";v="146"',
                "sec-ch-ua-mobile": "?0",
                "sec-ch-ua-platform": '"Windows"',
            },
        },
    )
    yield (
        pw.chromium,
        {
            "headless": headless,
            "channel": "chrome",
            "args": ["--disable-blink-features=AutomationControlled"],
        },
        common_ctx,
    )


def _find_session_cookie(cookies: list[dict]) -> Optional[str]:
    for c in cookies:
        if c.get("name") == "po_session":
            return c.get("value")
    return None


# ── Captcha-solver HTTP backend (CapSolver + 2captcha) ─────────────────────────


def _login_captcha_solver(
    email: str,
    password: str,
    *,
    api_key: str,
    service: Literal["capsolver", "2captcha"],
    timeout: int,
) -> str:
    """Solve reCAPTCHA v3 via a solver API then POST credentials over HTTP."""
    try:
        import requests as req
    except ImportError as exc:
        raise ImportError(
            "requests is required for captcha-solver backends.\n"
            "Install it with: pip install requests"
        ) from exc

    s = req.Session()
    s.headers.update({"User-Agent": _DEFAULT_UA, "Accept-Language": "en-US,en;q=0.9"})

    # Step 1: GET login page to collect cookies and register_page value
    r = s.get(LOGIN_URL, timeout=30)
    r.raise_for_status()
    m = _REGISTER_PAGE_RE.search(r.text)
    register_page = (m.group(1) or m.group(2)) if m else "0"

    # Step 2: Solve reCAPTCHA v3
    if service == "capsolver":
        captcha_token = _solve_via_capsolver(api_key, timeout=timeout)
    else:
        captcha_token = _solve_via_2captcha(api_key, timeout=timeout)

    # Step 3: POST the login form
    boundary = "----WebKitFormBoundary" + uuid.uuid4().hex[:16].upper()
    fields = {
        "submitLogin": "1",
        "email": email,
        "password": password,
        "remember": "1",
        "g-recaptcha-response": "",
        "register_page": register_page,
        "token": captcha_token,
    }
    body = _build_multipart(fields, boundary)

    resp = s.post(
        LOGIN_URL,
        data=body,
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Content-Length": str(len(body)),
            "Accept": "application/json, text/javascript, */*; q=0.01",
            "X-Requested-With": "XMLHttpRequest",
            "Origin": BASE_URL,
            "Referer": LOGIN_URL,
            "Sec-Fetch-Site": "same-origin",
            "Sec-Fetch-Mode": "cors",
            "Sec-Fetch-Dest": "empty",
        },
        timeout=30,
        allow_redirects=False,
    )

    # Step 4: Check for server-side errors in JSON response
    try:
        data = resp.json()
        if data.get("status") is False:
            err = data.get("error", {})
            raise LoginError(f"Server rejected login: {err}")
    except (ValueError, AttributeError):
        pass

    # Step 5: Extract session cookie
    session_value = s.cookies.get("po_session")
    if not session_value:
        _check_response_for_errors(resp.text)
        raise LoginError(
            f"Login request returned HTTP {resp.status_code} but 'po_session' "
            "cookie was not set. Check your credentials."
        )
    return session_value


def _solve_via_capsolver(api_key: str, *, timeout: int) -> str:
    """Submit a ReCaptchaV3TaskProxyless task to CapSolver and return the token."""
    import requests as req

    submit = req.post(
        "https://api.capsolver.com/createTask",
        json={
            "clientKey": api_key,
            "task": {
                "type": "ReCaptchaV3TaskProxyless",
                "websiteURL": LOGIN_URL,
                "websiteKey": RECAPTCHA_SITEKEY,
                "pageAction": "login",
                "minScore": 0.5,
            },
        },
        timeout=30,
    )
    result = submit.json()
    if result.get("errorId") != 0:
        raise LoginError(
            f"CapSolver task creation failed: {result.get('errorDescription', result)}\n"
            "Get a free API key at https://capsolver.com"
        )
    task_id = result["taskId"]

    deadline = time.time() + timeout
    while time.time() < deadline:
        time.sleep(3)
        poll = req.post(
            "https://api.capsolver.com/getTaskResult",
            json={"clientKey": api_key, "taskId": task_id},
            timeout=30,
        )
        data = poll.json()
        if data.get("errorId") != 0:
            raise LoginError(f"CapSolver error: {data.get('errorDescription', data)}")
        if data.get("status") == "ready":
            return data["solution"]["gRecaptchaResponse"]

    raise LoginError(f"CapSolver did not return a token within {timeout}s")


def _solve_via_2captcha(api_key: str, *, timeout: int) -> str:
    """Submit a reCAPTCHA v3 task to 2captcha and return the token."""
    import requests as req

    submit = req.post(
        "https://2captcha.com/in.php",
        data={
            "key": api_key,
            "method": "userrecaptcha",
            "googlekey": RECAPTCHA_SITEKEY,
            "pageurl": LOGIN_URL,
            "version": "v3",
            "action": "login",
            "min_score": "0.5",
            "json": "1",
        },
        timeout=30,
    )
    result = submit.json()
    if result.get("status") != 1:
        raise LoginError(f"2captcha submission failed: {result}")
    task_id = result["request"]

    deadline = time.time() + timeout
    while time.time() < deadline:
        time.sleep(5)
        poll = req.get(
            f"https://2captcha.com/res.php?key={api_key}&action=get&id={task_id}&json=1",
            timeout=30,
        )
        data = poll.json()
        if data.get("status") == 1:
            return data["request"]
        if data.get("request") not in ("CAPCHA_NOT_READY", "CAPTCHA_NOT_READY"):
            raise LoginError(f"2captcha error: {data}")

    raise LoginError(f"2captcha did not return a token within {timeout}s")


# ── Shared helpers ─────────────────────────────────────────────────────────────


def _build_multipart(fields: dict[str, str], boundary: str) -> bytes:
    parts: list[bytes] = []
    sep = f"--{boundary}".encode()
    for name, value in fields.items():
        parts.append(sep)
        parts.append(
            f'Content-Disposition: form-data; name="{name}"\r\n\r\n{value}'.encode()
        )
    parts.append(f"--{boundary}--".encode())
    return b"\r\n".join(parts)


def _check_response_for_errors(body: str) -> None:
    lower = body.lower()
    if "invalid" in lower or "incorrect" in lower or "wrong" in lower:
        raise LoginError("Invalid credentials: server rejected the email/password.")
    if "captcha" in lower:
        raise LoginError("Login blocked by CAPTCHA — the solver token may be stale.")


class LoginError(RuntimeError):
    """Raised when authentication fails."""
