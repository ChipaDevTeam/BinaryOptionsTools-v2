"""
Login module for PocketOption — obtain a session SSID from email/password.

Flow:
  1. GET /en/login/ to collect cookies and scrape the hidden CSRF token.
  2. POST /en/login/ with multipart/form-data (email, password, token, …).
  3. Parse the Set-Cookie headers for the `po_session` cookie.
  4. Return the SSID string ready for use with PocketOptionAsync / PocketOption.

Note: PocketOption uses reCAPTCHA v3 (invisible).  The login endpoint accepts
an empty `g-recaptcha-response` field when requests originate from the same IP
that loaded the login page, which is the case for direct HTTP requests that
carry the correct session cookies obtained in step 1.
"""

from __future__ import annotations

import re
import uuid
from http.cookiejar import CookieJar
from typing import Optional
from urllib.parse import urlencode
from urllib.request import (
    HTTPCookieProcessor,
    Request,
    build_opener,
)

BASE_URL = "https://pocketoption.com"
LOGIN_PATH = "/en/login/"
LOGIN_URL = BASE_URL + LOGIN_PATH

_DEFAULT_UA = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/146.0.0.0 Safari/537.36"
)

# Regex that matches the hidden _token / token field in the login form HTML.
_TOKEN_RE = re.compile(
    r'<input[^>]+name=["\']token["\'][^>]+value=["\']([^"\']+)["\']',
    re.IGNORECASE,
)
_TOKEN_RE_ALT = re.compile(
    r'<input[^>]+value=["\']([^"\']+)["\'][^>]+name=["\']token["\']',
    re.IGNORECASE,
)

# The cookie name that holds the session identifier used as the SSID.
_SESSION_COOKIE_NAME = "po_session"


def _build_multipart(fields: dict[str, str], boundary: str) -> bytes:
    """Encode *fields* as a multipart/form-data body."""
    parts: list[bytes] = []
    sep = f"--{boundary}".encode()
    for name, value in fields.items():
        parts.append(sep)
        parts.append(
            f'Content-Disposition: form-data; name="{name}"\r\n\r\n{value}'.encode()
        )
    parts.append(f"--{boundary}--".encode())
    return b"\r\n".join(parts)


def login(
    email: str,
    password: str,
    *,
    demo: bool = False,
    user_agent: str = _DEFAULT_UA,
    timeout: int = 30,
) -> str:
    """Login to PocketOption and return the SSID string.

    Args:
        email: Account e-mail address.
        password: Account password.
        demo: If True the returned SSID will request the demo account; if False
              it will request the real-money account.  PocketOption determines
              this via the ``isDemo`` field embedded in the SSID JSON, which
              this function patches after obtaining the session.
        user_agent: Browser User-Agent header to send.
        timeout: HTTP timeout in seconds.

    Returns:
        SSID string of the form ``42["auth",{...}]`` that can be passed
        directly to :class:`~BinaryOptionsToolsV2.pocketoption.PocketOptionAsync`.

    Raises:
        LoginError: When the server rejects the credentials or the session
                    cookie is not found in the response.
        ValueError: When the CSRF token cannot be located on the login page.
    """
    jar = CookieJar()
    opener = build_opener(HTTPCookieProcessor(jar))

    # ── Step 1: GET login page ────────────────────────────────────────────────
    get_req = Request(
        LOGIN_URL,
        headers={
            "User-Agent": user_agent,
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            "Accept-Language": "en-GB,en;q=0.9",
            "Referer": BASE_URL,
        },
    )
    with opener.open(get_req, timeout=timeout) as resp:
        html = resp.read().decode("utf-8", errors="replace")

    token = _extract_token(html)
    if not token:
        raise ValueError(
            "Could not find the CSRF token on the PocketOption login page. "
            "The page structure may have changed."
        )

    # ── Step 2: POST credentials ──────────────────────────────────────────────
    boundary = f"----WebKitFormBoundary{uuid.uuid4().hex[:16].upper()}"
    fields = {
        "submitLogin": "1",
        "email": email,
        "password": password,
        "remember": "1",
        "g-recaptcha-response": "",
        "token": token,
    }
    body = _build_multipart(fields, boundary)

    post_req = Request(
        LOGIN_URL,
        data=body,
        headers={
            "User-Agent": user_agent,
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Content-Length": str(len(body)),
            "Accept": "application/json, text/javascript, */*; q=0.01",
            "Accept-Language": "en-GB,en;q=0.9",
            "X-Requested-With": "XMLHttpRequest",
            "Origin": BASE_URL,
            "Referer": LOGIN_URL,
            "Sec-Fetch-Site": "same-origin",
            "Sec-Fetch-Mode": "cors",
            "Sec-Fetch-Dest": "empty",
        },
    )

    with opener.open(post_req, timeout=timeout) as resp:
        response_body = resp.read().decode("utf-8", errors="replace")

    # ── Step 3: Extract session cookie ────────────────────────────────────────
    session_value: Optional[str] = None
    for cookie in jar:
        if cookie.name == _SESSION_COOKIE_NAME:
            session_value = cookie.value
            break

    if not session_value:
        # Attempt to detect a wrong-credentials error from the response body.
        _check_error(response_body)
        raise LoginError(
            f"Login appeared to succeed but '{_SESSION_COOKIE_NAME}' cookie was "
            "not found. The session cookie name may have changed."
        )

    # ── Step 4: Build SSID ────────────────────────────────────────────────────
    is_demo_int = 1 if demo else 0
    ssid = (
        f'42["auth",{{"session":"{session_value}",'
        f'"isDemo":{is_demo_int},"uid":0,"platform":2}}]'
    )
    return ssid


async def login_async(
    email: str,
    password: str,
    *,
    demo: bool = False,
    user_agent: str = _DEFAULT_UA,
    timeout: int = 30,
) -> str:
    """Async wrapper around :func:`login` using :mod:`asyncio`.

    Runs the blocking HTTP calls in a thread-pool executor so the event loop
    is not blocked.
    """
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
            user_agent=user_agent,
            timeout=timeout,
        ),
    )


# ── Helpers ───────────────────────────────────────────────────────────────────


def _extract_token(html: str) -> Optional[str]:
    m = _TOKEN_RE.search(html) or _TOKEN_RE_ALT.search(html)
    return m.group(1) if m else None


def _check_error(body: str) -> None:
    """Raise a descriptive LoginError when the response signals failure."""
    lower = body.lower()
    if "invalid" in lower or "incorrect" in lower or "wrong" in lower:
        raise LoginError("Invalid credentials: server rejected the email/password.")
    if "captcha" in lower:
        raise LoginError(
            "Login blocked by CAPTCHA. Try again later or provide a valid "
            "reCAPTCHA token."
        )


class LoginError(RuntimeError):
    """Raised when authentication fails."""
