#!/usr/bin/env python3
"""
CloseOption SSID Exporter

Extracts CloseOption session credentials from browser storage.

Usage:
    python scripts/export_closeoption_ssid.py [--browser chrome]
    
Supported browsers: chrome, firefox, edge, brave
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Optional, Tuple


def get_browser_paths(browser: str) -> Optional[Path]:
    """Get the browser profile path for the current OS."""
    home = Path.home()
    
    paths = {
        "chrome": {
            "windows": home / "AppData" / "Local" / "Google" / "Chrome" / "User Data",
            "mac": home / "Library" / "Application Support" / "Google" / "Chrome",
            "linux": home / ".config" / "chrome",
        },
        "firefox": {
            "windows": home / "AppData" / "Roaming" / "Mozilla" / "Firefox" / "Profiles",
            "mac": home / "Library" / "Application Support" / "Firefox" / "Profiles",
            "linux": home / ".mozilla" / "firefox" / "profiles",
        },
        "edge": {
            "windows": home / "AppData" / "Local" / "Microsoft" / "Edge" / "User Data",
            "mac": home / "Library" / "Application Support" / "Microsoft Edge",
            "linux": home / ".config" / "microsoft-edge",
        },
        "brave": {
            "windows": home / "AppData" / "Local" / "BraveSoftware" / "Brave-Browser" / "User Data",
            "mac": home / "Library" / "Application Support" / "BraveSoftware" / "Brave-Browser",
            "linux": home / ".config" / "brave-browser",
        },
    }
    
    platform = sys.platform
    if platform.startswith("win"):
        platform = "windows"
    elif platform.startswith("darwin"):
        platform = "mac"
    else:
        platform = "linux"
    
    return paths.get(browser, {}).get(platform)


def decrypt_chrome_password(encrypted: bytes) -> bytes:
    """
    Decrypt Chrome/Chromium encrypted passwords.
    Uses Windows DPAPI via ctypes.
    """
    try:
        import ctypes
        from ctypes import wintypes
        
        # Get current user's password
        crypto_api = ctypes.windll.crypt32
        blob = ctypes.create_string_buffer(encrypted)
        
        # Decrypt using DPAPI
        data = ctypes.create_string_buffer(len(encrypted) + 100)
        size = wintypes.DWORD(0)
        
        if crypto_api.CryptUnprotectData(
            ctypes.byref(blob), None, None, None, None, 0,
            ctypes.byref(data), ctypes.byref(size)
        ):
            return data[:size.value]
    except Exception:
        pass
    return encrypted


def find_closeoption_cookies(browser_path: Path) -> Optional[dict]:
    """Find CloseOption cookies in browser storage."""
    # Check for Chrome/Chromium cookies database
    cookies_db = browser_path / "Default" / "Cookies"
    if not cookies_db.exists():
        # Try other profiles
        for profile_dir in browser_path.glob("Profile *"):
            cookies_db = profile_dir / "Cookies"
            if cookies_db.exists():
                break
        else:
            return None
    
    # For simplicity, we'll return a placeholder since direct cookie reading
    # requires handling SQLite and encryption
    return None


def extract_from_local_storage(browser_path: Path) -> Optional[dict]:
    """Try to extract session data from localStorage."""
    local_storage = browser_path / "Default" / "Local Storage" / "leveldb"
    
    if not local_storage.exists():
        return None
    
    # Look for closeoption.com entries
    # This is a simplified approach - real implementation would parse LevelDB
    return None


def get_ssid_from_file(filepath: Optional[str] = None) -> Optional[str]:
    """
    Extract SSID from a saved session file.
    
    Expected JSON format:
    {
        "token": "your_token",
        "sid": "your_session_id",
        "demo": true,
        "public_code": "your_public_code",
        "hidden_code": "your_hidden_code"
    }
    """
    if not filepath:
        # Try common locations
        candidates = [
            os.path.expanduser("~/.closeoption_session.json"),
            os.path.expanduser("~/.bin options_tools/closeoption.json"),
            "closeoption_session.json",
        ]
    else:
        candidates = [filepath]
    
    for path in candidates:
        try:
            with open(path, 'r') as f:
                data = json.load(f)
            
            token = data.get('token', '')
            sid = data.get('sid', '')
            demo = data.get('demo', True)
            public_code = data.get('public_code', '')
            hidden_code = data.get('hidden_code', '')
            
            if token and sid:
                return format_ssid(token, sid, demo, public_code, hidden_code)
        except (FileNotFoundError, json.JSONDecodeError):
            continue
    
    return None


def format_ssid(
    token: str,
    sid: str,
    demo: bool = True,
    public_code: str = "",
    hidden_code: str = ""
) -> str:
    """Format SSID components into pipe-delimited string."""
    demo_str = "true" if demo else "false"
    return f"{token}|{sid}|{demo_str}|{public_code}|{hidden_code}"


def parse_ssid(ssid: str) -> Tuple[str, str, bool, str, str]:
    """Parse SSID into components."""
    parts = ssid.split('|')
    
    if len(parts) >= 5:
        token = parts[0]
        sid = parts[1]
        demo = parts[2].lower() in ('true', '1', 'yes')
        public_code = parts[3]
        hidden_code = parts[4]
    elif len(parts) == 3:
        token = parts[0]
        sid = parts[1]
        demo = parts[2].lower() in ('true', '1', 'yes')
        public_code = ""
        hidden_code = ""
    else:
        token = ssid
        sid = ""
        demo = True
        public_code = ""
        hidden_code = ""
    
    return token, sid, demo, public_code, hidden_code


def save_ssid(ssid: str, filepath: Optional[str] = None):
    """Save SSID to file for future use."""
    token, sid, demo, public_code, hidden_code = parse_ssid(ssid)
    
    if not filepath:
        filepath = os.path.expanduser("~/.closeoption_session.json")
    
    os.makedirs(os.path.dirname(filepath) if os.path.dirname(filepath) else ".", exist_ok=True)
    
    data = {
        "token": token,
        "sid": sid,
        "demo": demo,
        "public_code": public_code,
        "hidden_code": hidden_code,
    }
    
    with open(filepath, 'w') as f:
        json.dump(data, f, indent=2)
    
    print(f"SSID saved to: {filepath}")


def print_export_instructions():
    """Print instructions for manual SSID extraction."""
    print("""
