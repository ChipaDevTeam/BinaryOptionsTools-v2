---
sidebar_position: 1
slug: /tutorials
---

# Tutorials

:::tip New to algorithmic trading?
Start by proving an idea works before you automate it.
**[ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=tutorials&utm_campaign=BinaryOptionsToolsV2)** — our AI-powered strategy builder — generates a
strategy from your description, backtests it, and deploys it, all in the browser (free tier).
Then come back here to build the same thing with full control.
Want crypto markets too? **[ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** has a demo mode to practise on.
:::


Step-by-step guides for getting started with BinaryOptionsTools.

## Getting Your PocketOption SSID

To use BinaryOptionsTools, you need your PocketOption session ID (SSID).

### Steps:

1. **Login to PocketOption** on your browser
2. **Navigate to Demo or Real account** whichever you'd like to use
3. **Press `Ctrl + Shift + I`** to open Developer Tools
4. **Click on Network** tab
5. **Click on "WS"** filter to show WebSocket connections
6. **Refresh the page**
7. **Find the Socket connection** with an "AUTH" line (should say "session" not "sessionToken")
8. **Right-click the AUTH message** → Copy → Copy message
9. **Paste the SSID** into your bot configuration

### Security Best Practices

- Treat the SSID/auth string as a **secret**. Do not share it.
- Store it in a secure secret store:
  - `.env` file (local development only) — never commit to version control
  - OS keychain (macOS Keychain, Windows Credential Manager)
  - Password manager (1Password, Bitwarden)
- Never hardcode secrets into source files or public repos
- Limit where copies of the secret exist; remove from clipboard history if your OS exposes it

---

## SSID Fetcher Userscript

For automated SSID retrieval, use the provided userscript.

### Prerequisites

- Install a userscript manager: [Violentmonkey](https://github.com/violentmonkey/violentmonkey), Tampermonkey, or Greasemonkey

### Installation

1. Locate the userscript at `tutorials/scripts/SSID_Fetcher_UserScript.user.js`
2. Open your userscript manager dashboard
3. Use "Add new script" or "Import" and paste the script
4. Ensure the script is active for `pocketoption.com`

### Usage

1. Open [PocketOption](https://pocketoption.com/) and log in
2. The script intercepts WebSocket outgoing messages
3. When it detects an authentication message, it prompts you to confirm
4. If confirmed, it copies the full auth string to clipboard and shows it in an alert

### Notes

- The script bypasses interception for WebSocket URLs matching `events-po.com` for safety
- Use only on accounts you own or are authorized to access
- If unsure, prefer manual inspection via Developer Tools over installing third-party scripts

---

## Scripts

- [SSID Fetcher Userscript](/tutorials/scripts)