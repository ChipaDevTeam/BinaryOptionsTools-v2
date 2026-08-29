/**
 * Node.js bindings for BinaryOptionsToolsV2.
 *
 * Every method is also available under its `snake_case` name, so
 * `api.getCandles(...)` and `api.get_candles(...)` are the same function.
 */

/** A single OHLC candle, as produced by the candle helpers. */
export interface Candle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
  [key: string]: unknown;
}

/** A trade, either running or settled. */
export interface Deal {
  id: string;
  asset: string;
  amount: number;
  profit: number;
  [key: string]: unknown;
}

/** Options accepted by {@link startLogs}. */
export interface LogOptions {
  /** Directory that receives `logs.log` and `error.log`. Defaults to `"."`. */
  path?: string;
  /** `TRACE`, `DEBUG`, `INFO`, `WARN` or `ERROR`. Defaults to `DEBUG`. */
  level?: string;
  /** Also print the logs to the terminal. Defaults to `false`. */
  terminal?: boolean;
}

/** Client configuration; omitted fields keep the library default. */
export interface ClientConfig {
  maxAllowedLoops?: number;
  sleepIntervalMs?: number;
  reconnectTimeSecs?: number;
  connectionInitializationTimeoutSecs?: number;
  timeoutSecs?: number;
  urls?: string[];
  proxy?: string;
  userAgent?: string;
  origin?: string;
  secWebsocketExtensions?: string;
  tlsCipherSuites?: string[];
  tlsAlpn?: string[];
}

/**
 * Installs the global tracing subscriber. Calling it more than once is a
 * no-op: the first subscriber wins.
 */
export function startLogs(options?: LogOptions): void;
export { startLogs as start_logs };

/**
 * Filters raw WebSocket messages.
 *
 * `new Validator()` accepts every message. Unlike the Python bindings there is
 * no `custom` constructor: a JavaScript callback cannot be invoked
 * synchronously from the WebSocket thread, so apply custom predicates to the
 * values yielded by {@link RawHandler.subscribe} instead.
 */
export class Validator {
  constructor();
  static regex(pattern: string): Validator;
  static contains(pattern: string): Validator;
  static startsWith(pattern: string): Validator;
  static starts_with(pattern: string): Validator;
  static endsWith(pattern: string): Validator;
  static ends_with(pattern: string): Validator;
  /** Negates `validator`. */
  static ne(validator: Validator): Validator;
  /** Matches only when every validator matches. */
  static all(validators: Validator[]): Validator;
  /** Matches when at least one validator matches. */
  static any(validators: Validator[]): Validator;
  check(message: string): boolean;
}

/** Async iterator over the candles of a symbol subscription. */
export class CandleStream implements AsyncIterable<Candle> {
  /** Resolves with the next candle, or `null` once the stream ends. */
  next(): Promise<Candle | null>;
  [Symbol.asyncIterator](): AsyncIterator<Candle>;
}

/** Async iterator over raw WebSocket messages. */
export class RawStream implements AsyncIterable<string> {
  /** Resolves with the next message, or `null` once the stream ends. */
  next(): Promise<string | null>;
  [Symbol.asyncIterator](): AsyncIterator<string>;
}

/** A registered raw message handler. */
export class RawHandler {
  id(): string;
  sendText(text: string): Promise<void>;
  send_text(text: string): Promise<void>;
  /** Alias of {@link RawHandler.sendText}. */
  send(text: string): Promise<void>;
  sendBinary(data: Buffer | Uint8Array): Promise<void>;
  send_binary(data: Buffer | Uint8Array): Promise<void>;
  sendAndWait(message: string): Promise<string>;
  send_and_wait(message: string): Promise<string>;
  sendAndWaitWithTimeout(message: string, timeoutMs: number): Promise<string>;
  send_and_wait_with_timeout(message: string, timeoutMs: number): Promise<string>;
  waitNext(): Promise<string>;
  wait_next(): Promise<string>;
  subscribe(): RawStream;
}

/** Entry point of the raw message module. */
export class RawHandle {
  create(validator: Validator, keepAlive?: string | null): Promise<RawHandler>;
  /** Resolves to `true` when a handler with that id existed. */
  remove(id: string): Promise<boolean>;
}

/**
 * PocketOption trading client.
 *
 * The constructor returns immediately and connects in the background; every
 * method awaits that connection first, so there is no need to sleep after
 * construction. Use the static `create` helpers when a connection failure
 * should be reported up front.
 */
export class PocketOption {
  constructor(ssid: string);

  static withUrl(ssid: string, url: string): PocketOption;
  static with_url(ssid: string, url: string): PocketOption;
  static withConfig(ssid: string, config: ClientConfig): PocketOption;
  static with_config(ssid: string, config: ClientConfig): PocketOption;

  static create(ssid: string): Promise<PocketOption>;
  static createWithUrl(ssid: string, url: string): Promise<PocketOption>;
  static create_with_url(ssid: string, url: string): Promise<PocketOption>;
  static createWithConfig(ssid: string, config: ClientConfig): Promise<PocketOption>;
  static create_with_config(ssid: string, config: ClientConfig): Promise<PocketOption>;

  /** Resolves when the initial connection succeeded, rejects otherwise. */
  ready(): Promise<void>;
  waitForAssets(timeoutSecs: number): Promise<void>;
  wait_for_assets(timeoutSecs: number): Promise<void>;
  isDemo(): Promise<boolean>;
  is_demo(): Promise<boolean>;
  isConnected(): Promise<boolean>;
  is_connected(): Promise<boolean>;

