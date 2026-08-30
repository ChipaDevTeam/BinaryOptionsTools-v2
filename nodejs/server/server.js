#!/usr/bin/env node
"use strict";

/**
 * A small HTTP wrapper around the native addon.
 *
 * The addon only runs inside Node, so a browser cannot load it directly. This
 * server does: it holds the clients, and the page in `public/` drives them over
 * HTTP. Run it on any machine and open it from a phone on the same network.
 *
 *   node server/server.js
 *
 * Environment:
 *   PORT           port to listen on                    (default 8787)
 *   HOST           interface to bind                    (default 127.0.0.1)
 *   AUTH_TOKEN     require ?token=... on every /api call (default none)
 *   ALLOW_TRADING  set to 1 to enable the trade endpoint (default off)
 *   SESSION_TTL_MS idle time before a session is dropped (default 1800000)
 */

const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const lib = require("..");
const { PocketOption, Validator } = lib;

const PORT = Number(process.env.PORT || 8787);
const HOST = process.env.HOST || "127.0.0.1";
const AUTH_TOKEN = process.env.AUTH_TOKEN || null;
const ALLOW_TRADING = process.env.ALLOW_TRADING === "1";
const SESSION_TTL_MS = Number(process.env.SESSION_TTL_MS || 30 * 60 * 1000);
const PUBLIC_DIR = path.join(__dirname, "public");
const ADDON = path.join(__dirname, "..", "binary-options-tools.node");

/** Sessions hold a connected client. The ssid stays here and is never sent back. */
const sessions = new Map();

setInterval(() => {
  const cutoff = Date.now() - SESSION_TTL_MS;
  for (const [id, session] of sessions) {
    if (session.touched < cutoff) {
      sessions.delete(id);
      session.client.shutdown().catch(() => {});
    }
  }
}, 60_000).unref();

// --- helpers --------------------------------------------------------------

/** The addon prefixes messages with the error kind; split it back out. */
function describeError(error) {
  const message = String((error && error.message) || error);
  const match = /^([A-Za-z]+Error): (.*)$/s.exec(message);
  return match ? { name: match[1], message: match[2] } : { name: "Error", message };
}

function send(res, status, body, headers) {
  const payload = JSON.stringify(body);
  res.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store",
    ...headers,
  });
  res.end(payload);
}

const fail = (res, status, name, message) => send(res, status, { error: { name, message } });

async function readJson(req) {
  const chunks = [];
  let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > 1_000_000) throw new Error("request body too large");
    chunks.push(chunk);
  }
  if (!chunks.length) return {};
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

function requireSession(res, id) {
  const session = sessions.get(id);
  if (!session) {
    fail(res, 404, "NoSuchSession", "That session has expired or never existed. Connect again.");
    return null;
  }
  session.touched = Date.now();
  return session;
}

/**
 * Builds a native Validator from a JSON spec:
 *   {"type":"contains","value":"World"}
 *   {"type":"all","of":[ ...specs ]}
 *   {"type":"not","of": ...spec }
 */
function buildValidator(spec) {
  if (!spec || typeof spec !== "object") throw new Error("a validator spec must be an object");
  switch (spec.type) {
    case "none":
      return new Validator();
    case "regex":
      return Validator.regex(String(spec.value ?? ""));
    case "contains":
      return Validator.contains(String(spec.value ?? ""));
    case "startsWith":
      return Validator.startsWith(String(spec.value ?? ""));
    case "endsWith":
      return Validator.endsWith(String(spec.value ?? ""));
    case "not":
      return Validator.ne(buildValidator(spec.of));
    case "all":
      return Validator.all((spec.of || []).map(buildValidator));
    case "any":
      return Validator.any((spec.of || []).map(buildValidator));
    default:
      throw new Error(`unknown validator type: ${JSON.stringify(spec.type)}`);
  }
}

const isPublic = (key) =>
  !["constructor", "prototype", "length", "name"].includes(key) && !key.includes("_");

function describeClass(Class) {
  const members = (target) =>
    Object.getOwnPropertyNames(target)
      .filter((k) => isPublic(k) && typeof target[k] === "function")
      .sort();
  return { instance: members(Class.prototype), static: members(Class) };
}

// --- routes ---------------------------------------------------------------

const routes = [];
const route = (method, pattern, handler) => routes.push({ method, pattern, handler });

