# BinaryOptionsTools-v2 Security & Trading Audit Report

**Auditor Role:** Senior Systems Engineer and Quantitative Trading Auditor
**Date:** January 2025
**Repository:** BinaryOptionsTools-v2
**Tech Stack:** Rust (Tokio, Tungstenite), Python (PyO3, Asyncio), WebSocket

---

## Executive Summary

This audit identified **15 issues** across trading safety, memory management, async correctness, and security. **4 CRITICAL** issues could lead to financial loss through race conditions, double-trading, or failed execution. **3 HIGH** security/reliability issues require immediate attention. The codebase demonstrates solid architectural patterns but has critical gaps in edge-case handling for disconnection scenarios and concurrent trade execution.

---

## 🚨 CRITICAL: TRADING & FINANCIAL RISKS

### 1. **Race Condition in Concurrent Trade Execution**
- **Location:** `crates/binary_options_tools/src/pocketoption/modules/trades.rs:82-101`
- **Severity:** CRITICAL
- **Risk:** When multiple trades are executed concurrently using the same `TradesHandle`, responses can be misrouted. The module uses a shared `AsyncReceiver<CommandResponse>` without proper per-request isolation. While UUID matching provides some safety, the loop pattern `recv().await` → check UUID → continue if mismatch creates a window where responses can pile up in the channel and be consumed by the wrong caller.

**Vulnerable Code:**
```rust
pub async fn trade(&self, ...) -> PocketResult {
    let id = Uuid::new_v4();
    self.sender.send(Command::OpenOrder { ..., req_id: id }).await?;
    loop {
        match self.receiver.recv().await {  // ← SHARED RECEIVER
            Ok(CommandResponse::Success { req_id, deal }) => {
                if req_id == id {
                    return Ok(*deal);
                } else {
                    continue;  // ← Response queues up for wrong caller
                }
            }
            ...
        }
    }
}
```

**Impact:**
- Thread A calls `buy()` for EURUSD
- Thread B calls `sell()` for BTCUSD simultaneously
- Both responses arrive, but Thread A might get Thread B's response first
- While UUID check will reject it, Thread B's response is now stuck in Thread A's receive loop
- Potential deadlock or response timeout

**Fix:**
```rust
// Create per-request channels using oneshot
use tokio::sync::oneshot;
pub enum Command {
    OpenOrder {
        asset: String,
        action: Action,
        amount: f64,
        time: u32,
        req_id: Uuid,
        response_tx: oneshot::Sender<Result>,  // ← ADD THIS
    },
}
impl TradesHandle {
    pub async fn trade(&self, ...) -> PocketResult {
        let (tx, rx) = oneshot::channel();
        let id = Uuid::new_v4();
        self.sender.send(Command::OpenOrder {
            ...,
            req_id: id,
            response_tx: tx,  // ← Pass dedicated channel
        }).await?;

        rx.await.map_err(|_| PocketError::ChannelClosed)??
    }
}
// In TradesApiModule::run():
Command::OpenOrder { response_tx, ... } => {
    // ... send to WebSocket ...
    // Later when response arrives:
    let _ = response_tx.send(Ok(*deal));  // ← Direct response
}
```

---

### 2. **Socket Disconnection During Trade Placement (Lost Trades)**
- **Location:** `crates/core-pre/src/client.rs:507-530`, `crates/binary_options_tools/src/pocketoption/modules/trades.rs`
- **Severity:** CRITICAL
- **Risk:** If the WebSocket connection drops **after** a trade order is sent but **before** the response is received, the trade might execute on the server but the client loses track of it. The reconnection logic doesn't maintain a "pending trades" registry to retry or verify.

**Scenario:**
1. User calls `client.buy("EURUSD", 100, 60)` at 10:00:00.000
2. `OpenOrder` JSON sent to WebSocket at 10:00:00.050
3. Network hiccup → connection lost at 10:00:00.100
4. PocketOption server receives order at 10:00:00.120 → **Trade opens**
5. Client reconnects at 10:00:00.500
6. `successopenOrder` response is lost
7. Client thinks trade failed, but **real money is at risk**

**Current Disconnection Handler:**
```rust
// crates/core-pre/src/client.rs:507-530
_ = async {
    if let Some(reader_task) = &mut reader_task_opt {
        let _ = reader_task.await;
    }
} => {
    warn!("Connection lost unexpectedly.");
    // ❌ NO TRADE STATE RECONCILIATION
    self.router.middleware_stack.on_disconnect(&middleware_context).await;
    // Tasks aborted, no recovery
}
```

