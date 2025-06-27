import asyncio
import time
import json # For the SSID formatting function
import sys
import getpass
from datetime import datetime, timedelta, timezone
from tabulate import tabulate # Keep if catalogador is re-enabled (currently not used)
from colorama import init, Fore, Back

# Imports for Selenium Login
try:
    from selenium import webdriver
    from selenium.webdriver.common.by import By
    from selenium.webdriver.support.ui import WebDriverWait
    from selenium.webdriver.support import expected_conditions
    from selenium.webdriver.chrome.service import Service
    from webdriver_manager.chrome import ChromeDriverManager
    import urllib.parse
    SELENIUM_AVAILABLE = True
except ImportError:
    SELENIUM_AVAILABLE = False
    print(Fore.RED + "Selenium libraries not found. Login with email/password will not be available.")
    print(Fore.YELLOW + "Please install them: pip install selenium webdriver-manager")


### SSID FORMATTING FUNCTION ###
def format_session_for_pocketoption_auth(
    input_session_string: str,
    is_demo: int = 0,
    uid: int = 101884312, # !! CRITICAL: USER MUST VERIFY THIS UID !!
    platform: int = 2,
    is_fast_history: bool = False
) -> str:
    """
    Formats the raw session string into the specific auth structure required by PocketOption.
    Example Target: 42["auth",{"session":"PHP_SERIALIZED_STRING_WITH_HASH","isDemo":0,"uid":101884312,"platform":2,"isFastHistory":false}]
    The input_session_string is the PHP_SERIALIZED_STRING_WITH_HASH part.
    """
    # The input_session_string is the raw PHP serialized string.
    # json.dumps will handle escaping the double quotes within it correctly for JSON.
    
    auth_payload = {
        "session": input_session_string, # Use the original string directly
        "isDemo": is_demo,
        "uid": uid,
        "platform": platform,
        "isFastHistory": is_fast_history
    }
    
    # Convert the payload dictionary to a JSON string
    # json.dumps will correctly handle boolean to lowercase true/false
    # and ensure valid JSON syntax, including escaping quotes in input_session_string to \"
    auth_payload_json_string = json.dumps(auth_payload, separators=(',', ':'))

    # Construct the final target string
    target_string = f'42["auth",{auth_payload_json_string}]'
    
    # print(yellow + f"Formatted SSID: {target_string}") # Consider conditional printing or logging
    print(f"Formatted SSID: {target_string}") # Simplified print
    return target_string

