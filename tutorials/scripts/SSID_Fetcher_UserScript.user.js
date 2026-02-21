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

(function() {
    'use strict';

    // Hook the WebSocket constructor
    const OriginalWebSocket = window.WebSocket;

    window.WebSocket = function(url, protocols) {
        const socket = new OriginalWebSocket(url, protocols);
        // Manual tag as fallback in case the native .url property is restricted
        try {
            socket._interceptUrl = url.toString();
        } catch (e) {}
        return socket;
    };

    // Maintain prototype chain
    window.WebSocket.prototype = OriginalWebSocket.prototype;
    window.WebSocket.prototype.constructor = window.WebSocket;

    // Hook the send method
    const originalSend = OriginalWebSocket.prototype.send;

    OriginalWebSocket.prototype.send = function(data) {
        // Get the URL from the native property or our fallback tag
        const socketUrl = (this.url || this._interceptUrl || "").toLowerCase();

        // STRICT EXCLUSION: If the URL belongs to events-po.com, bypass the logic immediately
        if (socketUrl.includes("events-po.com")) {
            return originalSend.apply(this, arguments);
        }

        // Intercept authentication messages (Real or Demo)
        if (typeof data === 'string' && data.startsWith('42["auth",')) {

            // Security Check
            const userWantsToProceed = confirm(`Auth string intercepted from:\n${socketUrl}\n\nWould you like to show the full string and copy it to your clipboard?`);

            if (userWantsToProceed) {
                // Copy the ENTIRE string
                navigator.clipboard.writeText(data).then(() => {
                    alert("Auth String Copied to Clipboard:\n\n" + data);
                }).catch(err => {
                    console.error('Clipboard copy failed:', err);
                    alert("Auth String Found (Auto-copy failed):\n\n" + data);
                });
            }
        }

        // Always execute original send to maintain platform functionality
        return originalSend.apply(this, arguments);
    };

    console.log('Hooked. ignoring auth msgs from events-po.com.');
})();
