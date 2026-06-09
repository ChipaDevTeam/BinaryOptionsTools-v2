"""
Login module for PocketOption — obtain a session SSID from email/password.

Two backends are available:

* ``"playwright"`` (default) — launches a headless Chromium browser that
  fills the login form and clicks submit.  reCAPTCHA v3 is handled
  automatically by the real browser engine.  Requires the ``playwright``
  package (``pip install playwright && playwright install chromium``).

* ``"2captcha"`` — uses the 2captcha API to solve the reCAPTCHA v3 token
  and then submits the form via plain HTTP.  Requires a ``api_key``
  argument and the ``requests`` package.

Usage::

    from BinaryOptionsToolsV2.pocketoption.tools.login import login

    ssid = login("you@example.com", "password", demo=True)
    # ssid == '42["auth",{"session":"...","isDemo":1,...}]'
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
    backend: Literal["playwright", "2captcha"] = "playwright",
    api_key: Optional[str] = None,
    headless: bool = True,
    timeout: int = 60,
) -> str:
    """Login to PocketOption and return the SSID string.

    Args:
        email: Account e-mail address.
        password: Account password.
        demo: If True, the SSID targets the demo account.
        backend: ``"playwright"`` (default) uses a headless browser;
                 ``"2captcha"`` uses the 2captcha API (needs ``api_key``).
        api_key: 2captcha API key — only used when ``backend="2captcha"``.
        headless: Run the browser in headless mode (playwright only).
        timeout: Overall timeout in seconds.

    Returns:
        SSID string ``42["auth",{...}]`` ready for PocketOptionAsync.

    Raises:
        LoginError: Credentials rejected or session cookie not found.
        ImportError: Required backend library not installed.
    """
    if backend == "playwright":
        session = _login_playwright(email, password, headless=headless, timeout=timeout)
    elif backend == "2captcha":
        if not api_key:
            raise ValueError("api_key is required when backend='2captcha'")
        session = _login_2captcha(email, password, api_key=api_key, timeout=timeout)
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
    backend: Literal["playwright", "2captcha"] = "playwright",
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
    """Use a real Chromium browser to log in and return the session value."""
    try:
        from playwright.sync_api import sync_playwright, TimeoutError as PWTimeout
    except ImportError as exc:
        raise ImportError(
            "playwright is required for the 'playwright' backend.\n"
            "Install it with: pip install playwright && playwright install chromium"
        ) from exc

    with sync_playwright() as pw:
        browser = pw.chromium.launch(headless=headless)
        ctx = browser.new_context(
            user_agent=_DEFAULT_UA,
            locale="en-GB",
            extra_http_headers={"Accept-Language": "en-GB,en;q=0.9"},
        )
        page = ctx.new_page()

        try:
            page.goto(LOGIN_URL, wait_until="domcontentloaded", timeout=timeout * 1000)
            page.fill('input[name="email"]', email)
            page.fill('input[name="password"]', password)
            # Check the remember-me checkbox if present
            try:
                page.check('input[name="remember"]', timeout=2000)
            except Exception:
                pass

            page.click('button[type="submit"], input[type="submit"]')

            # Wait for either a redirect away from /login/ or an error message
            try:
                page.wait_for_url(
                    lambda url: "/login/" not in url,
                    timeout=timeout * 1000,
                )
            except PWTimeout:
                # Check for an error message on the page
                err_text = page.locator(".error, .alert, .form-error").first.text_content(timeout=2000) if page.locator(".error, .alert, .form-error").count() > 0 else ""
                raise LoginError(
                    f"Login did not redirect away from /login/ within {timeout}s. "
                    + (f"Page error: {err_text}" if err_text else "Credentials may be wrong or CAPTCHA blocked.")
                )

            # Grab po_session cookie
            cookies = ctx.cookies()
            session_value = _find_session_cookie(cookies)
            if not session_value:
                raise LoginError(
                    "Login appeared to succeed (redirected) but 'po_session' cookie "
                    "was not found in the browser context."
                )
            return session_value

        finally:
            browser.close()


def _find_session_cookie(cookies: list[dict]) -> Optional[str]:
    for c in cookies:
        if c.get("name") == "po_session":
            return c.get("value")
    return None


# ── 2captcha backend ───────────────────────────────────────────────────────────


def _login_2captcha(email: str, password: str, *, api_key: str, timeout: int) -> str:
    """Solve reCAPTCHA v3 via 2captcha, then POST credentials via HTTP."""
    try:
        import requests
    except ImportError as exc:
        raise ImportError(
            "requests is required for the '2captcha' backend.\n"
            "Install it with: pip install requests"
        ) from exc

    s = requests.Session()
    s.headers.update({"User-Agent": _DEFAULT_UA, "Accept-Language": "en-GB,en;q=0.9"})

    # Step 1: GET login page — collect cookies and register_page
    r = s.get(LOGIN_URL, timeout=30)
    r.raise_for_status()
    html = r.text

    m = _REGISTER_PAGE_RE.search(html)
    register_page = (m.group(1) or m.group(2)) if m else "0"

    # Step 2: Submit reCAPTCHA v3 task to 2captcha
    captcha_token = _solve_recaptcha_v3(api_key, pageurl=LOGIN_URL, timeout=timeout)

    # Step 3: POST login form
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

    # Step 4: Try to parse response
    try:
        data = resp.json()
        if not data.get("status", True) is False:
            pass  # success branch (status True or missing)
        else:
            err = data.get("error", {})
            raise LoginError(f"Server rejected login: {err}")
    except (ValueError, AttributeError):
        pass

    # Step 5: Extract session cookie
    session_value = s.cookies.get("po_session")
    if not session_value:
        _check_response_for_errors(resp.text)
        raise LoginError(
            "Login appeared to succeed but 'po_session' cookie was not found. "
            f"Response status: {resp.status_code}"
        )
    return session_value


def _solve_recaptcha_v3(api_key: str, pageurl: str, timeout: int) -> str:
    """Submit a reCAPTCHA v3 task to 2captcha and return the token."""
    import requests

    # Submit task
    submit = requests.post(
        "https://2captcha.com/in.php",
        data={
            "key": api_key,
            "method": "userrecaptcha",
            "googlekey": RECAPTCHA_SITEKEY,
            "pageurl": pageurl,
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

    # Poll for result
    deadline = time.time() + timeout
    while time.time() < deadline:
        time.sleep(5)
        poll = requests.get(
            f"https://2captcha.com/res.php?key={api_key}&action=get&id={task_id}&json=1",
            timeout=30,
        )
        data = poll.json()
        if data.get("status") == 1:
            return data["request"]
        if data.get("request") not in ("CAPCHA_NOT_READY", "CAPTCHA_NOT_READY"):
            raise LoginError(f"2captcha error: {data}")

    raise LoginError(f"2captcha did not return a token within {timeout}s")


# ── Helpers ────────────────────────────────────────────────────────────────────


def _build_multipart(fields: dict[str, str], boundary: str) -> bytes:
    parts: list[bytes] = []
    sep = f"--{boundary}".encode()
    for name, value in fields.items():
        parts.append(sep)
        parts.append(f'Content-Disposition: form-data; name="{name}"\r\n\r\n{value}'.encode())
    parts.append(f"--{boundary}--".encode())
    return b"\r\n".join(parts)


def _check_response_for_errors(body: str) -> None:
    lower = body.lower()
    if "invalid" in lower or "incorrect" in lower or "wrong" in lower:
        raise LoginError("Invalid credentials: server rejected the email/password.")
    if "captcha" in lower:
        raise LoginError("Login blocked by CAPTCHA.")


class LoginError(RuntimeError):
    """Raised when authentication fails."""
