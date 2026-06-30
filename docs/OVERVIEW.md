---
sidebar_position: 2
slug: /overview
---

# Documentation Overview

BinaryOptionsTools v2 features a modern, comprehensive documentation system built with Docusaurus. This system provides a dynamic, searchable, and maintainable documentation site.

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
2. Configuration is handled via `docusaurus.config.js` in the root.
3. Preview changes locally using `npm run start`.

## Quality and Coverage

- **6 Languages** covered with equivalent examples.
- **20+ API Methods** documented with parameters and return types.
- **100+ Code Snippets** ready for copy-pasting.
- **Interactive Guides** for complex features like Raw Handlers.

## Migration from MkDocs

This documentation was migrated from MkDocs Material to Docusaurus v3. Key improvements include:

- Better React-based theming and customization
- Improved MDX support for interactive components
- Faster build times with modern tooling
- Better TypeScript integration
- Enhanced Algolia DocSearch integration