route("GET", /^\/api\/health$/, async (req, res) => {
  let addon = null;
  try {
    const stat = fs.statSync(ADDON);
    addon = { path: ADDON, bytes: stat.size };
  } catch {
    addon = { path: ADDON, bytes: null };
  }
  send(res, 200, {
    ok: true,
    node: process.version,
    platform: `${process.platform}-${process.arch}`,
    addon,
    tradingEnabled: ALLOW_TRADING,
    sessions: sessions.size,
    uptimeSeconds: Math.round(process.uptime()),
  });
});

route("GET", /^\/api\/surface$/, async (req, res) => {
  const classes = ["PocketOption", "Validator", "RawHandle", "RawHandler", "CandleStream", "RawStream"];
  send(res, 200, {
    classes: classes.map((name) => ({ name, ...describeClass(lib[name]) })),
    functions: ["startLogs"],
  });
});

route("POST", /^\/api\/validator$/, async (req, res) => {
  const body = await readJson(req);
  if (typeof body.message !== "string") {
    return fail(res, 400, "InvalidParameterError", "Send a `message` string to check.");
  }
  const validator = buildValidator(body.validator);
  send(res, 200, { matches: validator.check(body.message) });
});

route("POST", /^\/api\/session$/, async (req, res) => {
  const body = await readJson(req);
  const ssid = typeof body.ssid === "string" ? body.ssid.trim() : "";
  if (!ssid) return fail(res, 400, "InvalidParameterError", "Send your PocketOption `ssid`.");

  const client = body.url ? PocketOption.withUrl(ssid, String(body.url)) : new PocketOption(ssid);
  await client.ready();

  const id = crypto.randomUUID();
  sessions.set(id, { client, touched: Date.now() });
  send(res, 201, { id, demo: await client.isDemo() });
});

route("GET", /^\/api\/session\/([\w-]+)$/, async (req, res, [id]) => {
  const session = requireSession(res, id);
  if (!session) return;
  send(res, 200, {
    id,
    connected: await session.client.isConnected(),
    demo: await session.client.isDemo(),
    serverTime: await session.client.serverTime(),
  });
});

route("DELETE", /^\/api\/session\/([\w-]+)$/, async (req, res, [id]) => {
  const session = sessions.get(id);
  if (!session) return fail(res, 404, "NoSuchSession", "That session has already gone.");
  sessions.delete(id);
  await session.client.shutdown().catch(() => {});
  send(res, 200, { closed: true });
});

route("GET", /^\/api\/session\/([\w-]+)\/balance$/, async (req, res, [id]) => {
  const session = requireSession(res, id);
  if (!session) return;
  send(res, 200, { balance: await session.client.balance() });
});

route("GET", /^\/api\/session\/([\w-]+)\/payout$/, async (req, res, [id]) => {
  const session = requireSession(res, id);
  if (!session) return;
  await session.client.waitForAssets(15).catch(() => {});
  send(res, 200, { payouts: await session.client.payout() });
});

route("GET", /^\/api\/session\/([\w-]+)\/candles$/, async (req, res, [id], url) => {
  const session = requireSession(res, id);
  if (!session) return;
  const asset = url.searchParams.get("asset") || "EURUSD_otc";
  const period = Number(url.searchParams.get("period") || 60);
  const candles = await session.client.candles(asset, period);
  send(res, 200, { asset, period, count: candles.length, candles: candles.slice(-120) });
});

route("GET", /^\/api\/session\/([\w-]+)\/ticks$/, async (req, res, [id], url) => {
  const session = requireSession(res, id);
  if (!session) return;
  const asset = url.searchParams.get("asset") || "EURUSD_otc";
  const seconds = Number(url.searchParams.get("seconds") || 300);
  const ticks = await session.client.getTicks(asset, seconds);
  send(res, 200, { asset, seconds, count: ticks.length, ticks: ticks.slice(-500) });
});

route("GET", /^\/api\/session\/([\w-]+)\/deals$/, async (req, res, [id]) => {
  const session = requireSession(res, id);
  if (!session) return;
  send(res, 200, {
    opened: await session.client.openedDeals(),
    closed: await session.client.closedDeals(),
  });
});

