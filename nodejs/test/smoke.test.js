"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");

const {
  PocketOption,
  Validator,
  RawHandle,
  RawHandler,
  CandleStream,
  RawStream,
  startLogs,
} = require("..");

test("exports the documented surface", () => {
  for (const value of [PocketOption, Validator, RawHandle, RawHandler, CandleStream, RawStream]) {
    assert.equal(typeof value, "function");
  }
  assert.equal(typeof startLogs, "function");
});

test("validators match the same way as the other bindings", () => {
  assert.equal(new Validator().check("hello"), true);

  const regex = Validator.regex("([A-Z])\\w+");
  assert.equal(regex.check("Hello"), true);
  assert.equal(regex.check("hello"), false);

  const startsWith = Validator.startsWith("Hello");
  assert.equal(startsWith.check("Hello World"), true);
  assert.equal(startsWith.check("hi World"), false);

  const endsWith = Validator.endsWith("Bye");
  assert.equal(endsWith.check("Hello Bye"), true);
  assert.equal(endsWith.check("Hello there"), false);

  const contains = Validator.contains("World");
  assert.equal(contains.check("Hello World"), true);
  assert.equal(contains.check("Hello there"), false);

  const negated = Validator.ne(contains);
  assert.equal(negated.check("Hello World"), false);
  assert.equal(negated.check("Hello there"), true);

  const all = Validator.all([regex, startsWith]);
  assert.equal(all.check("Hello World"), true);
  assert.equal(all.check("hello World"), false);
  assert.equal(all.check("Hey there"), false);

  const any = Validator.any([contains, endsWith]);
  assert.equal(any.check("Hello World"), true);
  assert.equal(any.check("Hello Bye"), true);
  assert.equal(any.check("Hello there"), false);
});

test("an invalid regex is reported instead of matching nothing", () => {
  assert.throws(() => Validator.regex("([A-Z]"), /InvalidRegexError/);
});

test("snake_case aliases resolve to the camelCase methods", () => {
  assert.equal(Validator.starts_with, Validator.startsWith);
  assert.equal(Validator.ends_with, Validator.endsWith);
  assert.equal(PocketOption.create_with_url, PocketOption.createWithUrl);
  assert.equal(PocketOption.prototype.get_candles, PocketOption.prototype.getCandles);
  assert.equal(PocketOption.prototype.check_win, PocketOption.prototype.checkWin);
  assert.equal(RawHandler.prototype.send_and_wait, RawHandler.prototype.sendAndWait);
});

test("streams are async iterable", () => {
  assert.equal(typeof CandleStream.prototype[Symbol.asyncIterator], "function");
  assert.equal(typeof RawStream.prototype[Symbol.asyncIterator], "function");
});

test("the constructor does not block and surfaces connection errors", async () => {
  const started = Date.now();
  const api = new PocketOption("not-a-session");
  assert.ok(Date.now() - started < 1000, "the constructor must return immediately");

  await assert.rejects(api.balance(), /Failed to parse ssid/);
  // The same failed connection attempt is reused by every method.
  await assert.rejects(api.ready(), /Failed to parse ssid/);
});
