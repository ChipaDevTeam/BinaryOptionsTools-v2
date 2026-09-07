// ==UserScript==
// @name         BinaryOptionsTools SSID Extractor
// @namespace    SixsBinaryOptionsSSIDFetcher
// @version      3.7
// @description  Extract SSID/credentials for PocketOption and CloseOption for BinaryOptionsToolsV2 - use Violentmonkey/Tampermonkey menu to extract and copy
// @author       Six
// @match        *://pocketoption.com/*
// @match        *://*.pocketoption.com/*
// @match        *://closeoption.com/*
// @match        *://*.closeoption.com/*
// @grant        GM_setClipboard
// @grant        GM_registerMenuCommand
// @grant        GM_notification
// @grant        GM_getValue
// @grant        GM_setValue
// @run-at       document-start
// ==/UserScript==

(function() {
    'use strict';

    // Platform detection
    function detectPlatform() {
        const hostname = window.location.hostname.toLowerCase();
        if (hostname.includes('pocketoption.com')) return 'pocketoption';
        if (hostname.includes('closeoption.com')) return 'closeoption';
        return 'unknown';
    }

    // Detect if we're on a demo page
    function detectDemo() {
        const url = window.location.href.toLowerCase();
        return url.includes('demo') || url.includes('practice');
    }

    // Platform-specific state
    const platform = detectPlatform();
    const DEBUG = false; // Set to true to log WebSocket traffic/credentials to the console
    let pocketOptionSsid = null;
    let pocketOptionAuthMessage = null;
    let closeOptionCreds = null;
    // ==================== POCKETOPTION ====================

    // Extract SSID from intercepted auth message
    function extractPocketOptionSsid(authData) {
        try {
            // Auth message format: 42["auth",{"session":"...","isDemo":1,"uid":123,"platform":2,"isFastHistory":true,"isOptimized":true}]
            // Handle potential whitespace and multiple concatenated messages
            const match = authData.match(/42\["auth",\s*(\{.+?\})\s*\]/);
            if (!match) return null;

            const payload = JSON.parse(match[1]);
            const token = payload.token || payload.sessionToken || payload.session || payload.ssid || '';
            const demo = payload.isDemo !== undefined ? payload.isDemo : detectDemo();

            if (!token) return null;

            // Return full 42["auth",{...}] format (what Rust Ssid::parse expects)
            // Reconstruct with canonical JSON
            const canonicalPayload = {
                session: token,
                isDemo: demo ? 1 : 0,
                uid: payload.uid || 0,
                platform: payload.platform || 2,
                isFastHistory: payload.isFastHistory || false,
                isOptimized: payload.isOptimized || false
            };
            // Include any extra fields from original payload
            Object.keys(payload).forEach(key => {
                if (!['token', 'sessionToken', 'session', 'ssid', 'isDemo', 'uid', 'platform', 'isFastHistory', 'isOptimized'].includes(key)) {
                    canonicalPayload[key] = payload[key];
                }
            });
            return `42["auth",${JSON.stringify(canonicalPayload)}]`;
        } catch (e) {
            console.error('[PO SSID] Failed to parse auth message:', e, 'Data:', authData.substring(0, 200));
            return null;
        }
    }

    // ==================== CLOSEOPTION ====================

    // Extract CloseOption credentials from localStorage/cookies/WebSocket
    function extractCloseOptionCreds() {
        try {
            // Try to get from localStorage first
            const token = localStorage.getItem('token') || localStorage.getItem('_token') || getCookie('token') || getCookie('_token') || getCookie('XSRF-TOKEN');
            const sid = localStorage.getItem('sid') || localStorage.getItem('socket_sid') || localStorage.getItem('session_id') || getCookie('sid') || getCookie('socket_sid') || getCookie('session_id');
            const publicCode = localStorage.getItem('public_code') || localStorage.getItem('publicCode') || getCookie('public_code') || getCookie('publicCode');
            const hiddenCode = localStorage.getItem('hidden_code') || localStorage.getItem('hiddenCode') || getCookie('hidden_code') || getCookie('hiddenCode');

            // Also check for alternative storage keys
            const altToken = localStorage.getItem('auth_token') || localStorage.getItem('access_token');
            const altSid = localStorage.getItem('socket_sid') || localStorage.getItem('session_id');

            // Fall back to WebSocket-captured values
            const finalToken = token || altToken || window._coToken;
            const finalSid = sid || altSid || window._coSid;
            const finalPublicCode = publicCode || window._coPublicCode;
            const finalHiddenCode = hiddenCode || window._coHiddenCode;

            if (!finalToken || !finalSid || !finalPublicCode || !finalHiddenCode) {
                console.log('[CO SSID] Missing credentials:', { token: !!finalToken, sid: !!finalSid, publicCode: !!finalPublicCode, hiddenCode: !!finalHiddenCode });
                return null;
            }

            const demo = detectDemo() ? '1' : '0';

            // CloseOption credential format: token|sid|demo|public_code|hidden_code (matches Python parse_ssid)
            return `${finalToken}|${finalSid}|${demo}|${finalPublicCode}|${finalHiddenCode}`;
        } catch (e) {
            console.error('[CO SSID] Failed to extract credentials:', e);
            return null;
        }
    }

    // Helper to get cookie by name
    function getCookie(name) {
        const value = `; ${document.cookie}`;
        const parts = value.split(`; ${name}=`);
        if (parts.length === 2) return parts.pop().split(';').shift();
        return null;
    }

    // ==================== WEBSOCKET HOOKING ====================

    // Hook a single WebSocket instance
    function hookSocket(socket, url) {
        if (socket._hooked) return;
        socket._hooked = true;

        try {
            socket._interceptUrl = url.toString();
        } catch (e) {}

        const originalSend = socket.send;
        socket.send = function(data) {
            const result = originalSend.apply(this, arguments);

            const rawSocketUrl = this.url || this._interceptUrl || '';
            
            // Extract sid from WebSocket URL for CloseOption
            if (platform === 'closeoption' && rawSocketUrl) {
                try {
                    const urlObj = new URL(rawSocketUrl);
                    const sid = urlObj.searchParams.get('sid');
                    if (sid && !window._coSid) {
                        window._coSid = sid;
                        if (DEBUG) console.log('[CO SSID] Extracted sid from WebSocket URL:', sid.substring(0, 6) + '...');
                    }
                } catch (e) {}
            }
            
            // Skip events-po.com (PocketOption analytics)
            let socketHost = '';
            try {
                socketHost = new URL(rawSocketUrl, window.location.href).hostname.toLowerCase();
            } catch (e) {}
            if (socketHost === 'events-po.com' || socketHost.endsWith('.events-po.com')) {
                return result;
            }

            // Log ALL PocketOption WebSocket traffic for debugging (DEBUG builds only)
            if (DEBUG && platform === 'pocketoption' && typeof data === 'string') {
                console.log('[PO SSID] OUTGOING:', data.substring(0, 500));
                
                // Check for auth in outgoing messages
                if (data.includes('"auth"')) {
                    const match = data.match(/42\["auth",\s*(\{.+?\})\s*\]/);
                    if (match) {
                        pocketOptionAuthMessage = match[0];
                        pocketOptionSsid = extractPocketOptionSsid(match[0]);
                        if (pocketOptionSsid) {
                            if (DEBUG) console.log('[PO SSID] Auth intercepted (send), SSID ready:', pocketOptionSsid.substring(0, 12) + '...');
                        }
                    }
                }
            }

            // Extract CloseOption credentials from WebSocket messages
            if (platform === 'closeoption' && typeof data === 'string') {
                // Look for _token and publicCode in outgoing messages
                if (data.includes('_token') || data.includes('publicCode') || data.includes('hiddenCode')) {
                    try {
                        // Try to extract from JSON payload
                        const jsonMatch = data.match(/42\[.+?,(\{.+})\]/);
                        if (jsonMatch) {
                            const payload = JSON.parse(jsonMatch[1]);
                            if (payload._token && !closeOptionCreds) {
                                // Store token for later use
                                window._coToken = payload._token;
                            }
                            if (payload.publicCode && !closeOptionCreds) {
                                window._coPublicCode = payload.publicCode;
                            }
                            if (payload.hiddenCode && !closeOptionCreds) {
                                window._coHiddenCode = payload.hiddenCode;
                            }
                        }
                    } catch (e) {}
                }
            }

            // Log CloseOption events for debugging (DEBUG builds only)
            if (DEBUG && platform === 'closeoption' && typeof data === 'string' && data.startsWith('42[')) {
                console.log('[CO SSID] Outgoing event:', data.substring(0, 100));
            }

            return result;
        };

        // Also hook onmessage to catch incoming PocketOption auth
        const originalOnMessage = socket.onmessage;
        socket.onmessage = function(event) {
            if (platform === 'pocketoption' && typeof event.data === 'string') {
                if (DEBUG) console.log('[PO SSID] INCOMING:', event.data.substring(0, 500));
                
                // Check for auth in incoming messages (may be concatenated)
                if (event.data.includes('"auth"')) {
                    const match = event.data.match(/42\["auth",\s*(\{.+?\})\s*\]/);
                    if (match) {
                        pocketOptionAuthMessage = match[0];
                        pocketOptionSsid = extractPocketOptionSsid(match[0]);
                        if (pocketOptionSsid) {
                            if (DEBUG) console.log('[PO SSID] Auth received (onmessage), SSID ready:', pocketOptionSsid.substring(0, 12) + '...');
                        }
                    }
                }
            }
            if (originalOnMessage) originalOnMessage.call(this, event);
        };
    }

    // Hook WebSocket constructor AND prototype methods
    function hookWebSocket() {
        const OriginalWebSocket = window.WebSocket;

        // 1. Hook constructor
        window.WebSocket = function(url, protocols) {
            const socket = new OriginalWebSocket(url, protocols);
            hookSocket(socket, url);
            return socket;
        };

        // Copy static properties
        Object.getOwnPropertyNames(OriginalWebSocket).forEach(prop => {
            if (prop !== 'prototype') {
                Object.defineProperty(window.WebSocket, prop, Object.getOwnPropertyDescriptor(OriginalWebSocket, prop));
            }
        });
        Object.getOwnPropertySymbols(OriginalWebSocket).forEach(sym => {
            Object.defineProperty(window.WebSocket, sym, Object.getOwnPropertyDescriptor(OriginalWebSocket, sym));
        });

        window.WebSocket.prototype = OriginalWebSocket.prototype;
        window.WebSocket.prototype.constructor = window.WebSocket;

        // 2. Also hook prototype.send and prototype.onmessage setter as fallback
        const proto = OriginalWebSocket.prototype;
        const originalProtoSend = proto.send;
        proto.send = function(data) {
            // This catches any WebSocket that wasn't caught by constructor hook
            if (!this._hooked) {
                hookSocket(this, this.url || 'unknown');
            }
            return originalProtoSend.apply(this, arguments);
        };

        // Hook onmessage setter
        const originalOnMessageDescriptor = Object.getOwnPropertyDescriptor(proto, 'onmessage');
        if (originalOnMessageDescriptor && originalOnMessageDescriptor.set) {
            Object.defineProperty(proto, 'onmessage', {
                set: function(handler) {
                    if (!this._hooked) {
                        hookSocket(this, this.url || 'unknown');
                    }
                    return originalOnMessageDescriptor.set.call(this, handler);
                },
                get: originalOnMessageDescriptor.get,
                configurable: true
            });
        }

        console.log(`[SSID] WebSocket constructor + prototype hooked for ${platform}`);
    }

    // Scan for existing WebSocket instances on the page
    function scanExistingWebSockets() {
        // Check for WebSocket instances stored in common places
        const checkObj = (obj, path) => {
            if (!obj || typeof obj !== 'object') return;
            if (obj instanceof WebSocket) {
                console.log('[PO SSID] Found existing WebSocket at:', path);
                hookSocket(obj, obj.url || 'unknown');
            }
            // Recursively check properties (but avoid circular refs)
            const seen = new WeakSet();
            const recurse = (o, p) => {
                if (!o || typeof o !== 'object' || seen.has(o)) return;
                seen.add(o);
                for (const key of Object.keys(o)) {
                    const val = o[key];
                    if (val instanceof WebSocket) {
                        console.log('[PO SSID] Found existing WebSocket at:', p + '.' + key);
                        hookSocket(val, val.url || 'unknown');
                    } else if (val && typeof val === 'object') {
                        recurse(val, p + '.' + key);
                    }
                }
            };
            recurse(obj, path);
        };

        // Check common global objects that might hold WebSocket references
        checkObj(window, 'window');
        checkObj(window.__PO_WS__, 'window.__PO_WS__');
        checkObj(window.socket, 'window.socket');
        checkObj(window.ws, 'window.ws');
        checkObj(window.websocket, 'window.websocket');
    }

    // ==================== CLIPBOARD & NOTIFICATIONS ====================

    // Copy to clipboard
    function copyToClipboard(text) {
        if (typeof GM_setClipboard !== 'undefined') {
            GM_setClipboard(text);
            return Promise.resolve();
        }
        return navigator.clipboard.writeText(text).catch(() => {
            const ta = document.createElement('textarea');
            ta.value = text;
            ta.style.position = 'fixed';
            ta.style.opacity = '0';
            document.body.appendChild(ta);
            ta.select();
            document.execCommand('copy');
            document.body.removeChild(ta);
        });
    }

    // Notify user (GM_notification or console)
    function notify(title, text, type = 'info') {
        if (typeof GM_notification !== 'undefined') {
            GM_notification({ title, text, timeout: 3000, type });
        } else {
            console.log(`[${title}] ${text}`);
        }
    }

    // ==================== MENU COMMANDS ====================

    // Register menu commands (Violentmonkey/Tampermonkey)
    if (typeof GM_registerMenuCommand !== 'undefined') {
        // PocketOption: Extract & Copy SSID
        if (platform === 'pocketoption') {
            GM_registerMenuCommand('Extract & Copy PocketOption SSID', async () => {
                if (!pocketOptionSsid) {
                    notify('PocketOption SSID', 'No auth message intercepted yet. Make sure you are logged in and the WebSocket has connected.', 'warning');
                    return;
                }

                try {
                    await copyToClipboard(pocketOptionSsid);
                    notify('PocketOption SSID', 'Full 42["auth",{...}] SSID copied to clipboard!', 'success');
                } catch (e) {
                    notify('PocketOption SSID', 'Failed to copy to clipboard', 'error');
                }
            });

            // Debug command for PocketOption
            GM_registerMenuCommand('Debug: Show PocketOption WebSocket Traffic', () => {
                console.log('[PO SSID] Last auth message:', pocketOptionAuthMessage);
                console.log('[PO SSID] Extracted SSID:', pocketOptionSsid);
                notify('PocketOption Debug', 'Check console for WebSocket traffic', 'info');
            });

            // Force re-scan for WebSockets
            GM_registerMenuCommand('Debug: Re-scan for WebSockets', () => {
                scanExistingWebSockets();
                notify('PocketOption Debug', 'Re-scanned for WebSocket instances', 'info');
            });
        }

        // CloseOption: Extract & Copy Credentials
        if (platform === 'closeoption') {
            GM_registerMenuCommand('Extract & Copy CloseOption Credentials', async () => {
                // First try stored credentials
                let creds = extractCloseOptionCreds();
                
                // If not found, try to build from WebSocket-captured values
                if (!creds) {
                    const token = window._coToken || localStorage.getItem('token') || localStorage.getItem('_token') || getCookie('token') || getCookie('_token') || getCookie('XSRF-TOKEN');
                    const sid = window._coSid || localStorage.getItem('sid') || localStorage.getItem('socket_sid') || localStorage.getItem('session_id') || getCookie('sid') || getCookie('socket_sid') || getCookie('session_id');
                    const publicCode = window._coPublicCode || localStorage.getItem('public_code') || localStorage.getItem('publicCode') || getCookie('public_code') || getCookie('publicCode');
                    const hiddenCode = window._coHiddenCode || localStorage.getItem('hidden_code') || localStorage.getItem('hiddenCode') || getCookie('hidden_code') || getCookie('hiddenCode');
                    
                    if (token && sid && publicCode && hiddenCode) {
                        const demo = detectDemo() ? '1' : '0';
                        creds = `${token}|${sid}|${demo}|${publicCode}|${hiddenCode}`;
                    }
                }
                
                if (!creds) {
                    notify('CloseOption Credentials', 'Could not find all required credentials (token, sid, public_code, hidden_code). Make sure you are logged in and have made a request.', 'warning');
                    return;
                }

                try {
                    await copyToClipboard(creds);
                    notify('CloseOption Credentials', 'Credentials copied to clipboard! Format: token|sid|demo|public_code|hidden_code', 'success');
                } catch (e) {
                    notify('CloseOption Credentials', 'Failed to copy to clipboard', 'error');
                }
            });

            // Also add a debug command to show what's in localStorage
            GM_registerMenuCommand('Debug: Show CloseOption Storage', () => {
                const keys = ['token', '_token', 'sid', 'socket_sid', 'session_id', 'public_code', 'publicCode', 'hidden_code', 'hiddenCode', 'auth_token', 'access_token'];
                const found = {};
                keys.forEach(k => {
                    const v = localStorage.getItem(k);
                    if (v) found[k] = v.substring(0, 30) + (v.length > 30 ? '...' : '');
                });
                console.log('[CO SSID] localStorage:', found);
                console.log('[CO SSID] Cookies:', document.cookie);
                console.log('[CO SSID] Captured from WS:', { _coToken: window._coToken, _coSid: window._coSid, _coPublicCode: window._coPublicCode, _coHiddenCode: window._coHiddenCode });
                notify('CloseOption Debug', 'Check console for storage contents', 'info');
            });
        }
    }

    // ==================== INITIALIZATION ====================

    function init() {
        // WebSocket hooking happens immediately at document-start (see below)
        
        // For CloseOption, also try to extract credentials immediately
        if (platform === 'closeoption') {
            const creds = extractCloseOptionCreds();
            if (creds) {
                closeOptionCreds = creds;
                if (DEBUG) console.log('[CO SSID] Credentials found on load:', creds.substring(0, 30) + '...');
            } else {
                console.log('[CO SSID] Credentials not found yet, will try on menu click');
            }
        }

        // For PocketOption, scan for existing WebSocket instances
        if (platform === 'pocketoption') {
            // Scan immediately
            scanExistingWebSockets();
            
            // Scan again after a delay (in case WebSocket is created later)
            setTimeout(scanExistingWebSockets, 1000);
            setTimeout(scanExistingWebSockets, 3000);
            
            console.log('[PO SSID] Initialization complete, waiting for auth message...');
        }

        console.log(`[SSID] Ready for ${platform}. Use Violentmonkey/Tampermonkey menu to extract credentials.`);
    }

    // Hook WebSocket immediately at document-start so early connections are captured
    hookWebSocket();
    // Run at document-start to catch early WebSocket creation
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();