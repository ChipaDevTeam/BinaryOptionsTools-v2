---
sidebar_position: 1
slug: /tutorials/scripts
---

# SSID Fetcher Userscript

Automated SSID retrieval script for PocketOption.

## File: `SSID_Fetcher_UserScript.user.js`

```javascript
// ==UserScript==
// @name        PocketOption SSID Fetcher
// @namespace   SixsPocketOptionSSIDFetcher
// @match       *://pocketoption.com/*
// @match       *://*.pocketoption.com/*
// @grant       none
// @version     1.3
// @author      Six
// @description Intercepts auth SSID from PocketOption
// ==/UserScript==

(function () {
  "use strict";

  // Hook the WebSocket constructor
  const OriginalWebSocket = window.WebSocket;

  window.WebSocket = function (url, protocols) {
    const socket = new OriginalWebSocket(url, protocols);
    // Manual tag as fallback in case the native .url property is restricted
    try {
      socket._interceptUrl = url.toString();
    } catch (e) {}
    return socket;
  };

  // Copy static properties and symbols from OriginalWebSocket to the new constructor
  Object.getOwnPropertyNames(OriginalWebSocket).forEach((prop) => {
    if (prop !== "prototype") {
      Object.defineProperty(
        window.WebSocket,
        prop,
        Object.getOwnPropertyDescriptor(OriginalWebSocket, prop),
      );
    }
  });
  Object.getOwnPropertySymbols(OriginalWebSocket).forEach((sym) => {
    Object.defineProperty(
      window.WebSocket,
      sym,
      Object.getOwnPropertyDescriptor(OriginalWebSocket, sym),
    );
  });

  // Maintain prototype chain
  window.WebSocket.prototype = OriginalWebSocket.prototype;
  window.WebSocket.prototype.constructor = window.WebSocket;

  // Hook the send method
  const originalSend = OriginalWebSocket.prototype.send;

  OriginalWebSocket.prototype.send = function (data) {
    // Always execute original send immediately to maintain platform functionality
    const result = originalSend.apply(this, arguments);

    // Get the URL from the native property or our fallback tag
    const rawSocketUrl = this.url || this._interceptUrl || "";
    const socketUrl = rawSocketUrl.toLowerCase();

    // STRICT EXCLUSION: If the URL host is events-po.com or one of its subdomains, bypass immediately
    let socketHost = "";
    try {
      socketHost = new URL(
        rawSocketUrl,
        window.location.href,
      ).hostname.toLowerCase();
    } catch (e) {}
    if (
      socketHost === "events-po.com" ||
      socketHost.endsWith(".events-po.com")
    ) {
      return result;
    }

    // Intercept authentication messages (Real or Demo)
    if (typeof data === "string" && data.startsWith('42["auth",')) {
      // Handle the intercepted auth string asynchronously to avoid blocking the WebSocket
      setTimeout(() => {
        const userWantsToProceed = confirm(
          `Auth string intercepted from:\n${socketUrl}\n\nWould you like to show the full string and copy it to your clipboard?`,
        );

        if (userWantsToProceed) {
          // Copy the ENTIRE string
          navigator.clipboard
            .writeText(data)
            .then(() => {
              alert("Auth String Copied to Clipboard:\n\n" + data);
            })
            .catch((err) => {
              console.error("Clipboard copy failed:", err);
              alert("Auth String Found (Auto-copy failed):\n\n" + data);
            });
        }
      }, 0);
    }

    return result;
  };

  console.log("Hooked. bypassing send-hook for events-po.com.");
})();
```

## Installation

1. Install a userscript manager: [Violentmonkey](https://github.com/violentmonkey/violentmonkey), Tampermonkey, or Greasemonkey
2. Open the userscript manager dashboard
3. Use "Add new script" or "Import" and paste the script above
4. Ensure the script is active for `pocketoption.com`

## Usage

1. Open [PocketOption](https://pocketoption.com/) and log in to your account
2. The script intercepts WebSocket outgoing messages
3. When it detects an authentication message (starts with `42["auth",...`), it prompts you to confirm
4. If confirmed, it copies the full auth string to clipboard and shows it in an alert

## Security Notes

- The script bypasses interception for WebSocket URLs matching `events-po.com`
- Use only on accounts you own or are authorized to access
- Treat the SSID/auth string as a secret - do not share it
- Store securely in `.env` (local only), OS keychain, or password manager
- Never hardcode secrets in source files or public repos

## Source

The script is also available in the repository at:
`tutorials/scripts/SSID_Fetcher_UserScript.user.js`