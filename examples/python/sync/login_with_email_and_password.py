"""
Login to PocketOption using email and password (no SSID needed) — sync version.

Requirements:
    pip install patchright
    patchright install chromium          # first time only

Optional (2captcha backend):
    pip install requests
    # get an API key at https://2captcha.com

Usage:
    python login_with_email_and_password.py
"""

import os

from BinaryOptionsToolsV2.pocketoption import PocketOption
from BinaryOptionsToolsV2.pocketoption.tools.login import LoginError, login


def main() -> None:
    email = os.getenv("POCKET_OPTION_EMAIL") or input("Email: ")
    password = os.getenv("POCKET_OPTION_PASSWORD") or input("Password: ")
    demo = True  # set False for real-money account

    print("Logging in via headless browser …")
    try:
        ssid = login(email, password, demo=demo)
    except LoginError as exc:
        print(f"Login failed: {exc}")
        return
    except ImportError as exc:
        print(f"Missing dependency: {exc}")
        return

    print(f"Got SSID (first 60 chars): {ssid[:60]}…")

    # Use the SSID to connect and fetch the balance
    with PocketOption(ssid) as api:
        balance = api.balance()
        account = "DEMO" if demo else "REAL"
        print(f"[{account}] Balance: {balance}")


if __name__ == "__main__":
    main()
