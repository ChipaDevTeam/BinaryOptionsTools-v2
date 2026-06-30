---
sidebar_position: 2
slug: /architecture/dataflow
---

# System Architecture: Data Flow and Components

This document shows how data moves through the system: Client, Runner, Router, Middleware, ApiModules, LightweightModules, Lightweight Handlers, and Handles.

## Legend

- **WS**: WebSocket connection managed by the Runner via the Connector
- **Router**: multiplexes messages to modules and handlers using rules
- **Middleware**: pre-/post-processing for inbound/outbound WS messages
- **ApiModule**: full-featured module with commands, responses, and a Handle
- **LightweightModule**: background task, receives routed WS messages, no command/response
- **Lightweight Handler**: global stateless callback receiving every WS message

## End-to-end Overview

The data flow consists of:

1. **Inbound Path**: WebSocket → Connector → Runner → Middleware (inbound) → Router → Lightweight Handlers, LightweightModules, ApiModules via rules
2. **Outbound Path**: ApiModule via Handle, LightweightModule → Runner → Middleware (outbound) → Connector → WebSocket

## ApiModule internals: commands, responses, and routing

- The builder registers an M::Handle in a shared map. `Client.get_handle::<M>()` returns it.
- The module runs its own loop, reading commands and WS messages, emitting responses.

## LightweightModule internals: simple routed loop

- No Handle or command/response. Great for keep-alive, monitoring, or augmenting state.

## Lightweight Handlers: global tap

- Registered callbacks executed for all messages (e.g., logging).

## Middleware positioning

- Middleware can inspect/modify inbound and outbound traffic globally.

## ClientBuilder, Runner, and module registration (sequence)

## Inbound message flow (detailed)

## Outbound message flow (detailed)

## Reconnect flow (high level)

## Where to look in the code

- **Core**: `crates/core/src`
  - `builder.rs`: ClientBuilder (module registration, routing rules)
  - `client.rs`, `connector.rs`, `router` inside `builder.rs`
  - `traits.rs`: ApiModule, LightweightModule, AppState, Rule, ReconnectCallback
  - `middleware.rs`: Middleware stack
- **PocketOption integration**: `crates/binary_options_tools/src/pocketoption`
  - `modules/*`: concrete modules (subscriptions, trades, server_time, raw, ...)
  - `pocket_client.rs`: registers modules and exposes get_handle helpers