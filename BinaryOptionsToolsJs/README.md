# Binary Options Tools for JavaScript/TypeScript
[![npm version](https://badge.fury.io/js/@rick-29%2Fbinary-options-tools.svg)](https://badge.fury.io/js/@rick-29%2Fbinary-options-tools)

A powerful Node.js library for binary options trading, providing programmatic access to various trading platforms. Currently supports Pocket Option with both real and demo accounts.

## Installation

```bash
# Using npm
npm install @rick-29/binary-options-tools

# Using yarn
yarn add @rick-29/binary-options-tools
```

## Features

- **Trading Operations**
  - Place buy/sell trades for any asset
  - Check trade results (win/loss/draw)
  - Real-time trade monitoring
  
- **Market Data**
  - Fetch historical candle data
  - Real-time price updates via WebSocket
  - Asset payout information
  
- **Account Management**
  - Get account balance
  - View opened trades
  - Access trade history
  
- **Advanced Features**
  - Comprehensive logging system
  - Both Promise-based and streaming APIs
  - TypeScript support with full type definitions

## Quick Start

```javascript
const { PocketOption } = require('@rick-29/binary-options-tools');
// Or using ES modules:
// import { PocketOption } from '@rick-29/binary-options-tools';

async function main() {
    // Initialize client with your session ID
    const client = new PocketOption('your-session-id');
    
    // Wait for connection
    await client.connect();
    
    // Get account balance
    const balance = await client.balance();
    console.log('Current balance:', balance);
    
    // Place a trade
    const tradeId = await client.buy('EUR/USD', 10, 60); // $10 trade for 1 minute
    console.log('Trade placed:', tradeId);
    
    // Check trade result
    const result = await client.checkWin(tradeId);
    console.log('Trade result:', result); // 'win', 'loss', or 'draw'
}

main().catch(console.error);
```

## Detailed API Reference

### PocketOption Class

#### Constructor
```javascript
const client = new PocketOption(ssid: string);
```

#### Trading Methods

##### `buy(asset: string, amount: number, duration: number): Promise<string>`
Places a buy (call) trade.
```javascript
const tradeId = await client.buy('EUR/USD', 25, 30); // $25 trade for 30 seconds
```

##### `sell(asset: string, amount: number, duration: number): Promise<string>`
Places a sell (put) trade.
```javascript
const tradeId = await client.sell('EUR/USD', 25, 30);
```

##### `checkWin(id: string): Promise<string>`
Checks the result of a trade.
```javascript
const result = await client.checkWin(tradeId);
console.log(result); // 'win', 'loss', or 'draw'
```

#### Market Data Methods

##### `history(asset: string, period: number): Promise<string>`
Fetches historical candle data.
```javascript
const history = await client.history('EUR/USD', 60);
const data = JSON.parse(history);
console.log(`Retrieved ${data.length} candles`);
```

##### `subscribeSymbol(symbol: string): Promise<StreamIterator>`
Subscribes to real-time price updates.
```javascript
const stream = await client.subscribeSymbol('EUR/USD');
for await (const update of stream) {
    const data = JSON.parse(update);
    console.log('New price:', data.price);
}
```

#### Account Methods

##### `balance(): Promise<number>`
Gets current account balance.
```javascript
const balance = await client.balance();
```

##### `openedDeals(): Promise<string>`
Lists all currently open trades.
```javascript
const deals = await client.openedDeals();
const openTrades = JSON.parse(deals);
```

##### `closedDeals(): Promise<string>`
Lists recent closed trades.
```javascript
const history = await client.closedDeals();
const trades = JSON.parse(history);
```

### Logging System

The library includes a powerful logging system for debugging and monitoring.

```javascript
const { LogBuilder, Logger } = require('@rick-29/binary-options-tools');

// Configure logging
const builder = new LogBuilder();
builder.terminal('DEBUG'); // Console output
builder.logFile('./trading.log', 'INFO'); // File output
builder.build();

// Create logger
const logger = new Logger();
logger.info('Trading session started');
logger.debug('Processing trade...');
```

#### Real-time Log Streaming

```javascript
const builder = new LogBuilder();
const iterator = builder.createLogsIterator('DEBUG');

// Stream logs in real-time
for await (const log of iterator) {
    console.log('New log:', log);
}
```

## Platform Support

Currently supported platforms:
- Pocket Option (Quick Trading)

Planned support:
- Expert Option
- Additional platforms (coming soon)

## System Requirements

- Node.js 10.0 or later
- Supported platforms: Windows, macOS, Linux
- Both x64 and ARM architectures supported

## Troubleshooting

### "Cannot find module '@rick-29/binary-options-tools-[platform]'" Error

If you encounter an error like `Cannot find module '@rick-29/binary-options-tools-win32-x64-msvc'`, this means the native binary for your platform is not available. Here's how to resolve it:

**For Users:**
1. Make sure you have the latest version: `npm update @rick-29/binary-options-tools`
2. Clear npm cache: `npm cache clean --force`
3. Reinstall the package: `npm uninstall @rick-29/binary-options-tools && npm install @rick-29/binary-options-tools`
4. If the issue persists, check if your platform is supported in the [System Requirements](#system-requirements) section

**For Developers:**
1. Build the native binaries: `npm run build`
2. Set up development environment: `npm run setup-dev`
3. Check that your platform is configured in the `napi.triples` section of `package.json`

**Supported Platforms:**
- Windows: x64, ia32, arm64 (MSVC)
- macOS: x64, arm64, universal
- Linux: x64, arm64, arm (GNU/musl)
- FreeBSD: x64
- Android: arm64, arm

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- [Discord Community](https://discord.gg/p7YyFqSmAz)
- [GitHub Issues](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)

## Acknowledgments

This JavaScript module is part of the BinaryOptionsTools-v2 project, which provides binary options trading tools across multiple programming languages.