route("POST", /^\/api\/session\/([\w-]+)\/trade$/, async (req, res, [id]) => {
  if (!ALLOW_TRADING) {
    return fail(res, 403, "NotAllowedError", "Trading is off. Restart the server with ALLOW_TRADING=1 to place orders.");
  }
  const session = requireSession(res, id);
  if (!session) return;
  const body = await readJson(req);
  const direction = body.direction === "sell" ? "sell" : "buy";
  const asset = String(body.asset || "EURUSD_otc");
  const amount = Number(body.amount);
  const seconds = Number(body.seconds || 60);
  if (!Number.isFinite(amount) || amount <= 0) {
    return fail(res, 400, "InvalidParameterError", "`amount` must be a positive number.");
  }
  const [dealId, deal] = await session.client[direction](asset, amount, seconds);
  send(res, 201, { dealId, deal });
});

route("GET", /^\/api\/session\/([\w-]+)\/result\/([\w-]+)$/, async (req, res, [id, dealId]) => {
  const session = requireSession(res, id);
  if (!session) return;
  send(res, 200, { deal: await session.client.result(dealId) });
});

/** Server-sent events: one message per candle, until the client disconnects. */
route("GET", /^\/api\/session\/([\w-]+)\/stream$/, async (req, res, [id], url) => {
  const session = requireSession(res, id);
  if (!session) return;
  const asset = url.searchParams.get("asset") || "EURUSD_otc";
  const seconds = url.searchParams.get("seconds");

  const stream = seconds
    ? await session.client.subscribe(asset, Number(seconds))
    : await session.client.subscribeSymbol(asset);

  res.writeHead(200, {
    "content-type": "text/event-stream",
    "cache-control": "no-store",
    connection: "keep-alive",
  });
  res.write(`event: open\ndata: ${JSON.stringify({ asset })}\n\n`);

  let live = true;
  req.on("close", () => {
    live = false;
    session.client.unsubscribe(asset).catch(() => {});
  });

  try {
    for await (const candle of stream) {
      if (!live) break;
      session.touched = Date.now();
      res.write(`data: ${JSON.stringify(candle)}\n\n`);
    }
  } catch (error) {
    if (live) res.write(`event: error\ndata: ${JSON.stringify(describeError(error))}\n\n`);
  }
  res.end();
});

// --- static files ---------------------------------------------------------

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
};

function serveStatic(req, res, pathname) {
  const rel = pathname === "/" ? "index.html" : pathname.replace(/^\/+/, "");
  const file = path.join(PUBLIC_DIR, rel);
  if (!file.startsWith(PUBLIC_DIR)) return fail(res, 403, "Forbidden", "No.");
  fs.readFile(file, (error, data) => {
    if (error) return fail(res, 404, "NotFound", `No route or file for ${pathname}.`);
    res.writeHead(200, { "content-type": TYPES[path.extname(file)] || "application/octet-stream" });
    res.end(data);
  });
}

// --- server ---------------------------------------------------------------

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://${req.headers.host || "localhost"}`);
  const pathname = url.pathname;

  if (!pathname.startsWith("/api/")) return serveStatic(req, res, pathname);

  if (AUTH_TOKEN) {
    const supplied = url.searchParams.get("token") || req.headers["x-auth-token"];
    if (supplied !== AUTH_TOKEN) {
      return fail(res, 401, "Unauthorized", "Wrong or missing token.");
    }
  }

  for (const { method, pattern, handler } of routes) {
    const match = pattern.exec(pathname);
    if (!match) continue;
    if (req.method !== method) return fail(res, 405, "MethodNotAllowed", `Use ${method} here.`);
    try {
      return await handler(req, res, match.slice(1), url);
    } catch (error) {
      const described = describeError(error);
      if (!res.headersSent) return send(res, 500, { error: described });
      return res.end();
    }
  }

  fail(res, 404, "NotFound", `No route for ${req.method} ${pathname}.`);
});

server.listen(PORT, HOST, () => {
  console.log(`binary-options-tools API on http://${HOST}:${PORT}`);
  if (HOST === "127.0.0.1") {
    console.log("Bound to localhost. To reach it from a phone, run with HOST=0.0.0.0");
  } else {
    console.log("Reachable from your network. Anyone who can reach this port can use any");
    console.log("session you open here, so set AUTH_TOKEN unless the network is trusted.");
  }
  console.log(ALLOW_TRADING ? "Trading is ENABLED — orders placed here are real." : "Trading is off (set ALLOW_TRADING=1 to enable).");
});
