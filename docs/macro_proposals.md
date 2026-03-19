# Macro Proposals for Faster Platform Onboarding

## Goals
- Let contributors add a new trading platform by focusing only on WebSocket I/O semantics (what to send, what to parse), not on runner plumbing.
- Standardize docs (especially multi-language bindings) via macros to avoid drift.
- Reduce boilerplate for modules, handles, rules, and platform scaffolding across Rust, PyO3, and UniFFI layers.

## Existing/Done
- `#[uniffi_doc(path = "...")]`: injects feature-gated docs from JSON into UniFFI-exposed items.

## High-Value New Macros

### 1) `#[lightweight_module(...)]` (proc-macro attribute)
**Purpose:** Generate the struct fields, `fn new`, `fn rule`, and the run-loop scaffolding for `LightweightModule` implementations.

**Inputs (attribute):**
- `name = "BalanceModule"` (optional; default = type name)
- `rule_pattern = "451-[\"updateAssets\","` **or** `rule_fn = rule_fn_name` (function returning `Box<dyn Rule>`)
- Optional flags: `needs_state`, `needs_sender`, `needs_runner_cmd` (inferred by param list if possible)

**User-provided function signature:**
- `async fn handle(msg: &Message, state: Option<&Arc<State>>, sender: Option<&AsyncSender<Message>>, runner_cmd: Option<&AsyncSender<RunnerCommand>>) -> CoreResult<()>`
  - Macro inspects which params are present to decide which fields to store.

**Generated:**
- Struct fields for only what is needed (`state`, `receiver`, optional `sender`, optional `runner_cmd`).
- `fn new(...)` wiring standard channels.
- `fn rule()` using `rule_pattern` or `rule_fn`.
- `async fn run()` with the `while let Ok(msg) = receiver.recv().await` loop and standardized error return.
- Doc forwarding: copies user `///` from `handle` onto the generated struct, plus a small example showing `.with_lightweight_module::<Name>()`.

### 2) `#[api_module(...)]` (proc-macro attribute/derive)
**Purpose:** Remove boilerplate for `ApiModule` implementors (new platform modules).

**Inputs (attribute):**
- `state = StateType`
- `rule_pattern = "successopenOrder"` **or** `rule_fn = rule_fn_name`
- Optional: `handle_struct = CustomHandle` (else auto-gen)

**Generated:**
- Struct fields for command/message channels.
- `fn new(...)` wiring all required channels.
- `fn create_handle(...)` wiring sender/receiver.
- Optional auto-generated `Handle` with basic constructor if not supplied.
- Doc forwarding from the annotated item, plus example showing `.with_module::<Name>()` or `.with_lightweight_module::<Name>()` as appropriate.
- Leaves the `run()` body to the author (too domain-specific) but enforces the signature and channel types.

### 3) `#[action_rule(name = "...")]` (proc-macro derive or attribute)
**Purpose:** Replace the repeated `MultiPatternRule::new(vec![...])` / `TwoStepRule::new(...)` declarations.

**Inputs:**
- `pattern = "451-[\"foo\","` **or** `patterns = ["foo", "bar"]`

**Generated:**
- A `Rule` implementation and a `rule()` free function returning the boxed rule for reuse in modules and validators.

### 4) `#[platform_client(...)]` (proc-macro attribute)
**Purpose:** Scaffold a new platform end-to-end (core + bindings) from a single annotated core client.

**Inputs:**
- `ws_url_fn = path::to::make_url` (function to derive URL/headers)
- `platform_name = "pocketoption"` (kebab/slug)
- `modules = [AssetsModule, BalanceModule, ...]`
- Optional: `bindings = { pyo3 = true, uniffi = true }`

**Generated (Rust core side):**
- The platform `Client` type alias/struct with `.builder()` wiring listed modules.
- Region glue using `RegionImpl` if supplied.
- Standard reconnection and runner wiring.

**Generated (bindings side, when enabled):**
- For PyO3: thin wrappers for each public async method, using a shared `#[pyo3_async_json]`-style helper (see Macro 5).
- For UniFFI: object + methods stubs with consistent error mapping (see Macro 6).
- Docs auto-wired via `#[uniffi_doc]` or a PyO3 doc helper if provided.