**Fix:**
```rust
// Add to State:
pub pending_orders: RwLock<HashMap>,
// In TradesApiModule::run():
Command::OpenOrder { asset, action, amount, time, req_id } => {
    let order = OpenOrder::new(...);

    // ✅ Track pending order BEFORE sending
    self.state.pending_orders.write().await.insert(
        req_id,
        (order.clone(), Instant::now())
    );

    self.to_ws_sender.send(Message::text(order.to_string())).await?;
}
// On response:
ServerResponse::Success(deal) => {
    self.state.pending_orders.write().await.remove(&deal.request_id.unwrap());
    // ... rest of logic
}
// Add reconnection callback:
struct TradeReconciliationCallback;
#[async_trait]
impl ReconnectCallback for TradeReconciliationCallback {
    async fn call(&self, state: Arc, ws_sender: &AsyncSender) -> CoreResult {
        let pending = state.pending_orders.read().await;

        for (req_id, (order, created_at)) in pending.iter() {
            // If order was sent >5 seconds ago, verify it
            if created_at.elapsed() > Duration::from_secs(5) {
                // Send check request or re-verify opened deals
                warn!("Verifying potentially lost trade: {}", req_id);
            }
        }

        // Clean up orders >2 minutes old (failed/timed out)
        state.pending_orders.write().await.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(120));

        Ok(())
    }
}
```

---

### 3. **Check Win Timeout Leaves Stale Waitlist Entries**
- **Location:** `crates/binary_options_tools/src/pocketoption/modules/deals.rs:82-120, 257-273`
- **Severity:** CRITICAL
- **Risk:** The `check_result_with_timeout()` function adds trade IDs to a waitlist but **never removes them on timeout**. This causes:
  1. **Memory leak**: Waitlist grows indefinitely
  2. **Stale responses**: When the deal eventually closes, the response goes to a caller who already timed out
  3. **Channel confusion**: The next caller for a different trade might receive the old response

**Vulnerable Code:**
```rust
pub async fn check_result_with_timeout(&self, trade_id: Uuid, timeout: Duration) -> PocketResult {
    self.sender.send(Command::CheckResult(trade_id)).await?;  // ← Adds to waitlist

    loop {
        tokio::select! {
            result = self.receiver.recv() => { /* ... */ }
            _ = &mut timeout_future => {
                return Err(PocketError::Timeout { ... });  // ❌ EXITS WITHOUT CLEANUP
            }
        }
    }
}
// In DealsApiModule::run():
Command::CheckResult(trade_id) => {
    if self.state.trade_state.contains_opened_deal(trade_id).await {
        self.waitlist.push(trade_id);  // ← NEVER REMOVED IF CALLER TIMES OUT
    }
}
```

**Impact Example:**
```python
# Python user code
trade_id = client.buy("EURUSD", 100, 60)[0]
# Check result with 5-second timeout (but trade takes 60s)
try:
    result = client.check_win(trade_id)  # Times out after 5s
except TimeoutError:
    print("Timeout, but trade_id still in waitlist!")
# 55 seconds later, trade closes...
# DealsApiModule sends response to channel
# But original caller is gone!
# Next caller gets this stale response:
other_trade_id = client.buy("BTCUSD", 50, 30)[0]
result = client.check_win(other_trade_id)  # ← Might get EURUSD result!
```

**Fix:**
```rust
pub enum Command {
    CheckResult(Uuid),
    CancelCheckResult(Uuid),  // ← ADD CANCELLATION
}
pub async fn check_result_with_timeout(&self, trade_id: Uuid, timeout: Duration) -> PocketResult {
    self.sender.send(Command::CheckResult(trade_id)).await?;

    let timeout_future = tokio::time::sleep(timeout);
    tokio::pin!(timeout_future);

    loop {
        tokio::select! {
            result = self.receiver.recv() => { /* ... */ }
            _ = &mut timeout_future => {
                // ✅ Clean up waitlist entry
                let _ = self.sender.send(Command::CancelCheckResult(trade_id)).await;
                return Err(PocketError::Timeout { ... });
            }
        }
    }
}
// In DealsApiModule::run():
Command::CancelCheckResult(trade_id) => {
    self.waitlist.retain(|id| *id != trade_id);
}
// Also add TTL cleanup:
async fn run(&mut self) -> CoreResult {
    let mut cleanup_interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        tokio::select! {
            _ = cleanup_interval.tick() => {
                // Remove waitlist entries for deals that closed >5 min ago
                let closed_ids: Vec = self.state.trade_state
                    .get_closed_deals().await.keys().cloned().collect();
                self.waitlist.retain(|id| !closed_ids.contains(id));
            }
            // ... existing match arms
        }
    }
}
```

