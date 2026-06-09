"""Quick test: try patchright vs playwright to reach pocketoption.com/en/login/"""
import sys

def try_patchright():
    try:
        from patchright.sync_api import sync_playwright
        print("Using patchright")
    except ImportError:
        from playwright.sync_api import sync_playwright
        print("Using playwright (patchright not available)")

    with sync_playwright() as pw:
        browser = pw.chromium.launch(
            headless=True,
            args=["--disable-blink-features=AutomationControlled", "--no-sandbox"],
        )
        ctx = browser.new_context(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36",
            locale="en-US",
            viewport={"width": 1366, "height": 768},
        )
        ctx.add_init_script("Object.defineProperty(navigator, 'webdriver', {get: () => undefined})")
        page = ctx.new_page()
        try:
            page.goto("https://pocketoption.com/en/login/", wait_until="domcontentloaded", timeout=30000)
            print("STATUS: loaded OK")
            print("URL:", page.url)
            # Check for form
            inputs = page.locator("input").all()
            for inp in inputs:
                print("INPUT:", inp.get_attribute("name"), inp.get_attribute("type"))
            cookies = ctx.cookies()
            print("COOKIES:", [c["name"] for c in cookies])
        except Exception as e:
            print("ERROR:", e)
        finally:
            browser.close()

try_patchright()
