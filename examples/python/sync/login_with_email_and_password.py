"""
Login to PocketOption using email and password (no SSID needed) — sync version.

Two backends are supported — pick the one that works in your environment:

  A) Playwright headless browser (default)
     Works when browser processes can reach pocketoption.com.
       pip install playwright
       py -3 -m playwright install firefox chromium

  B) CapSolver captcha solver API  ← use this if browsers are firewall-blocked
     Python's requests library CAN usually reach the site even when browsers can't.
       pip install requests
       Get a FREE API key at https://capsolver.com  (no credit card needed)

Usage:
    python login_with_email_and_password.py
"""

import os
import sys
from pathlib import Path

# Allow running directly from the repo without installing the package
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "python"))

from BinaryOptionsToolsV2.pocketoption import PocketOption
from BinaryOptionsToolsV2.pocketoption.tools.login import LoginError, login

# ── Configuration ──────────────────────────────────────────────────────────────

EMAIL = os.getenv("POCKET_OPTION_EMAIL") or input("Email: ")
PASSWORD = os.getenv("POCKET_OPTION_PASSWORD") or input("Password: ")
DEMO = True  # set False for real-money account

# Leave empty to use the Playwright browser backend.
# Set to your CapSolver key to use the HTTP + captcha-solver backend instead.
# Get a free key at https://capsolver.com
CAPSOLVER_API_KEY = os.getenv("CAPSOLVER_API_KEY", "")

# ── Login ──────────────────────────────────────────────────────────────────────


def main() -> None:
    if CAPSOLVER_API_KEY:
        print("Logging in via CapSolver (HTTP backend) …")
        backend_kwargs = {"backend": "capsolver", "api_key": CAPSOLVER_API_KEY}
    else:
        print("Logging in via headless browser …")
        print("  (If this fails, set CAPSOLVER_API_KEY to use the HTTP backend)")
        backend_kwargs = {"backend": "auto"}

    try:
        ssid = login(EMAIL, PASSWORD, demo=DEMO, **backend_kwargs)
    except LoginError as exc:
        print(f"\nLogin failed:\n{exc}")
        return
    except ImportError as exc:
        print(f"\nMissing dependency: {exc}")
        return

    print(f"\nGot SSID (first 60 chars): {ssid[:60]}…")

    with PocketOption(ssid) as api:
        balance = api.balance()
        account = "DEMO" if DEMO else "REAL"
        print(f"[{account}] Balance: {balance}")


if __name__ == "__main__":
    main()
