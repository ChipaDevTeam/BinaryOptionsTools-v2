"""Probe which HTTP method can reach pocketoption.com/en/login/"""
import sys

def try_curl_cffi():
    try:
        from curl_cffi import requests as cr
        r = cr.get("https://pocketoption.com/en/login/", impersonate="chrome", timeout=20)
        print(f"[curl_cffi chrome] status={r.status_code} len={len(r.text)}")
        return r.status_code == 200
    except Exception as e:
        print(f"[curl_cffi] failed: {e}")
        return False

def try_playwright_firefox():
    try:
        from playwright.sync_api import sync_playwright
        with sync_playwright() as pw:
            browser = pw.firefox.launch(headless=True)
            page = browser.new_page()
            page.goto("https://pocketoption.com/en/login/", wait_until="domcontentloaded", timeout=20000)
            print(f"[playwright firefox] url={page.url} title={page.title()}")
            browser.close()
            return True
    except Exception as e:
        print(f"[playwright firefox] failed: {e}")
        return False

def try_playwright_chrome_channel():
    try:
        from playwright.sync_api import sync_playwright
        with sync_playwright() as pw:
            browser = pw.chromium.launch(headless=True, channel="chrome")
            page = browser.new_page()
            page.goto("https://pocketoption.com/en/login/", wait_until="domcontentloaded", timeout=20000)
            print(f"[playwright chrome channel] url={page.url}")
            browser.close()
            return True
    except Exception as e:
        print(f"[playwright chrome channel] failed: {e}")
        return False

print("Testing connectivity methods...")
ok1 = try_curl_cffi()
ok2 = try_playwright_firefox()
ok3 = try_playwright_chrome_channel()
print(f"\nResults: curl_cffi={ok1}  playwright_firefox={ok2}  playwright_chrome={ok3}")
