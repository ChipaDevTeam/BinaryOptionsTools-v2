"""
Login to PocketOption using email and password (no SSID needed).

Requirements:
    pip install patchright
    patchright install chromium          # first time only

Optional (2captcha backend):
    pip install requests
    # get an API key at https://2captcha.com

Usage:
    python login_with_email_and_password.py
"""

import asyncio
import os
import sys
from pathlib import Path

# Allow running directly from the repo without installing the package
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "python"))

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
from BinaryOptionsToolsV2.pocketoption.tools.login import LoginError, login_async


async def main() -> None:
    email = os.getenv("POCKET_OPTION_EMAIL") or input("Email: ")
    password = os.getenv("POCKET_OPTION_PASSWORD") or input("Password: ")
    demo = True  # set False for real-money account

    print("Logging in via headless browser …")
    try:
        ssid = await login_async(email, password, demo=demo)
    except LoginError as exc:
        print(f"Login failed: {exc}")
        return
    except ImportError as exc:
        print(f"Missing dependency: {exc}")
        return

    print(f"Got SSID (first 60 chars): {ssid[:60]}…")

    # Use the SSID to connect and fetch the balance
    async with PocketOptionAsync(ssid) as api:
        balance = await api.balance()
        account = "DEMO" if demo else "REAL"
        print(f"[{account}] Balance: {balance}")


if __name__ == "__main__":
    asyncio.run(main())
