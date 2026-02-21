// ==UserScript==
// @name        PocketOption SSID Fetcher
// @namespace   SixsPocketOptionSSIDFetcher
// @match       *://pocketoption.com/*
// @match       *://*.pocketoption.com/*
// @grant       none
// @version     1.2
// @author      Six
// @description Intercepts auth SSID from PocketOption
// ==/UserScript==

(function() {
    'use strict';

    const originalSend = WebSocket.prototype.send;

    WebSocket.prototype.send = function(data) {
        if (typeof data === 'string' && data.startsWith('42["auth",')) {
            try {
                const jsonStr = data.substring(2);
                const parsedData = JSON.parse(jsonStr);

                if (parsedData[0] === 'auth' && parsedData[1] && parsedData[1].session) {
                    const ssid = parsedData[1].session;

                    // Ask the user before showing sensitive info (basic security check)
                    const userwantsToShow = confirm("SSID Intercepted. Would you like to display the Session ID (SSID)?");

                    if (userwantsToShow) {
                        alert("Your SSID is:\n\n" + ssid);
                    } else {
                        console.log("[SSID Fetcher] Display dismissed by user.");
                    }
                }
            } catch (e) {
                // Ignore parsing errors to prevent site disruption
            }
        }

        return originalSend.apply(this, arguments);
    };

    console.log('[SSID Fetcher] Hooked and waiting for authentication...');
})();
