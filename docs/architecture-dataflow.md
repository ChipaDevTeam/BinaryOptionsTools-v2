# System Architecture: Data Flow and Components

This document shows how data moves through the system: Client, Runner, Router, Middleware, ApiModules, LightweightModules, Lightweight Handlers, and Handles.

- Keep it simple: a few diagrams cover the full picture.
- Applies to all modules (Subscriptions, Trades, Raw, etc.).

## Legend

- WS: WebSocket connection managed by the Runner via the Connector
- Router: multiplexes messages to modules and handlers using rules
- Middleware: pre-/post-processing for inbound/outbound WS messages
- ApiModule: full-featured module with commands, responses, and a Handle
- LightweightModule: background task, receives routed WS messages, no command/response
- Lightweight Handler: global stateless callback receiving every WS message

## End-to-end Overview

```mermaid
flowchart LR
    subgraph Platform
      subgraph App[Client + Runner]
        direction TB
        Conn[Connector]
        WS[WebSocket]
        Runner[ClientRunner]
        Router
        Middleware[Middleware Stack]
      end

      subgraph Modules
        direction TB
        LWH[Lightweight Handlers]
        LWM[LightweightModules]
        AM[ApiModules]
        Handles[Module Handles]
      end
    end

    WS <--> Conn <--> Runner
    Runner <--> Router
    Router <--> Middleware

    %% Dispatch inbound
    Router -- rules --> LWM
    Router -- rules --> AM
    Router -- all msgs --> LWH

    %% Handles registration
    AM --- Handles

    %% Outbound path
    Handles ----> Runner
    LWM ----> Runner

    %% Through middleware for outbound
    Runner -.-> Middleware
    Runner --> Conn --> WS
```

- Inbound: WS -> Connector -> Runner -> Middleware (inbound) -> Router -> {LWH, LWM, AM} via rules.
- Outbound: {ApiModule via Handle, LightweightModule} -> Runner -> Middleware (outbound) -> Connector -> WS.

## ApiModule internals: commands, responses, and routing

```mermaid
flowchart LR
    subgraph Client
      Handle[Module Handle]
      Router
      subgraph Module[ApiModule<M>]
        direction TB
        RunLoop[run()]
        CmdRx[(CommandReceiver)]
        CmdTx[(CommandResponder)]
        MsgRx[(WS Msg Receiver)]
      end
    end

    %% User -> Module
    UserCode -->|send Command| Handle --> CmdRx
    RunLoop --> CmdTx -->|CommandResponse| Handle --> UserCode

    %% Routing of WS messages into module
    Router -- rule(M::rule)|--> MsgRx --> RunLoop
```

- The builder registers an M::Handle in a shared map. Client.get_handle::<M>() returns it.
- The module runs its own loop, reading commands and WS messages, emitting responses.

## LightweightModule internals: simple routed loop

```mermaid
flowchart LR
    Router -- rule(LightweightModule::rule) --> MsgRx[(WS Msg Receiver)] --> RunLoop[run()]
```

- No Handle or command/response. Great for keep-alive, monitoring, or augmenting state.

## Lightweight Handlers: global tap

```mermaid
flowchart LR
    Router -- every WS msg --> Handler1
    Router -- every WS msg --> Handler2
```

- Registered callbacks executed for all messages (e.g., logging).

## Middleware positioning

```mermaid
flowchart TB
    InboundWS[Inbound WS] --> PreRecv[Middleware: on_receive*] --> Router
    Handles --> PreSend[Middleware: on_send*] --> OutboundWS[Outbound WS]
```

- Middleware can inspect/modify inbound and outbound traffic globally.

## ClientBuilder, Runner, and module registration (sequence)

```mermaid
sequenceDiagram
    participant User
    participant Builder as ClientBuilder
    participant Router
    participant JoinSet
    participant Runner

    User->>Builder: with_module::<M>() / with_lightweight_module::<L>()
    Builder->>Router: register rule + channels
    Builder->>JoinSet: spawn handle registration (ApiModule only)
    Note over Router,Runner: Router owns rules and channels
    Builder->>Runner: build() -> Client + ClientRunner
    User->>Runner: run()
    Runner->>Router: start routing WS msgs
```

## Inbound message flow (detailed)

```mermaid
sequenceDiagram
    participant WS
    participant Conn as Connector
    participant Runner
    participant Middleware
    participant Router
    participant LWH as L. Handlers
    participant LWM as L. Modules
    participant AM as ApiModules

    WS-->>Conn: Message
    Conn-->>Runner: Message
    Runner->>Middleware: on_receive
    Middleware-->>Runner: possibly modified msg
    Runner->>Router: route(msg)
    Router->>LWH: broadcast
    Router->>LWM: if rule(msg)
    Router->>AM: if rule(msg)
```

## Outbound message flow (detailed)

```mermaid
sequenceDiagram
    participant Handle as Module Handle
    participant LWM as L. Modules
    participant Runner
    participant Middleware
    participant Conn as Connector
    participant WS

    Handle->>Runner: send(Message)
    LWM->>Runner: send(Message)
    Runner->>Middleware: on_send
    Middleware-->>Runner: possibly modified msg
    Runner->>Conn: send(msg)
    Conn->>WS: send(msg)
```

## Reconnect flow (high level)

```mermaid
sequenceDiagram
    participant Runner
    participant Reconn as ReconnectCallbackStack
    participant M as Module Callback

    Runner->>Reconn: on_reconnect()
    Reconn->>M: call(state, ws_sender)
    M-->>Runner: (re-subscribe, resend keep-alive, etc.)
```

## Where to look in the code

- Core: crates/core-pre/src
  - builder.rs: ClientBuilder (module registration, routing rules)
  - client.rs, connector.rs, router inside builder.rs
  - traits.rs: ApiModule, LightweightModule, AppState, Rule, ReconnectCallback
  - middleware.rs: Middleware stack
- PocketOption integration: crates/binary_options_tools/src/pocketoption
  - modules/\*: concrete modules (subscriptions, trades, server_time, raw, ...)
  - pocket_client.rs: registers modules and exposes get_handle helpers
