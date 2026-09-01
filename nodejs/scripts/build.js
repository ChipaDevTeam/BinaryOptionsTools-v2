#!/usr/bin/env node
"use strict";

/**
 * Builds the native addon with cargo and copies it next to `index.js` as
 * `binary-options-tools.node`, which is the name `require()` expects.
 *
 * Usage: node scripts/build.js [--release] [extra cargo args...]
 */

const { spawnSync } = require("node:child_process");
const { copyFileSync, existsSync } = require("node:fs");
const { join, resolve } = require("node:path");

const CRATE = "binary_options_tools_napi";
const packageRoot = resolve(__dirname, "..");
const repoRoot = resolve(packageRoot, "..");

const args = process.argv.slice(2);
const release = args.includes("--release");

const cargo = spawnSync(
  process.env.CARGO || "cargo",
  ["build", "-p", CRATE, ...args],
  { cwd: repoRoot, stdio: "inherit" },
);

if (cargo.error) {
  console.error(`Failed to run cargo: ${cargo.error.message}`);
  console.error("Install the Rust toolchain from https://rustup.rs and try again.");
  process.exit(1);
}
if (cargo.status !== 0) {
  process.exit(cargo.status ?? 1);
}

const suffix = { win32: ".dll", darwin: ".dylib" }[process.platform] || ".so";
const prefix = process.platform === "win32" ? "" : "lib";
const artifact = join(repoRoot, "target", release ? "release" : "debug", `${prefix}${CRATE}${suffix}`);

if (!existsSync(artifact)) {
  console.error(`cargo succeeded but ${artifact} is missing.`);
  process.exit(1);
}

const destination = join(packageRoot, "binary-options-tools.node");
copyFileSync(artifact, destination);
console.log(`Wrote ${destination}`);