### 5) `#[pyo3_async_json]` (proc-macro attribute)
**Purpose:** Collapse the repeated `future_into_py` + JSON serialization wrappers in `BinaryOptionsToolsV2`.

**Behavior:**
- Applied to `impl` methods on PyO3 classes.
- Infers: clone `self.client`, await an async call, map errors via `BinaryErrorPy::from`, JSON-serialize return value, and `Python::attach` it.
- Supports void-return variants and simple scalar returns.

### 6) `uni_err!` (declarative macro or attribute)
**Purpose:** Remove `.map_err(|e| UniError::from(BinaryOptionsError::from(e)))` chains in UniFFI layer.

**Behavior:**
- Wraps an expression and applies the double conversion.
- Optional form: `#[uni_try]` attribute on methods to wrap all `?` sites.

### 7) `#[ws_message]` / `#[ws_matcher]` (proc-macro attribute)
**Purpose:** Let developers define websocket message shapes declaratively.

**Behavior:**
- Attribute on enums/structs describing inbound/outbound messages with patterns or prefixes.
- Generates serde derive + Display/ToString for outbound + pattern matching helpers for inbound.
- Can emit `Rule` implementors (replacing manual `TwoStepRule`/`MultiPatternRule` wiring).
- Optionally integrates with `ResponseRouter`-like logic by auto-generating a `matches(&Message)` fn.

### 8) `#[validator_factory]` (proc-macro)
**Purpose:** Make it easy to build and compose validators (prefix/contains/regex) for raw handlers.

**Behavior:**
- Declarative list of predicates -> emits a `Validator` constructor + docstring.
- Example config: `#[validator_factory(name = BalanceValidator, contains = "\"balance\"", starts_with = "42[\"success"]`.

### 9) `#[connect_strategy]` (proc-macro attribute)
**Purpose:** Abstract WebSocket connection setup per platform (headers, query params, auth).

**Behavior:**
- Attribute on a function that returns connection params; macro generates a typed `Connect` struct implementing the expected trait for `ClientBuilder`.
- Reduces per-platform connection boilerplate.

### 10) `#[module_doc_example(...)]` (proc-macro helper)
**Purpose:** Standardize docs for modules/clients by generating a minimal, correct usage snippet.

**Behavior:**
- Takes `builder_call = "with_lightweight_module::<AssetsModule>()"` or similar.
- Emits a `///` block with a templated `no_run` example that stays in sync with the type name.

## How These Reduce Platform Onboarding
- A contributor describes:
  - Connection strategy (`#[connect_strategy]`).
  - Inbound/outbound messages (`#[ws_message]`).
  - Validators or rules (`#[validator_factory]`/`#[action_rule]`).
  - Modules as simple handlers (`#[lightweight_module]`).
  - API modules with minimal signatures (`#[api_module]`).
  - Exposes the client via `#[platform_client]` which then auto-wires PyO3/UniFFI surfaces and docs.
- They never touch runner wiring, channel setup, or repetitive doc/comment plumbing.

## Suggested Order of Implementation
1) `#[pyo3_async_json]` (fast win, removes most boilerplate today).
2) `#[lightweight_module]` (covers many modules and sets the pattern).
3) `#[api_module]` + `#[action_rule]` (reduces core module boilerplate).
4) `uni_err!` (small but high-frequency cleanup).
5) `#[ws_message]` + `#[validator_factory]` (developer ergonomics for new platforms).
6) `#[connect_strategy]` + `#[platform_client]` (full-platform scaffolding).
7) `#[module_doc_example]` (keeps docs in sync).
8) Extend `#[uniffi_doc]` as needed for new bindings.

## Mini Examples (macro surface vs. expanded code)