async def login_and_get_ssid(email_str: str, password_str: str) -> str | None:
    """Handles Selenium login and returns the formatted SSID string or None on failure."""
    if not SELENIUM_AVAILABLE:
        print(red + "Selenium libraries are not available. Cannot login with email/password.")
        return None

    # This inner function contains the blocking Selenium code
    def get_ssid_blocking(email_val: str, password_val: str) -> str | None:
        print(yellow + "Initializing WebDriver for PocketOption login...")
        driver = None # Initialize driver to None for finally block
        try:
            chrome_options = webdriver.ChromeOptions()
            # Common options to try for stability, especially in automated/headless environments
            chrome_options.add_argument("--no-sandbox")
            chrome_options.add_argument("--disable-dev-shm-usage")
            chrome_options.add_argument("--disable-gpu") # Often recommended for headless
            chrome_options.add_argument("start-maximized") # Maximize window
            chrome_options.add_argument("disable-infobars")
            chrome_options.add_argument("--disable-extensions")
            # chrome_options.add_argument("--headless=new") # For new Selenium headless
            # chrome_options.add_argument("--window-size=1920,1080") # Specify window size

            service = Service(ChromeDriverManager().install())
            driver = webdriver.Chrome(service=service, options=chrome_options)
            
            print(yellow + "Navigating to PocketOption login page (po.trade/login)...")
            driver.get("https://po.trade/login") # Official PocketOption URL for login

            # Wait for email field to be present and interactable
            email_field = WebDriverWait(driver, 30).until(
                expected_conditions.element_to_be_clickable((By.NAME, "email"))
            )
            print(yellow+"Login page loaded. Entering credentials...")
            email_field.click()
            email_field.clear()
            email_field.send_keys(email_val)

            password_field = driver.find_element(By.NAME, "password")
            password_field.click()
            password_field.clear()
            password_field.send_keys(password_val)

            # Try to find the login button using various robust selectors
            login_button_selectors = [
                "button[type='submit'].btn.btn-green-light", # From original user example
                "//button[normalize-space(translate(text(), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'))='log in' and @type='submit']", # Case-insensitive XPath
                "//button[normalize-space(translate(text(), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'))='login' and @type='submit']", # Case-insensitive XPath
                "form button[type='submit']" # General form submit button
            ]
            login_button = None
            for idx, selector in enumerate(login_button_selectors):
                try:
                    by_method = By.XPATH if selector.startswith("//") else By.CSS_SELECTOR
                    login_button = WebDriverWait(driver, 5).until(
                        expected_conditions.element_to_be_clickable((by_method, selector))
                    )
                    if login_button:
                        print(yellow + f"Login button found with selector #{idx+1} ('{selector}').")
                        break
                except:
                    if idx == len(login_button_selectors) -1 : # If last selector also failed
                         print(red + f"Selector '{selector}' not found or not clickable.")
            
            if not login_button:
                print(red + "Could not find a clickable login button.")
                driver.save_screenshot("debug_login_button_not_found.png")
                return None # Exit if button not found

            login_button.click()
            print(yellow + "Login submitted. Waiting for page load/redirection (up to 60s)...")

            # Wait for a condition that indicates successful login
            # e.g., URL change from /login, or presence of a dashboard/cabinet element
            WebDriverWait(driver, 60).until(
                lambda d: d.current_url != "https://po.trade/login/" and \
                          (expected_conditions.url_contains("cabinet")(d) or \
                           expected_conditions.presence_of_element_located((By.ID, "crm-widget-wrapper"))(d) or # Element from PO live trading
                           expected_conditions.presence_of_element_located((By.CSS_SELECTOR, ".is_real"))(d) or # Real account indicator
                           expected_conditions.presence_of_element_located((By.CSS_SELECTOR, ".is_demo"))(d) )  # Demo account indicator
            )
            print(green + f"Login appears successful. Current URL: {driver.current_url}")
            print(yellow+ "Retrieving session cookie(s)...")

            cookies = driver.get_cookies()
            session_token = None
            # Common session cookie names for PHP sites like PocketOption. PHPSESSID is very common.
            possible_session_cookie_names = ['PHPSESSID', 'ci_session', 'po_session', 'SID', 'ssid'] 
            for cookie in cookies:
                if cookie["name"] in possible_session_cookie_names:
                    session_token = cookie["value"]
                    print(green + f"Found session cookie: {cookie['name']} = {session_token[:30]}...")
                    break # Take the first one found
            
            if not session_token:
                print(red + "Could not find a known session cookie (e.g., PHPSESSID, ci_session) after login.")
                print(yellow + "Available cookies:")
                for cookie_idx, cookie_data in enumerate(cookies): print(f"  {cookie_idx+1}. {cookie_data['name']}: {cookie_data['value'][:40]}...")
                driver.save_screenshot("debug_no_session_cookie.png")
                return None

            # URL decode the raw cookie value
            decoded_cookie_value = urllib.parse.unquote(session_token)
            print(green + f"Raw session cookie value (decoded): {decoded_cookie_value[:60]}...")
            
            # Format this decoded cookie value into the target SSID structure
            # !! IMPORTANT: The defaults for isDemo, uid in format_session_for_pocketoption_auth are from user's example.
            # !! These might need to be dynamically determined (scraped) by Selenium from the page if they vary.
            # Example: a basic dynamic check for demo account (might need refinement)
            is_demo_account = 0 # Default to real
            try:
                if driver.find_elements(By.CSS_SELECTOR, ".is_demo_balance") or "demo" in driver.current_url.lower():
                    is_demo_account = 1
                    print(yellow + "Detected DEMO account from page elements/URL.")
            except: pass # Ignore if elements not found, keep default

            # UID is harder to get dynamically and is critical. Using hardcoded from example for now.
            # User MUST ensure the UID in format_session_for_pocketoption_auth is correct for their account.
            formatted_ssid = format_session_for_pocketoption_auth(decoded_cookie_value, is_demo=is_demo_account) 
            print(green + f"Formatted SSID for PocketOptionAsync: {formatted_ssid[:80]}...")
            return formatted_ssid

        except Exception as e_selenium:
            print(red + f"Error during Selenium login operations: {e_selenium}")
            import traceback
            traceback.print_exc()
            if driver:
                try:
                    driver.save_screenshot("selenium_login_error.png")
                    print(yellow + "Screenshot saved as selenium_login_error.png")
                    print(yellow + f"Current URL at error: {driver.current_url if driver.current_url else 'N/A'}")
                except Exception as e_screenshot:
                    print(red + f"Could not save screenshot: {e_screenshot}")
            return None
        finally:
            if driver:
                print(yellow + "Closing WebDriver.")
                driver.quit()
    
    # Run the blocking Selenium function in a separate thread to avoid blocking asyncio event loop
    return await asyncio.to_thread(get_ssid_blocking, email_str, password_str)