---

### 4. **No Duplicate Trade Prevention (Double-Trading Risk)**
- **Location:** `crates/binary_options_tools/src/pocketoption/pocket_client.rs:191-219`
- **Severity:** CRITICAL
- **Risk:** If a user retries a trade (due to timeout or UI double-click), there's no idempotency mechanism to prevent duplicate orders. Both trades could execute, doubling the exposure.

**Scenario:**
```python
# User clicks "BUY $100" button
try:
    result = client.buy("EURUSD", 100, 60)
except NetworkTimeout:
    # User clicks again thinking it failed
    result = client.buy("EURUSD", 100, 60)  # ← SECOND ORDER PLACED
    # Now $200 is at risk instead of $100
```

**Current Code (No Protection):**
```rust
pub async fn trade(&self, asset: impl ToString, action: Action, time: u32, amount: f64) -> PocketResult {
    // ❌ Every call creates new UUID, no deduplication
    handle.trade(asset_str, action, amount, time).await.map(|d| (d.id, d))
}
```

**Fix:**
```rust
// Add deduplication cache to State:
pub recent_trades: RwLock<HashMap>,
pub async fn trade(&self, asset: impl ToString, action: Action, time: u32, amount: f64) -> PocketResult {
    let asset_str = asset.to_string();
    self.validate_asset(&asset_str, time).await?;

    // Create fingerprint (amount as integer cents to avoid f64 comparison)
    let amount_cents = (amount * 100.0).round() as u64;
    let fingerprint = (asset_str.clone(), action, time, amount_cents);

    // ✅ Check for recent duplicate (within 2 seconds)
    let mut recent = self.client.state.recent_trades.write().await;
    if let Some((existing_id, created_at)) = recent.get(&fingerprint) {
        if created_at.elapsed() < Duration::from_secs(2) {
            return Err(PocketError::DuplicateTrade {
                message: format!("Duplicate trade detected (ID: {})", existing_id),
                original_id: *existing_id,
            });
        }
    }

    let result = handle.trade(asset_str.clone(), action, amount, time).await?;

    // Store for deduplication
    recent.insert(fingerprint, (result.id, Instant::now()));

    // Cleanup old entries (>5 seconds)
    recent.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(5));

    Ok((result.id, result))
}
```

**Alternative (Idempotency Token):**
```python
# Python API:
idempotency_key = str(uuid.uuid4())
result = client.buy("EURUSD", 100, 60, idempotency_key=idempotency_key)
# Retry with same key:
result = client.buy("EURUSD", 100, 60, idempotency_key=idempotency_key)  # ← Returns cached result
```

---

## 🦀 RUST & BINDING BUGS

### 5. **Subscription Channel Race Condition**
- **Location:** `crates/binary_options_tools/src/pocketoption/modules/subscriptions.rs:185-199`
- **Severity:** MEDIUM
- **Issue:** Same pattern as trades module - shared `AsyncReceiver<CommandResponse>` for all subscriptions. Multiple concurrent `subscribe()` calls can receive each other's stream receivers.

**Impact:** Low (subscriptions are less time-critical than trades), but could cause stream data to go to wrong consumer.

**Fix:** Apply same oneshot channel pattern as trade fix #1.

---

### 6. **State Not Cleared on Reconnection**
- **Location:** `crates/binary_options_tools/src/pocketoption/state.rs:128-134`
- **Severity:** MEDIUM
- **Issue:** The `clear_temporal_data()` only clears balance but leaves `opened_deals`, `closed_deals`, `active_subscriptions` intact. On reconnection, these could be stale.

**Vulnerable Code:**
```rust
#[async_trait]
impl AppState for State {
    async fn clear_temporal_data(&self) {
        let mut balance = self.balance.write().await;
        *balance = None;
        // ❌ MISSING:
        // - opened_deals might reference closed trades
        // - active_subscriptions are dead
        // - raw_validators/raw_sinks are stale
    }
}
```

