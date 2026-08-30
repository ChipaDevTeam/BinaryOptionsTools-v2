// ==UserScript==
// @name         CloseOption SSID Extractor
// @namespace    http://tampermonkey.net/
// @version      1.0
// @description  Extract CloseOption SSID (token|sid|demo|public_code|hidden_code) for BinaryOptionsToolsV2
// @author       Six
// @match        https://www.closeoption.com/*
// @match        https://closeoption.com/*
// @grant        GM_setClipboard
// @grant        GM_registerMenuCommand
// @run-at       document-idle
// ==/UserScript==

(function() {
    'use strict';

    // State
    let ssid = null;
    let isDemo = false;

    // Detect if we're on a demo page
    function detectDemo() {
        const path = window.location.pathname;
        return path.includes('/demo') || path.includes('demo');
    }

    // Extract publicCode and hiddenCode from hidden inputs
    function extractCodes() {
        const publicCodeInput = document.querySelector('input[name="publicCode"]');
        const hiddenCodeInput = document.querySelector('input[name="hiddenCode"]');
        
        const publicCode = publicCodeInput ? publicCodeInput.value : '';
        const hiddenCode = hiddenCodeInput ? hiddenCodeInput.value : '';
        
        return { publicCode, hiddenCode };
    }

    // Get JWT token from the API endpoint
    async function getJwtToken() {
        try {
            const resp = await fetch('/dashboard/jwt-token/get', {
                headers: { 'X-Requested-With': 'XMLHttpRequest' },
                credentials: 'include',
            });
            if (resp.ok) {
                const data = await resp.json();
                return data.jwt || '';
            }
        } catch (e) {
            console.error('[CloseOption SSID] Failed to get JWT:', e);
        }
        return '';
    }

    // Establish socket.io connection to get sid
    function getSocketSid() {
        return new Promise((resolve) => {
            // Make a fresh socket.io polling handshake to get a new sid
            const xhr = new XMLHttpRequest();
            const url = `https://www.closeoption.com:8443/socket.io/?EIO=3&transport=polling&t=${Date.now()}`;

            xhr.open('GET', url, true);
            xhr.withCredentials = true;

            xhr.onload = function() {
                try {
                    const resp = xhr.responseText;
                    // Parse socket.io handshake response: "0{...}2:40"
                    const match = resp.match(/"sid":"([^"]+)"/);
                    if (match) {
                        resolve(match[1]);
                    } else {
                        console.error('[CloseOption SSID] Could not parse sid from:', resp.slice(0, 200));
                        resolve('');
                    }
                } catch (e) {
                    console.error('[CloseOption SSID] Error parsing sid:', e);
                    resolve('');
                }
            };

            xhr.onerror = function() {
                console.error('[CloseOption SSID] Failed to connect to socket.io');
                resolve('');
            };

            xhr.send();
        });
    }

    // Build SSID string
    function buildSsid(token, sid, demo, publicCode, hiddenCode) {
        return `${token}|${sid}|${demo}|${publicCode}|${hiddenCode}`;
    }

    // Main extraction function
    async function extractSsid() {
        console.log('[CloseOption SSID] Starting extraction...');
        
        isDemo = detectDemo();
        console.log(`[CloseOption SSID] Account type: ${isDemo ? 'DEMO' : 'REAL'}`);

        // Get codes from page
        const { publicCode, hiddenCode } = extractCodes();
        if (!publicCode || !hiddenCode) {
            console.error('[CloseOption SSID] Missing publicCode or hiddenCode. Navigate to trade room first.');
            return null;
        }
        console.log(`[CloseOption SSID] Found codes: public=${publicCode.slice(0, 10)}..., hidden=${hiddenCode.slice(0, 10)}...`);

        // Get JWT token
        const token = await getJwtToken();
        if (!token) {
            console.error('[CloseOption SSID] Failed to get JWT token. Make sure you are logged in.');
            return null;
        }
        console.log(`[CloseOption SSID] Got JWT token: ${token.slice(0, 20)}...`);

        // Get socket sid
        const sid = await getSocketSid();
        if (!sid) {
            console.error('[CloseOption SSID] Failed to get socket sid. Make sure socket.io connection is established.');
            return null;
        }
        console.log(`[CloseOption SSID] Got socket sid: ${sid}`);

        // Build SSID
        ssid = buildSsid(token, sid, isDemo, publicCode, hiddenCode);
        console.log(`[CloseOption SSID] Extracted SSID: ${ssid.slice(0, 50)}...`);
        
        return ssid;
    }

    // Copy to clipboard
    function copyToClipboard(text) {
        if (typeof GM_setClipboard !== 'undefined') {
            GM_setClipboard(text);
        } else {
            navigator.clipboard.writeText(text).catch(() => {
                const ta = document.createElement('textarea');
                ta.value = text;
                document.body.appendChild(ta);
                ta.select();
                document.execCommand('copy');
                document.body.removeChild(ta);
            });
        }
    }

    // Create UI button
    function createButton() {
        const btn = document.createElement('button');
        btn.textContent = 'Copy SSID';
        btn.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            z-index: 999999;
            padding: 12px 24px;
            background: #2563eb;
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            font-family: system-ui, -apple-system, sans-serif;
        `;
        btn.addEventListener('mouseenter', () => btn.style.background = '#1d4ed8');
        btn.addEventListener('mouseleave', () => btn.style.background = '#2563eb');
        btn.addEventListener('click', async () => {
            if (!ssid) {
                btn.textContent = 'Extracting...';
                btn.disabled = true;
                const result = await extractSsid();
                if (result) {
                    copyToClipboard(result);
                    btn.textContent = 'Copied!';
                    setTimeout(() => btn.textContent = 'Copy SSID', 2000);
                } else {
                    btn.textContent = 'Failed - check console';
                    setTimeout(() => btn.textContent = 'Copy SSID', 3000);
                }
                btn.disabled = false;
            } else {
                copyToClipboard(ssid);
                btn.textContent = 'Copied!';
                setTimeout(() => btn.textContent = 'Copy SSID', 2000);
            }
        });
        document.body.appendChild(btn);
    }

    // Auto-extract on trade room pages
    async function autoExtract() {
        const path = window.location.pathname;
        if (path.includes('/trade/room/')) {
            console.log('[CloseOption SSID] Detected trade room, auto-extracting...');
            const result = await extractSsid();
            if (result) {
                console.log('[CloseOption SSID] Auto-extraction complete. Click "Copy SSID" button to copy.');
            }
        }
    }

    // Initialize
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            createButton();
            autoExtract();
        });
    } else {
        createButton();
        autoExtract();
    }

    // Register menu command
    if (typeof GM_registerMenuCommand !== 'undefined') {
        GM_registerMenuCommand('Extract SSID', async () => {
            const result = await extractSsid();
            if (result) {
                copyToClipboard(result);
                alert('SSID copied to clipboard!');
            }
        });
    }
})();
