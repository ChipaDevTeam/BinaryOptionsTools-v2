---
sidebar_position: 2
slug: /overview
---

# Documentation Overview

:::tip Part of the Chipa Ecosystem
Alongside this library, ChipaDevTeam builds
**[ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=overview&utm_campaign=BinaryOptionsToolsV2)** — an AI-powered **algorithmic trading strategy
builder** (AI strategy generation, CHTL editor, backtesting, live broker integration; free tier) —
and **[ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — a hybrid crypto exchange for perpetuals, spot and margin.
See the [Chipa Ecosystem overview](/ecosystem).
:::

BinaryOptionsTools v2 ships a modern, comprehensive documentation tree written as plain Markdown, so it stays searchable, diffable and readable straight from the repository.

## Documentation Structure

The documentation is organized into logical sections for easier navigation:

- **API Reference**: Complete guides for multi-language and Python-specific APIs.
- **Guides**: Practical tutorials for trading strategies, raw handlers, and platform specifics.
- **Architecture**: Deep dives into the internal data flow and project structure.
- **Project Info**: Deployment guides, roadmaps, and documentation summaries.
- **Examples**: Code examples in all supported languages.
- **Tutorials**: Step-by-step guides for getting started.

## Key Features

### 1. Unified Search

Instantly search through the entire documentation base, including code snippets and API methods.

### 2. Multi-Language Code Tabs

Switch between different programming languages (Python, Kotlin, Swift, Go, Ruby, C#) within the same code block to compare implementations.

### 3. Responsive Design

The documentation site is fully responsive, working perfectly on desktops, tablets, and mobile phones.

### 4. Dark/Light Mode

Choose your preferred viewing experience with built-in dark and light mode support.

### 5. Automated Deployment

Integrated with GitHub Actions to automatically build and deploy the latest documentation on every push to the main branch.

## Getting Started

### For Developers

1. Read the [Introduction](/intro) and [Overview](/overview).
2. Explore the [API Reference](/api/reference) for your preferred language.
3. Check out the [Trading Guide](/guides/trading) for implementation patterns.

### For Contributors

1. Documentation source is located in the `docs/` directory.
2. Every page is plain Markdown with Docusaurus-style frontmatter; no site
   generator configuration is tracked, so changes are reviewed in a diff or a
   Markdown preview rather than a local server.
3. When you move or rename a file, update the links that point at it — see
   [Deployment](./project/deployment.md).

## Quality and Coverage

- **6 Languages** covered with equivalent examples.
- **20+ API Methods** documented with parameters and return types.
- **100+ Code Snippets** ready for copy-pasting.
- **Interactive Guides** for complex features like Raw Handlers.

## History

These pages carry frontmatter written for Docusaurus v3, which the project used
after migrating away from MkDocs Material. The generated site was removed in
0.2.15; the Markdown remains the source of truth, and
[Deployment](./project/deployment.md) explains how to build a site from it again.