**Fix:**
```rust
async fn clear_temporal_data(&self) {
    *self.balance.write().await = None;

    // ✅ Clear stale trade state (but keep closed deals for history)
    self.trade_state.clear_opened_deals().await;

    // ✅ Mark subscriptions as requiring re-subscription
    // (SubscriptionCallback already handles this, but clean up channels)
    self.active_subscriptions.write().await.clear();

    // ✅ Clear raw validators (will be re-created if needed)
    self.clear_raw_validators();

    // ⚠️ Don't clear closed_deals - user might be checking recent results
    // Don't clear server_time - offset is still valid
}
```

---

### 7. **Python GIL Bottleneck in Async Methods**
- **Location:** `BinaryOptionsToolsV2/src/pocketoption.rs:234-270`, `BinaryOptionsToolsV2/src/runtime.rs:8`
- **Severity:** MEDIUM (Performance)
- **Issue:** All async Python methods use `future_into_py(py, async move { ... })` which requires holding the GIL during future creation. This can block other Python threads. The tokio runtime is shared globally, which is good, but there's no explicit GIL release for blocking operations.

**Current Code:**
```rust
pub fn buy(&self, py: Python, ...) -> PyResult<Bound> {
    let client = self.client.clone();
    future_into_py(py, async move {  // ← GIL held until future starts
        let res = client.buy(asset, time, amount).await?;  // ← Could take seconds
        // ...
    })
}
```

**Impact:**
- If 10 Python threads each call `buy()`, they serialize on GIL acquisition
- Low impact if users use `asyncio`, high impact if using threading

**Fix:**
```rust
// For blocking sync API, release GIL:
pub fn buy_sync(&self, py: Python, asset: String, amount: f64, time: u32) -> PyResult {
    let client = self.client.clone();
    let runtime = get_runtime(py)?;

    // ✅ Release GIL for blocking operation
    py.allow_threads(|| {
        runtime.block_on(async move {
            let res = client.buy(asset, time, amount).await?;
            let deal = serde_json::to_string(&res.1)?;
            Ok(vec![res.0.to_string(), deal])
        })
    })
}
// For async API, spawn on runtime to release GIL faster:
pub fn buy(&self, py: Python, ...) -> PyResult<Bound> {
    let client = self.client.clone();
    let runtime = get_runtime(py)?;

    // ✅ Spawn immediately and return awaitable
    let future = runtime.spawn(async move {
        client.buy(asset, time, amount).await
    });

    future_into_py(py, async move {
        let res = future.await.map_err(...)??;
        Python::attach(|py| res.into_py_any(py))
    })
}
```

**Note:** The current approach is acceptable for most use cases. Only optimize if profiling shows GIL contention.

---

### 8. **Waitlist Memory Leak**
- **Location:** `crates/binary_options_tools/src/pocketoption/modules/deals.rs:131, 262`
- **Severity:** MEDIUM
- **Issue:** The `waitlist: Vec<Uuid>` in `DealsApiModule` grows unbounded. If `check_result()` is called for a deal that never closes (server bug), the UUID stays forever.

**Fix:** Added in Critical Issue #3 above (TTL cleanup).

---

### 9. **Error Context Loss in Python Bindings**
- **Location:** `BinaryOptionsToolsV2/src/error.rs:27-30`
- **Severity:** LOW
- **Issue:** All Rust errors are converted to `PyValueError` with just `.to_string()`, losing type information and context.

**Current:**
```rust
impl From for PyErr {
    fn from(value: BinaryErrorPy) -> Self {
        PyValueError::new_err(value.to_string())  // ❌ Generic error
    }
}
```

**Python sees:**
```python
try:
    client.buy(...)
except ValueError as e:
    print(e)  # "BinaryOptionsError, General, Timeout"
    # ❌ Can't distinguish timeout from validation error
```

**Fix:**
```rust
// Create specific exception types:
use pyo3::create_exception;
create_exception!(BinaryOptionsToolsV2, TimeoutError, PyException);
create_exception!(BinaryOptionsToolsV2, TradeError, PyException);
create_exception!(BinaryOptionsToolsV2, ConnectionError, PyException);
impl From for PyErr {
    fn from(value: BinaryErrorPy) -> Self {
        match value {
            BinaryErrorPy::PocketOptionError(ref e) => {
                if matches!(e.as_ref(), PocketError::Timeout { .. }) {
                    TimeoutError::new_err(value.to_string())
                } else if matches!(e.as_ref(), PocketError::FailOpenOrder { .. }) {
                    TradeError::new_err(value.to_string())
                } else {
                    PyValueError::new_err(value.to_string())
                }
            }
            // ... map other variants
        }
    }
}
// In Python:
from BinaryOptionsToolsV2 import TimeoutError, TradeError
try:
    client.buy(...)
except TimeoutError:
    # Retry logic
except TradeError as e:
    # Log failed trade
```

