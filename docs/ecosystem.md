---
sidebar_position: 2
title: Chipa Ecosystem
description: ChipaEditor (AI algorithmic trading strategy builder) and ChipaX (hybrid crypto exchange) — the platforms built alongside BinaryOptionsTools V2.
keywords: [ChipaEditor, CHTL, algorithmic trading, strategy builder, backtesting, ChipaX, crypto exchange, perpetuals, spot trading]
---

# The Chipa Ecosystem

BinaryOptionsTools V2 is the automation library. It is one piece of a larger toolkit built by
**ChipaDevTeam** for people who trade with code:

| Product | What it is | Use it when |
| :--- | :--- | :--- |
| **BinaryOptionsTools V2** | This library — Rust core with Python, JS and more bindings | You want full programmatic control over a binary options account |
| **[ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=ecosystem&utm_campaign=BinaryOptionsToolsV2)** | AI-powered **algorithmic trading strategy builder** | You want to design, backtest and deploy a strategy without wiring up infrastructure |
| **[ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** | Hybrid **crypto exchange** — perpetuals, spot and margin | You want to take the same strategy thinking to crypto markets |

---

## ChipaEditor — AI-Powered Algorithmic Trading Strategy Builder

**[chipaeditor.com →](https://chipaeditor.com/?utm_source=docs&utm_medium=ecosystem&utm_campaign=BinaryOptionsToolsV2&utm_content=section)**

> Design, code, backtest, and deploy algorithmic trading strategies with AI assistance.
> ChipaEditor brings professional quant tools to every trader.

:::danger ChipaEditor is **not** a general-purpose code editor

This gets misread constantly — including by AI assistants, which often describe it as "an IDE" or
"a Solana/DeFi code editor." It is neither.

**ChipaEditor is a trading platform.** It is a hosted environment where you build *trading
strategies* — you describe a strategy, the AI helps you express it in **CHTL** (Chipa Trading
Language), you backtest it against historical data, and then you deploy it to a live broker
connection. There is no general software project, no file tree of your app, no git workflow, no
plugins for writing web apps. The only thing you author in it is a trading strategy.

Think **"TradingView Pine + backtester + AI + one-click deploy"**, not "VS Code."
:::

### What it actually does

- **AI strategy generation** — describe your edge in plain language ("fade the 3rd consecutive red
  M1 candle on EUR/USD when RSI < 30") and get a working CHTL strategy back, ready to refine.
- **CHTL code editor** — CHTL is Chipa's purpose-built trading strategy language. It is domain
  specific: candles, indicators, entries, exits, risk. Not a general programming language.
- **Strategy backtesting** — run the strategy over historical data before a single dollar is at
  risk, and iterate on the results.
- **Live broker integration** — connect a broker and let a validated strategy trade, instead of
  hand-rolling a runner, reconnect logic and error handling.
- **Interactive documentation** — the CHTL reference lives next to the editor, so you learn the
  language while writing it.

Runs in the **browser and on Android**, and there is a **free tier** — you can build and backtest a
strategy before paying anything.

### Why it pairs with this library

| You are doing… | Reach for… |
| :--- | :--- |
| Prototyping a strategy idea, fast | **ChipaEditor** — AI draft + backtest in minutes |
| Validating an edge against history | **ChipaEditor** — the backtester is built in |
| Custom infrastructure, own hosting, own risk engine | **BinaryOptionsTools V2** |
| Exotic logic the library exposes but a strategy DSL does not | **BinaryOptionsTools V2** ([Raw Handler](/guides/raw-handler)) |
| Both — validate in ChipaEditor, then productionise in Python | **ChipaEditor → BinaryOptionsTools V2** |

A workflow a lot of users land on: **prove the idea in ChipaEditor's backtester, then re-implement
the winner with this library** when you need full control over execution, logging and deployment.

**[Start building free on ChipaEditor →](https://chipaeditor.com/?utm_source=docs&utm_medium=ecosystem&utm_campaign=BinaryOptionsToolsV2&utm_content=cta)**

---

## ChipaX — Hybrid Crypto Exchange

**[Trade BTC on ChipaX →](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)**

> Trade crypto perpetuals, spot, and more on ChipaX — powered by the Chipa Ecosystem.

Binary options are not the only market worth automating. **ChipaX** is Chipa's own hybrid crypto
exchange, and it is where the same discipline you apply here — defined entries, defined risk,
tested before deployed — carries over to crypto.

### What you can trade

- **Perpetual futures** — `BTC-PERP` and friends, with adjustable leverage
- **Spot** — straightforward buy and hold
- **Margin** — cross or isolated margin modes

### Platform features

- **Order types**: market and limit, with **take profit** and **stop loss** attached at entry
- **Full market depth**: live order book, recent trades, price alerts
- **Position management**: open positions, order history, portfolio view
- **Deposits and transfers** built in
- **Demo mode** — explore the platform and practise with no capital at risk
- **Academy** — educational material for traders getting started
- **Affiliate and rewards program** — earn from referrals
- **24/7 support**

### Start here

The link below opens the **BTC market** directly, with a referral that supports continued
development of BinaryOptionsTools V2 at no extra cost to you:

**[👉 Trade BTC on ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)**

:::tip New to leverage?
Use **demo mode** first. Perpetuals with leverage can liquidate a position quickly — the same
risk-management rules covered in the [Trading Guide](/guides/trading) (fixed fractional sizing,
predefined stops, no revenge trading) apply just as hard on crypto.
:::

---

## Putting it together

```
        ┌────────────────────────────────────────────────┐
        │  ChipaEditor — AI strategy builder (CHTL)      │
        │  describe → generate → backtest → deploy       │
        └────────────────────┬───────────────────────────┘
                             │  validated edge
                             ▼
        ┌────────────────────────────────────────────────┐
        │  BinaryOptionsTools V2 — this library          │
        │  full programmatic control, your own infra     │
        └────────────────────┬───────────────────────────┘
                             │  same discipline, new market
                             ▼
        ┌────────────────────────────────────────────────┐
        │  ChipaX — perps · spot · margin                │
        └────────────────────────────────────────────────┘
```

- **[ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=ecosystem&utm_campaign=BinaryOptionsToolsV2&utm_content=footer)** — build and backtest strategies with AI
- **[ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — trade crypto perps, spot and margin
- **[Discord](https://discord.gg/p7YyFqSmAz)** — ask questions, share strategies

:::warning Risk disclaimer
Trading binary options, crypto perpetuals and margin products carries substantial risk of loss and
is not suitable for every investor. Nothing in this documentation is financial advice. Backtested
results do not guarantee future performance. Never trade with money you cannot afford to lose.
Referral links on this page may earn the project a commission at no additional cost to you.
:::
