# Breaking Changes in Version 0.2.6

This document outlines the breaking changes introduced in version 0.2.6 of BinaryOptionsTools V2. These changes were necessary to improve performance, reliability, and architectural consistency.

## 1. Virtual Market Profit Semantics

### Change

The `Deal.profit` field in `VirtualMarket` now stores the **net gain or loss** instead of the total payout.

### Impact

* **Win**: `profit = stake * payout_percentage` (e.g., $1.00 stake at 80% returns $0.80 profit).
* **Loss**: `profit = -stake` (e.g., $1.00 stake returns -$1.00 profit).
* **Draw**: `profit = 0.00`.

### Why?

This aligns with standard trading API semantics and makes it easier to calculate overall PnL (Profit and Loss) by simply summing the `profit` fields.

---

## 2. WebSocket Event System Unification

### Change

The redundant `WebSocketEventHandler` trait has been removed in favor of the standard `EventHandler` trait. Additionally, `WebSocketEvent` variants have been converted from struct-style to tuple/unit-style.

### Impact

If you have implemented custom event handlers, you must update the trait signature and the match arms for events.

**Old Pattern (Struct-style):**

```rust
match event {
    WebSocketEvent::Connected { region } => { ... }
    WebSocketEvent::Disconnected { reason } => { ... }
}
```

**New Pattern (Tuple/Unit-style):**

```rust
match event {
    WebSocketEvent::Connected => { ... }
    WebSocketEvent::Disconnected(reason) => { ... }
}
```

---

## 3. Response Router Pre-registration

### Change

The `ResponseRouter` now requires explicit registration of a request ID *before* the command is sent to the module.

### Impact

This is primarily an internal change for developers extending the library. However, it ensures that high-speed responses are never "missed" by the router because the listener wasn't ready yet.

---

## 4. Error Variant Boxing

### Change

The `BinaryOptionsToolsError::WebsocketConnectionError` variant now contains a `Box<tokio_tungstenite::tungstenite::Error>` instead of a bare error.

### Impact

Code that matches on this specific error variant will need to handle the box:

```rust
// Old
Err(BinaryOptionsToolsError::WebsocketConnectionError(e)) => { ... }

// New
Err(BinaryOptionsToolsError::WebsocketConnectionError(boxed_e)) => {
    let e = *boxed_e;
    ...
}
```

---

## 5. Python Synchronous Client Lifecycle

### Change

Exiting the `PocketOption` context manager (`with` block) now explicitly closes the internal event loop.

### Impact

You cannot reuse a `PocketOption` instance after its `with` block has ended. A new instance must be created if further operations are needed. This change was necessary to prevent background resource leaks.

---

## 6. Type Hint Corrections (.pyi)

### Change

The `BinaryOptionsToolsV2.pyi` file has been corrected to show that most trading and data methods return **JSON strings** (or lists of strings) rather than Python dictionaries.

### Impact

Type checkers (like Mypy or Pyright) will now correctly flag code that assumes these methods return parsed dictionaries. You must use `json.loads()` on the return value if you are using the `RawPocketOption` class directly. (Note: `PocketOptionAsync` and `PocketOption` high-level wrappers still return parsed objects for convenience).