---

## ⚡ PERFORMANCE & LATENCY

### 10. **Fixed Reconnection Delay (No Exponential Backoff)**
- **Location:** `crates/core-pre/src/client.rs:356-360`
- **Severity:** MEDIUM
- **Observation:** Fixed 5-second retry delay could hammer the server if it's experiencing issues. Industry best practice is exponential backoff with jitter.

**Current:**
```rust
Err(e) => {
    warn!("Connection failed: {e}. Retrying in 5s...");
    tokio::time::sleep(Duration::from_secs(5)).await;  // ❌ Always 5s
    continue;
}
```

**Fix:**
```rust
// Add to ClientRunner:
pub(crate) reconnect_attempts: u32,
// In run loop:
Err(e) => {
    self.reconnect_attempts += 1;
    let delay = std::cmp::min(
        5 * 2u64.pow(self.reconnect_attempts),  // Exponential: 5s, 10s, 20s, 40s...
        300  // Cap at 5 minutes
    );

    // Add jitter (±20%)
    let jitter = rand::thread_rng().gen_range(0.8..1.2);
    let delay = Duration::from_secs((delay as f64 * jitter) as u64);

    warn!("Connection failed (attempt {}): {e}. Retrying in {:?}...", self.reconnect_attempts, delay);
    tokio::time::sleep(delay).await;
    continue;
}
// Reset on success:
Ok(stream) => {
    self.reconnect_attempts = 0;  // ✅ Reset counter
    // ...
}
```

---

### 11. **Middleware Hooks Without Error Handling**
- **Location:** `crates/core-pre/src/client.rs:122-126, 415-418`
- **Severity:** LOW
- **Observation:** Middleware hooks (on_send, on_receive) are called but don't propagate errors. If middleware needs to reject a message or log critical info, it can't.

**Current:**
```rust
self.middleware_stack.on_receive(&message, &middleware_context).await;
// ❌ No error handling
```

**Recommendation:**
```rust
// Change middleware trait to return Result:
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn on_receive(&self, msg: &Message, ctx: &MiddlewareContext) -> CoreResult;
}
// In router:
if let Err(e) = self.middleware_stack.on_receive(&message, &middleware_context).await {
    error!("Middleware rejected message: {e}");
    return Err(e);  // Or continue, depending on policy
}
```

---

### 12. **Subscription Limit Not Enforced Correctly**
- **Location:** `crates/binary_options_tools/src/pocketoption/modules/subscriptions.rs:62`
- **Severity:** LOW
- **Observation:** `MAX_SUBSCRIPTIONS = 4` is defined but only enforced by checking `active_subscriptions.len()`. If a subscription fails to initialize but stays in the map, it counts toward the limit.

**Fix:**
```rust
// In subscribe():
let active_count = self.state.active_subscriptions.read().await.len();
if active_count >= MAX_SUBSCRIPTIONS {
    return Err(SubscriptionError::MaxSubscriptionsReached.into());
}
// ... create subscription ...
// ✅ Only add to map if successful:
if subscription_created_ok {
    self.state.active_subscriptions.write().await.insert(...);
}
```

---

## 🛡️ SECURITY

### 13. **SSID Exposure in Debug Logs**
- **Location:** `crates/binary_options_tools/src/pocketoption/ssid.rs:10-14, 72-75`
- **Severity:** HIGH
- **Risk:** The `Ssid` struct derives `Debug` and contains sensitive session tokens. If logging is set to `TRACE` or `DEBUG` level, SSIDs (including session IDs and IP addresses) could be written to logs.