- `#[uniffi_doc]` (minimal vs expanded)
```rs
#[uniffi_doc(name = "Test", path = "BinaryOptionsToolsUni/docs_json/test.json")]
pub struct Test {
    // ...
}
```
Expanded:
```rs
#[doc = "Example of a JSON file for testing purposes.\n"]
#[cfg_attr(feature = "python", doc = "This file can be used to test JSON parsing in Python.")]
#[cfg_attr(feature = "javascript", doc = "It can also be used to test JSON parsing in JavaScript.")]
pub struct Test {
    // ...
}
```

- `#[lightweight_module]` (minimal vs expanded)
```rs
#[lightweight_module(name = "ServerTimeModule", rule_pattern = "451-[\"updateStream\",")]
async fn handle(msg: &Message, state: &Arc<State>) -> CoreResult<()> {
    if let Ok(candle) = serde_json::from_slice::<StreamData>(msg.as_bytes()) {
        state.update_server_time(candle.timestamp).await;
    }
    Ok(())
}
```
Expanded today:
```rs
pub struct ServerTimeModule {
    receiver: AsyncReceiver<Arc<Message>>,
    state: Arc<State>,
}

#[async_trait::async_trait]
impl LightweightModule<State> for ServerTimeModule {
    fn new(state: Arc<State>, _: AsyncSender<Message>, receiver: AsyncReceiver<Arc<Message>>, _: AsyncSender<RunnerCommand>) -> Self {
        Self { state, receiver }
    }

    fn rule() -> Box<dyn Rule + Send + Sync> {
        Box::new(TwoStepRule::new("451-[\"updateStream\","))
    }

    async fn run(&mut self) -> CoreResult<()> {
        while let Ok(msg) = self.receiver.recv().await {
            if let Ok(candle) = serde_json::from_slice::<StreamData>(msg.as_ref()) {
                self.state.update_server_time(candle.timestamp).await;
            }
        }
        Err(CoreError::LightweightModuleLoop("ServerTimeModule".into()))
    }
}
```

- `#[api_module]` (minimal vs expanded)
```rs
#[api_module(state = State, rule_pattern = "successopenOrder")]
pub struct TradesApiModule {
    // user fields only (e.g., trackers)
}

impl TradesApiModule {
    async fn run(&mut self) -> CoreResult<()> {
        // select! loop stays hand-written
        Ok(())
    }
}
```
Expanded today:
```rs
impl ApiModule<State> for TradesApiModule {
    type Command = Command;
    type CommandResponse = CommandResponse;
    type Handle = TradesHandle;

    fn new(state: Arc<State>, cmd_rx: AsyncReceiver<Self::Command>, cmd_tx: AsyncSender<Self::CommandResponse>, ws_rx: AsyncReceiver<Arc<Message>>, ws_tx: AsyncSender<Message>, _: AsyncSender<RunnerCommand>) -> Self {
        Self { /* user fields */, state, cmd_rx, cmd_tx, ws_rx, ws_tx }
    }

    fn create_handle(sender: AsyncSender<Self::Command>, receiver: AsyncReceiver<Self::CommandResponse>) -> Self::Handle {
        TradesHandle { sender, receiver }
    }

    async fn run(&mut self) -> CoreResult<()> {
        // user’s select! body here
        Ok(())
    }

    fn rule(_: Arc<State>) -> Box<dyn Rule + Send + Sync> {
        Box::new(MultiPatternRule::new(vec!["successopenOrder"]))
    }
}
```

- `#[ws_message]` (inbound + outbound, minimal vs expanded)
```rs
#[ws_message(pattern = "451-[\"successopenOrder\",")]
#[derive(Deserialize)]
pub struct OpenOrderSuccess {
    pub request_id: Uuid,
    pub deal: Deal,
}

#[ws_message(outbound)]
pub struct OpenOrderRequest {
    pub asset: String,
    pub amount: Decimal,
}
```
Expanded today:
```rs
#[derive(Deserialize)]
pub struct OpenOrderSuccess {
    pub request_id: Uuid,
    pub deal: Deal,
}
impl OpenOrderSuccess {
    pub fn matches(msg: &Message) -> bool {
        TwoStepRule::new("451-[\"successopenOrder\",").call(msg)
    }
}

pub struct OpenOrderRequest { pub asset: String, pub amount: Decimal }
impl std::fmt::Display for OpenOrderRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "42[\"openOrder\",{{\"asset\":\"{}\",\"amount\":{}}}]", self.asset, self.amount)
    }
}
```

