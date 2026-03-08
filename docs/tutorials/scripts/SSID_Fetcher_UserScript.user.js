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
    const socketUrl = (this.url || this._interceptUrl || "").toLowerCase();

    // STRICT EXCLUSION: If the URL belongs to events-po.com, bypass the logic immediately
    if (socketUrl.includes("events-po.com")) {
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