=== CloseOption SSID Export Instructions ===

Method 1: Using Browser DevTools
---------------------------------
1. Open CloseOption in your browser
2. Press F12 to open DevTools
3. Go to Application/Storage tab
4. Find Cookies for closeoption.com
5. Look for 'token', 'sid', 'publicCode', 'hiddenCode' values

Method 2: Using Network Tab
----------------------------
1. Open CloseOption in your browser
2. Press F12 and go to Network tab
3. Refresh the page
4. Click on any WebSocket connection
5. Look at Request Headers for authorization tokens

Method 3: Using Console
------------------------
Run this in browser console:
```javascript
// Get all cookies
document.cookie
// Or check localStorage
localStorage.getItem('session')
```

SSID Format:
------------
token|sid|demo|public_code|hidden_code

Example:
abc123token|xyz789sid|true|pub_code123|hid_code456
""")


def main():
    parser = argparse.ArgumentParser(
        description="Export CloseOption SSID from browser or file"
    )
    parser.add_argument(
        "--browser", "-b",
        choices=["chrome", "firefox", "edge", "brave"],
        help="Browser to extract from (default: auto-detect)"
    )
    parser.add_argument(
        "--file", "-f",
        help="Read SSID from JSON file"
    )
    parser.add_argument(
        "--save", "-s",
        help="Save extracted SSID to file"
    )
    parser.add_argument(
        "--manual", "-m",
        action="store_true",
        help="Print manual extraction instructions"
    )
    parser.add_argument(
        "--input", "-i",
        help="Input SSID directly (for testing)"
    )
    
    args = parser.parse_args()
    
    # Direct input mode
    if args.input:
        ssid = args.input
        print(f"Input SSID: {ssid[:30]}...")
        token, sid, demo, pub, hid = parse_ssid(ssid)
        print(f"  Token: {token[:20]}...")
        print(f"  SID: {sid[:20]}...")
        print(f"  Demo: {demo}")
        print(f"  Public Code: {pub}")
        print(f"  Hidden Code: {hid}")
        
        if args.save:
            save_ssid(ssid, args.save)
        
        # Print export command
        print(f"\nExport command:")
        if sys.platform.startswith("win"):
            print(f'  set CLOSEOPTION_SSID={ssid}')
        else:
            print(f'  export CLOSEOPTION_SSID="{ssid}"')
        return 0
    
    # Manual instructions mode
    if args.manual:
        print_export_instructions()
        return 0
    
    # Try to read from file
    if args.file:
        ssid = get_ssid_from_file(args.file)
        if ssid:
            print(f"Read from {args.file}:")
            print(f"  CLOSEOPTION_SSID={ssid[:50]}...")
            
            if args.save:
                save_ssid(ssid, args.save)
            
            # Print export command
            if sys.platform.startswith("win"):
                print(f'\nRun: set CLOSEOPTION_SSID={ssid}')
            else:
                print(f'\nRun: export CLOSEOPTION_SSID="{ssid}"')
            return 0
        else:
            print(f"Error: Could not read SSID from {args.file}")
            return 1
    
    # Try browser extraction
    browser = args.browser or "chrome"
    browser_path = get_browser_paths(browser)
    
    if browser_path and browser_path.exists():
        print(f"Checking {browser.capitalize()} at: {browser_path}")
        # Note: Real browser extraction requires handling encrypted storage
        # This is a placeholder for the actual implementation
        print("Browser extraction requires additional setup.")
        print("Please use --manual for instructions or --input to provide SSID directly.\n")
    else:
        print(f"Browser {browser} not found at expected location.")
    
    # Fall back to checking saved files
    ssid = get_ssid_from_file()
    if ssid:
        print(f"Found saved SSID:")
        print(f"  CLOSEOPTION_SSID={ssid[:50]}...")
        
        if sys.platform.startswith("win"):
            print(f'\nRun: set CLOSEOPTION_SSID={ssid}')
        else:
            print(f'\nRun: export CLOSEOPTION_SSID="{ssid}"')
        return 0
    
    # Show instructions
    print_export_instructions()
    return 0


if __name__ == "__main__":
    sys.exit(main())
