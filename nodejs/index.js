"use strict";

const { existsSync } = require("node:fs");
const { join, resolve } = require("node:path");

const CRATE = "binary_options_tools_napi";

/**
 * Every location the compiled addon may live in, in priority order: the
 * packaged copy first, then the cargo build directories so that a plain
 * `cargo build` is enough while developing.
 */
function candidatePaths() {
  const paths = [];
  if (process.env.BINARY_OPTIONS_TOOLS_NATIVE) {
    paths.push(resolve(process.env.BINARY_OPTIONS_TOOLS_NATIVE));
  }
  paths.push(join(__dirname, "binary-options-tools.node"));

  const suffix = { win32: ".dll", darwin: ".dylib" }[process.platform] || ".so";
  const prefix = process.platform === "win32" ? "" : "lib";
  const artifact = `${prefix}${CRATE}${suffix}`;
  const target = join(__dirname, "..", "target");
  for (const profile of ["release", "debug"]) {
    paths.push(join(target, profile, artifact));
  }
  return paths;
}

/**
 * `require` only understands the `.node` extension, so the raw cargo artifacts
 * (`.so`, `.dylib`, `.dll`) are opened directly instead.
 */
function loadFrom(path) {
  if (path.endsWith(".node")) {
    return require(path);
  }
  const shim = { exports: {} };
  process.dlopen(shim, path);
  return shim.exports;
}

function loadNative() {
  const candidates = candidatePaths();
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return loadFrom(candidate);
    }
  }
  throw new Error(
    [
      "Could not find the binary-options-tools native addon.",
      "Build it with `npm run build` from the nodejs/ directory",
      "(or `cargo build -p binary_options_tools_napi --release` from the repository root).",
      "Looked in:",
      ...candidates.map((path) => `  - ${path}`),
    ].join("\n"),
  );
}

const native = loadNative();

const toSnakeCase = (name) => name.replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`);

/**
 * Adds a `snake_case` alias for every `camelCase` method of `target`.
 *
 * The Rust bindings expose idiomatic JavaScript names, while the examples and
 * the documentation of the other language bindings use `snake_case`. Both
 * spellings resolve to the same function.
 */
function addSnakeCaseAliases(target) {
  const skip = new Set(["constructor", "prototype", "length", "name"]);
  for (const key of Object.getOwnPropertyNames(target)) {
    if (skip.has(key)) continue;

    const descriptor = Object.getOwnPropertyDescriptor(target, key);
    if (!descriptor || typeof descriptor.value !== "function") continue;

    const alias = toSnakeCase(key);
    if (alias === key || Object.prototype.hasOwnProperty.call(target, alias)) continue;

    Object.defineProperty(target, alias, {
      value: descriptor.value,
      writable: true,
      configurable: true,
      enumerable: false,
    });
  }
}

/** Lets `for await (const candle of stream)` drive a native stream. */
function makeAsyncIterable(Class) {
  Object.defineProperty(Class.prototype, Symbol.asyncIterator, {
    enumerable: false,
    value: function () {
      const stream = this;
      return {
        async next() {
          const value = await stream.next();
          return value === null || value === undefined
            ? { done: true, value: undefined }
            : { done: false, value };
        },
        [Symbol.asyncIterator]() {
          return this;
        },
      };
    },
  });
}

const CLASSES = [
  "PocketOption",
  "RawHandle",
  "RawHandler",
  "Validator",
  "CandleStream",
  "RawStream",
];

for (const name of CLASSES) {
  addSnakeCaseAliases(native[name]);
  addSnakeCaseAliases(native[name].prototype);
}

makeAsyncIterable(native.CandleStream);
makeAsyncIterable(native.RawStream);

module.exports = {
  PocketOption: native.PocketOption,
  Validator: native.Validator,
  RawHandle: native.RawHandle,
  RawHandler: native.RawHandler,
  CandleStream: native.CandleStream,
  RawStream: native.RawStream,
  startLogs: native.startLogs,
  start_logs: native.startLogs,
};