- `#[platform_client]` (minimal vs expanded)
```rs
#[platform_client(
    platform_name = "pocketoption",
    ws_url_fn = pocket_connect,
    modules = [AssetsModule, BalanceModule, TradesApiModule],
    bindings = { pyo3 = true, uniffi = true }
)]
pub struct PocketOptionClient;
```
Expanded today:
```rs
pub struct PocketOptionClient {
    client: Client<State>,
    _runner: Arc<JoinHandle<()>>,
}

impl PocketOptionClient {
    pub async fn new(ssid: String) -> PocketResult<Self> {
        let state = StateBuilder::default().ssid(Ssid::parse(ssid)?).build()?;
        let (client, mut runner) = ClientBuilder::new(PocketConnect, state)
            .with_lightweight_module::<AssetsModule>()
            .with_lightweight_module::<BalanceModule>()
            .with_module::<TradesApiModule>()
            .build()
            .await?;
        let _runner = tokio::spawn(async move { runner.run().await });
        Ok(Self { client, _runner })
    }
}

// PyO3 wrapper (auto-generated)
#[pymethods]
impl PyPocketOptionClient {
    #[new]
    fn new_py(ssid: String, py: Python<'_>) -> PyResult<Self> {
        let inner = tokio::runtime::Runtime::new().unwrap().block_on(PocketOptionClient::new(ssid))
            .map_err(BinaryErrorPy::from)?;
        Ok(Self { inner })
    }
}

// UniFFI wrapper (auto-generated)
#[derive(uniffi::Object)]
pub struct UniPocketOptionClient { inner: PocketOptionClient }
#[uniffi::export]
impl UniPocketOptionClient {
    #[uniffi::constructor]
    pub async fn new(ssid: String) -> Result<Arc<Self>, UniError> {
        Ok(Arc::new(Self { inner: PocketOptionClient::new(ssid).await.map_err(UniError::from)? }))
    }
}
```

- `#[pyo3_async_json]` (minimal vs expanded)
```rs
#[pymethods]
impl RawPocketOption {
    #[pyo3_async_json]
    pub fn get_candles(&self, asset: String, period: i64, offset: i64) -> PyResult<PyObject> {
        // body filled in by macro
    }
}
```
Expanded today:
```rs
#[pymethods]
impl RawPocketOption {
    pub fn get_candles(&self, py: Python<'_>, asset: String, period: i64, offset: i64) -> PyResult<PyObject> {
        let client = self.client.clone();
        future_into_py(py, async move {
            let res = client.get_candles(asset, period, offset).await.map_err(BinaryErrorPy::from)?;
            Python::with_gil(|py| serde_json::to_string(&res).map_err(BinaryErrorPy::from)?.into_py(py))
        })
    }
}
```

- `uni_err!` (minimal vs expanded)
```rs
let deal = uni_err!(self.inner.result(uuid).await)?;
```
Expanded today:
```rs
let deal = self.inner.result(uuid).await.map_err(|e| UniError::from(BinaryOptionsError::from(e)))?;
```

- `#[module_doc_example]` (minimal vs expanded doc)
```rs
#[module_doc_example(builder_call = "with_lightweight_module::<AssetsModule>()")]
pub struct AssetsModule;
```
Generated doc snippet:
```rs
/// # Example
/// ```ignore
/// let (client, runner) = ClientBuilder::new(PocketConnect, state)
///     .with_lightweight_module::<AssetsModule>()
///     .build()
///     .await?;
/// ```
pub struct AssetsModule;
```

## Notes / Constraints
- Keep all macros in `crates/macros` unless a new crate is justified; maintain separation of platform-agnostic infra vs. platform-specific helpers.
- Ensure generated code respects existing traits in `core-pre`.
- Generated docs should use `no_run` or `ignore` to avoid doctest failures.
- Avoid adding new runtime dependencies; use `darling`, `syn`, `quote` already present.