  balance(): Promise<number>;
  /** Opens a call trade. Resolves to `[dealId, deal]`. */
  buy(asset: string, amount: number, time: number): Promise<[string, Deal]>;
  /** Opens a put trade. Resolves to `[dealId, deal]`. */
  sell(asset: string, amount: number, time: number): Promise<[string, Deal]>;
  /** Waits for the trade to settle. */
  result(tradeId: string): Promise<Deal>;
  /** Alias of {@link PocketOption.result}. */
  checkWin(tradeId: string): Promise<Deal>;
  check_win(tradeId: string): Promise<Deal>;
  getDealEndTime(tradeId: string): Promise<number | null>;
  get_deal_end_time(tradeId: string): Promise<number | null>;

  candles(asset: string, period: number): Promise<Candle[]>;
  /** Alias of {@link PocketOption.candles}. */
  history(asset: string, period: number): Promise<Candle[]>;
  getCandles(asset: string, period: number, offset: number): Promise<Candle[]>;
  get_candles(asset: string, period: number, offset: number): Promise<Candle[]>;
  getCandlesAdvanced(asset: string, period: number, offset: number, time: number): Promise<Candle[]>;
  get_candles_advanced(asset: string, period: number, offset: number, time: number): Promise<Candle[]>;
  /** Raw tick history as `[timestamp, price]` pairs. */
  getTicks(asset: string, lookbackSeconds: number): Promise<Array<[number, number]>>;
  get_ticks(asset: string, lookbackSeconds: number): Promise<Array<[number, number]>>;
  compileCandles(asset: string, customPeriod: number, lookbackPeriod: number): Promise<Candle[]>;
  compile_candles(asset: string, customPeriod: number, lookbackPeriod: number): Promise<Candle[]>;

  /** Payout percentage of every currently active asset. */
  payout(): Promise<Record<string, number>>;
  activeAssets(): Promise<unknown>;
  active_assets(): Promise<unknown>;

  closedDeals(): Promise<Record<string, Deal>>;
  closed_deals(): Promise<Record<string, Deal>>;
  getClosedDeal(tradeId: string): Promise<Deal | null>;
  get_closed_deal(tradeId: string): Promise<Deal | null>;
  clearClosedDeals(): Promise<void>;
  clear_closed_deals(): Promise<void>;
  openedDeals(): Promise<Record<string, Deal>>;
  opened_deals(): Promise<Record<string, Deal>>;
  getOpenedDeal(tradeId: string): Promise<Deal | null>;
  get_opened_deal(tradeId: string): Promise<Deal | null>;

  openPendingOrder(
    openType: number,
    amount: number,
    asset: string,
    openTime: string,
    openPrice: number,
    timeframe: number,
    minPayout: number,
    command: number,
  ): Promise<unknown>;
  open_pending_order(
    openType: number,
    amount: number,
    asset: string,
    openTime: string,
    openPrice: number,
    timeframe: number,
    minPayout: number,
    command: number,
  ): Promise<unknown>;
  cancelPendingOrder(ticket: string): Promise<{ ticket: string; status: string }>;
  cancel_pending_order(ticket: string): Promise<{ ticket: string; status: string }>;
  cancelPendingOrders(tickets: string[]): Promise<{ cancelled: string[] }>;
  cancel_pending_orders(tickets: string[]): Promise<{ cancelled: string[] }>;

  sendRaw(message: string): Promise<void>;
  send_raw(message: string): Promise<void>;
  sendRawMessage(message: string): Promise<void>;
  send_raw_message(message: string): Promise<void>;
  subscribeRaw(): Promise<RawStream>;
  subscribe_raw(): Promise<RawStream>;

  /** Without `seconds` every update is yielded; with it they are aggregated. */
  subscribe(symbol: string, seconds?: number): Promise<CandleStream>;
  subscribeSymbol(symbol: string): Promise<CandleStream>;
  subscribe_symbol(symbol: string): Promise<CandleStream>;
  subscribeSymbolChunked(symbol: string, chunkSize: number): Promise<CandleStream>;
  subscribe_symbol_chunked(symbol: string, chunkSize: number): Promise<CandleStream>;
  subscribeSymbolTimed(symbol: string, seconds: number): Promise<CandleStream>;
  subscribe_symbol_timed(symbol: string, seconds: number): Promise<CandleStream>;
  subscribeSymbolTimeAligned(symbol: string, seconds: number): Promise<CandleStream>;
  subscribe_symbol_time_aligned(symbol: string, seconds: number): Promise<CandleStream>;
  unsubscribe(asset: string): Promise<void>;

  rawHandle(): Promise<RawHandle>;
  raw_handle(): Promise<RawHandle>;
  createRawHandler(validator: Validator, keepAlive?: string | null): Promise<RawHandler>;
  create_raw_handler(validator: Validator, keepAlive?: string | null): Promise<RawHandler>;
  createRawOrder(message: string, validator: Validator): Promise<string>;
  create_raw_order(message: string, validator: Validator): Promise<string>;
  createRawOrderWithTimeout(message: string, validator: Validator, timeoutMs: number): Promise<string>;
  create_raw_order_with_timeout(message: string, validator: Validator, timeoutMs: number): Promise<string>;
  createRawOrderWithTimeoutAndRetry(message: string, validator: Validator, timeoutMs: number): Promise<string>;
  create_raw_order_with_timeout_and_retry(message: string, validator: Validator, timeoutMs: number): Promise<string>;
  createRawIterator(message: string, validator: Validator, timeoutMs?: number | null): Promise<RawStream>;
  create_raw_iterator(message: string, validator: Validator, timeoutMs?: number | null): Promise<RawStream>;

  serverTime(): Promise<number>;
  server_time(): Promise<number>;
  getServerTime(): Promise<number>;
  get_server_time(): Promise<number>;

  shutdown(): Promise<void>;
  disconnect(): Promise<void>;
  connect(): Promise<void>;
  reconnect(): Promise<void>;
}