**Vulnerable Code:**
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]  // ← Debug prints raw data
pub struct SessionData {
    session_id: String,       // ← Sensitive!
    ip_address: String,       // ← PII
    user_agent: String,
    last_activity: u64,
}
#[derive(Debug, Serialize, Clone)]  // ← Debug prints raw SSID
pub enum Ssid {
    Demo(Demo),
    Real(Real),  // Contains SessionData
}
```

**Risk Example:**
```rust
tracing::debug!("State: {:?}", client.state);  // ← Logs entire SSID
```

**Fix:**
```rust
// Remove Debug, implement custom Display that redacts:
use std::fmt;
impl fmt::Debug for SessionData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SessionData")
            .field("session_id", &"[REDACTED]")
            .field("ip_address", &self.ip_address.chars().take(3).collect::() + ".***.***")
            .field("user_agent", &self.user_agent.chars().take(20).collect::() + "...")
            .field("last_activity", &self.last_activity)
            .finish()
    }
}
impl fmt::Debug for Ssid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ssid::Demo(demo) => write!(f, "Ssid::Demo(uid: {}, is_demo: {})", demo.uid, demo.is_demo),
            Ssid::Real(real) => write!(f, "Ssid::Real(uid: {}, is_demo: {})", real.uid, real.is_demo),
        }
    }
}
```

**Additional Mitigation:**
```rust
// Add to documentation:
/// ⚠️ SECURITY: Never log SSID in production. Use Display trait, not Debug.
/// The session token grants full access to the user's account.
pub enum Ssid { ... }
```

---

### 14. **TLS Configuration**
- **Location:** `crates/binary_options_tools/src/pocketoption/utils.rs:70-108`
- **Severity:** MEDIUM (Informational)
- **Status:** ✅ **SECURE**
- **Observation:** TLS is properly configured with rustls and native certificate verification. The `danger_accept_invalid_certs` parameter is `false`, which is correct. However, there's no certificate pinning.

**Current (Good):**
```rust
let tls_config = rustls::ClientConfig::builder()
    .with_root_certificates(root_store)  // ✅ Uses native cert store
    .with_no_client_auth();
connect_async_tls_with_config(request, None, false, Some(connector))  // ✅ false = reject invalid certs
```

**Enhancement (Optional):**
For maximum security against MITM attacks, pin PocketOption's certificate:
```rust
use rustls::client::ServerCertVerifier;
struct PocketOptionCertVerifier {
    // SHA-256 fingerprint of PocketOption's cert
    expected_fingerprint: [u8; 32],
}
impl ServerCertVerifier for PocketOptionCertVerifier {
    fn verify_server_cert(&self, cert: &Certificate, ...) -> Result {
        let fingerprint = sha256(cert.0);
        if fingerprint != self.expected_fingerprint {
            return Err(Error::InvalidCertificate(...));
        }
        // ... also verify with normal chain validation
    }
}
```

**Note:** Only implement if PocketOption provides a stable certificate. May break on cert rotation.

---

### 15. **No Input Validation on Trade Amounts**
- **Location:** `crates/binary_options_tools/src/pocketoption/pocket_client.rs:201-210`
- **Severity:** MEDIUM
- **Issue:** While min/max amount is validated, there's no check for:
  - Negative amounts (though u64 cast would fail)
  - NaN or Infinity
  - Exceeding account balance

**Current:**
```rust
if amount < MINIMUM_TRADE_AMOUNT {
    return Err(...);
}
if amount > MAXIMUM_TRADE_AMOUNT {
    return Err(...);
}
// ❌ Missing: NaN, balance check
```

**Fix:**
```rust
// Validate amount:
if !amount.is_finite() {
    return Err(PocketError::InvalidAmount("Amount must be a finite number".into()));
}
if amount <= 0.0 {
    return Err(PocketError::InvalidAmount("Amount must be positive".into()));
}
if amount < MINIMUM_TRADE_AMOUNT {
    return Err(PocketError::InvalidAmount(format!("Minimum trade amount is {}", MINIMUM_TRADE_AMOUNT)));
}
if amount > MAXIMUM_TRADE_AMOUNT {
    return Err(PocketError::InvalidAmount(format!("Maximum trade amount is {}", MAXIMUM_TRADE_AMOUNT)));
}
// Optional: Check balance
let balance = self.balance().await;
if balance > 0.0 && amount > balance {
    return Err(PocketError::InsufficientBalance {
        required: amount,
        available: balance,
    });
}
```

---

## 🛠️ ARCHITECTURAL DEBT

### 16. **Floating-Point for Money**
- **Severity:** MEDIUM (Correctness)
- **Observation:** All financial values (balance, trade amounts, prices) use `f64`. This can lead to rounding errors in calculations.

**Example:**
```rust
let balance: f64 = 100.10;
let amount: f64 = 100.10;
if balance >= amount {  // ← Might fail due to precision!
    trade(amount);
}
```

**Recommendation:**
```rust
// Use rust_decimal for money:
use rust_decimal::Decimal;
pub struct Deal {
    pub amount: Decimal,  // Instead of f64
    pub profit: Decimal,
    // ...
}
// Serialize/deserialize with serde:
#[derive(Serialize, Deserialize)]
#[serde(with = "rust_decimal::serde::float")]
pub amount: Decimal,
```

**Note:** This is a large refactor. Acceptable to keep f64 if:
1. All calculations are done server-side
2. Client only displays values
3. Comparison tolerance is used (`(a - b).abs() < 0.01`)

---

### 17. **Rule State Not Reset on Reconnection**
- **Location:** Multiple Rule implementations (e.g., `DealsUpdateRule`, `TwoStepRule`)
- **Severity:** LOW
- **Observation:** Rules like `DealsUpdateRule` use `AtomicBool` to track state across messages (Text → Binary pairing). If a reconnection happens mid-message, the state isn't reset.

**Example:**
```rust
impl Rule for DealsUpdateRule {
    fn call(&self, msg: &Message) -> bool {
        match msg {
            Message::Text(text) if text.starts_with(UPDATE_CLOSED_DEALS) => {
                self.valid.store(true, Ordering::SeqCst);  // ← Set flag
                true
            }
            Message::Binary(_) if self.valid.load(Ordering::SeqCst) => {
                self.valid.store(false, Ordering::SeqCst);  // ← Clear flag
                true
            }
            _ => false,
        }
    }

    fn reset(&self) {
        self.valid.store(false, Ordering::SeqCst);  // ← Called on reconnect?
    }
}
```

**Fix:** Ensure `Rule::reset()` is called in the reconnection callback:
```rust
// In connector or reconnection logic:
struct RuleResetCallback;
#[async_trait]
impl ReconnectCallback for RuleResetCallback {
    async fn call(&self, state: Arc, _: &AsyncSender) -> CoreResult {
        // Reset all module rules
        // (Would need to store Rule instances in State or Router)
        Ok(())
    }
}
```

**Current Status:** Not critical because Text/Binary pairing is usually within milliseconds, and reconnections are rare.

---

### 18. **No Metrics/Observability**
- **Severity:** LOW (Operational)
- **Observation:** The codebase uses `tracing` for logging but doesn't expose metrics like:
  - Trade success/failure rate
  - Average response time
  - Connection uptime
  - Number of reconnections

**Recommendation:**
```rust
// Add to State:
pub metrics: Arc,
pub struct Metrics {
    pub trades_opened: AtomicU64,
    pub trades_failed: AtomicU64,
    pub reconnections: AtomicU64,
    pub avg_response_time_ms: AtomicU64,
}
// Expose in Python:
#[pyclass]
pub struct RawPocketOption {
    // ...
    pub fn get_metrics(&self) -> PyResult {
        let metrics = self.client.state.metrics.clone();
        Ok(serde_json::to_string(&Metrics {
            trades_opened: metrics.trades_opened.load(Ordering::Relaxed),
            // ...
        })?)
    }
}
```

---

## Summary Table

| ID | Issue | Severity | Category | Impact |
|----|-------|----------|----------|--------|
| 1 | Race condition in concurrent trades | CRITICAL | Trading | Response misrouting, deadlock |
| 2 | Lost trades on disconnection | CRITICAL | Trading | Financial loss |
| 3 | Check win timeout memory leak | CRITICAL | Trading | Memory leak, stale responses |
| 4 | No duplicate trade prevention | CRITICAL | Trading | Double-trading |
| 5 | Subscription channel race | MEDIUM | Async | Stream misrouting |
| 6 | State not cleared on reconnect | MEDIUM | Async | Stale data |
| 7 | Python GIL bottleneck | MEDIUM | Performance | Thread contention |
| 8 | Waitlist memory leak | MEDIUM | Memory | Unbounded growth |
| 9 | Error context loss in Python | LOW | DX | Poor debugging |
| 10 | No exponential backoff | MEDIUM | Performance | Server hammering |
| 11 | Middleware error handling | LOW | Arch | Silent failures |
| 12 | Subscription limit enforcement | LOW | Correctness | Stale quota |
| 13 | SSID exposure in logs | HIGH | Security | Credential leak |
| 14 | TLS configuration | MEDIUM | Security | ✅ Secure (no issue) |
| 15 | No balance validation | MEDIUM | Validation | Overdraft attempts |
| 16 | Floating-point for money | MEDIUM | Correctness | Rounding errors |
| 17 | Rule state not reset | LOW | Correctness | Stale routing |
| 18 | No metrics | LOW | Ops | Poor observability |
---

## Recommended Action Plan

### Immediate (Critical - Deploy within 1 week):
1. **Fix #1:** Implement per-request oneshot channels for trades
2. **Fix #2:** Add pending order tracking with reconciliation callback
3. **Fix #3:** Implement waitlist cleanup on timeout + TTL
4. **Fix #4:** Add duplicate trade detection (fingerprinting or idempotency)

### Short-term (High - Deploy within 1 month):
5. **Fix #13:** Remove Debug trait from Ssid, implement redacted logging
6. **Fix #15:** Add NaN/balance validation for trade amounts
7. **Fix #6:** Properly clear state on reconnection

### Medium-term (Medium - Next quarter):
8. **Fix #10:** Exponential backoff for reconnections
9. **Fix #5:** Apply oneshot pattern to subscriptions
10. **Fix #7:** Optimize GIL handling if profiling shows contention
11. **Fix #16:** Consider migrating to `rust_decimal` for money

### Long-term (Low - Backlog):
12. **Fix #9:** Create specific exception types for Python
13. **Fix #11:** Make middleware errors propagate
14. **Fix #18:** Add metrics collection
15. **Fix #17:** Implement rule reset on reconnection

---

## Testing Recommendations

### Critical Path Tests:
```rust
#[tokio::test]
async fn test_concurrent_trades_no_race() {
    let client = PocketOption::new("demo_ssid").await.unwrap();

    // Spawn 10 concurrent trades
    let handles: Vec = (0..10).map(|i| {
        let client = client.clone();
        tokio::spawn(async move {
            client.buy(format!("EURUSD{}", i), 100.0, 60).await
        })
    }).collect();

    let results = futures::future::join_all(handles).await;

    // All should succeed with unique IDs
    let ids: HashSet = results.iter().map(|r| r.as_ref().unwrap().0).collect();
    assert_eq!(ids.len(), 10, "All trades should have unique IDs");
}
#[tokio::test]
async fn test_trade_during_disconnection() {
    let client = PocketOption::new("demo_ssid").await.unwrap();

    // Start a trade
    let trade_fut = client.buy("EURUSD", 100.0, 60);

    // Simulate disconnection mid-flight
    tokio::time::sleep(Duration::from_millis(50)).await;
    client.disconnect().await.unwrap();

    // Trade should either:
    // 1. Complete successfully (was sent before disconnect)
    // 2. Fail with ConnectionLost error (not silently lost)
    let result = trade_fut.await;
    assert!(result.is_ok() || matches!(result, Err(PocketError::ConnectionLost)));
}
#[tokio::test]
async fn test_check_win_timeout_cleanup() {
    let client = PocketOption::new("demo_ssid").await.unwrap();
    let (trade_id, _) = client.buy("EURUSD", 100.0, 60).await.unwrap();

    // Check with short timeout
    let result = client.result_with_timeout(trade_id, Duration::from_secs(1)).await;
    assert!(matches!(result, Err(PocketError::Timeout { .. })));

    // Verify waitlist was cleaned up (no public API for this - need internal test)
    // Check that a second timeout doesn't receive stale response
}
```

---

## Conclusion

The BinaryOptionsTools-v2 codebase demonstrates strong architectural foundations with proper async patterns, middleware support, and modular design. However, **4 critical issues** in trade execution and disconnection handling could lead to financial loss or inconsistent state. The recommended fixes are straightforward (mostly channel refactoring and state management) and should be prioritized immediately.

**Security posture is generally good** (proper TLS, no SQL injection vectors), but SSID logging exposure is a high-risk issue that could compromise user accounts if logs are leaked.

**Performance is adequate** for most use cases, with the main bottleneck being the fixed reconnection delay and potential GIL contention in high-frequency Python scenarios.

**Overall Grade: B** (would be A- after fixing critical issues)

---

**Report compiled by:** Systems Engineering & Quantitative Trading Audit
**Methodology:** Static code analysis + manual trace of WebSocket lifecycle + concurrency pattern review
**Scope:** Full Rust crates + PyO3 bindings (UniFFI bindings not audited)
