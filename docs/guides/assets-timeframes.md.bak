# Supported Assets and Timeframes

This document lists all supported assets and timeframes for the BinaryOptionsTools-v2 API.

## Supported Timeframes

The following timeframes are supported for trading and candle data:

| Timeframe | Duration (seconds) | Description |
| --------- | ------------------ | ----------- |
| 5s        | 5                  | 5 seconds   |
| 10s       | 10                 | 10 seconds  |
| 15s       | 15                 | 15 seconds  |
| 20s       | 20                 | 20 seconds  |
| 30s       | 30                 | 30 seconds  |
| 1m        | 60                 | 1 minute    |
| 2m        | 120                | 2 minutes   |
| 3m        | 180                | 3 minutes   |
| 5m        | 300                | 5 minutes   |
| 10m       | 600                | 10 minutes  |
| 15m       | 900                | 15 minutes  |
| 30m       | 1800               | 30 minutes  |
| 45m       | 2700               | 45 minutes  |
| 1h        | 3600               | 1 hour      |
| 2h        | 7200               | 2 hours     |
| 3h        | 10800              | 3 hours     |
| 4h        | 14400              | 4 hours     |

## Supported Assets

The API supports a wide range of assets across different categories:

### Forex Pairs (Currencies)

#### Major Pairs

- **EUR/USD** - Euro vs US Dollar (Symbols: `EURUSD`, `EURUSD_otc`)
- **GBP/USD** - British Pound vs US Dollar (Symbols: `GBPUSD`, `GBPUSD_otc`)
- **USD/JPY** - US Dollar vs Japanese Yen (Symbols: `USDJPY`, `USDJPY_otc`)
- **USD/CAD** - US Dollar vs Canadian Dollar (Symbols: `USDCAD`, `USDCAD_otc`)
- **USD/CHF** - US Dollar vs Swiss Franc (Symbols: `USDCHF`, `USDCHF_otc`)

#### Cross Pairs

- **EUR/GBP** - Euro vs British Pound (Symbols: `EURGBP`, `EURGBP_otc`)
- **EUR/JPY** - Euro vs Japanese Yen (Symbols: `EURJPY`, `EURJPY_otc`)
- **EUR/CHF** - Euro vs Swiss Franc (Symbols: `EURCHF`, `EURCHF_otc`)
- **GBP/JPY** - British Pound vs Japanese Yen (Symbols: `GBPJPY`, `GBPJPY_otc`)
- **AUD/USD** - Australian Dollar vs US Dollar (Symbols: `AUDUSD`, `AUDUSD_otc`)
- **NZD/USD** - New Zealand Dollar vs US Dollar (Symbols: `NZDUSD_otc`)
- And many more (see complete list below)

### Cryptocurrencies

- **BTC** - Bitcoin (Symbols: `BTCUSD`, `BTCUSD_otc`)
- **ETH** - Ethereum (Symbols: `ETHUSD`, `ETHUSD_otc`)
- **LTC** - Litecoin (Symbol: `LTCUSD_otc`)
- **XRP** - Ripple (Symbol: `XRPUSD_otc`)
- **BCH** - Bitcoin Cash (Symbols: `BCHEUR`, `BCHGBP`, `BCHJPY`)
- **DOGE** - Dogecoin (Symbol: `DOGE_otc`)
- **ADA** - Cardano (Symbol: `ADA-USD_otc`)
- **SOL** - Solana (Symbol: `SOL-USD_otc`)
- **MATIC** - Polygon (Symbol: `MATIC_otc`)
- **AVAX** - Avalanche (Symbol: `AVAX_otc`)
- **BNB** - Binance Coin (Symbol: `BNB-USD_otc`)
- **TON** - Toncoin (Symbol: `TON-USD_otc`)
- **TRX** - Tron (Symbol: `TRX-USD_otc`)
- **LINK** - Chainlink (Symbol: `LINK_otc`)
- **DOT** - Polkadot (Symbol: `DOTUSD_otc`)

### Commodities

- **GOLD** - Gold vs US Dollar (Symbols: `XAUUSD`, `XAUUSD_otc`)
- **SILVER** - Silver vs US Dollar (Symbols: `XAGUSD`, `XAGUSD_otc`)
- **OIL** - US Crude Oil (Symbols: `USCrude`, `USCrude_otc`)
- **BRENT** - UK Brent Oil (Symbols: `UKBrent`, `UKBrent_otc`)
- **NATURAL GAS** - Natural Gas (Symbols: `XNGUSD`, `XNGUSD_otc`)
- **PALLADIUM** - Palladium (Symbols: `XPDUSD`, `XPDUSD_otc`)
- **PLATINUM** - Platinum (Symbols: `XPTUSD`, `XPTUSD_otc`)

### Stock Indices

- **S&P 500** - US Stock Index (Symbols: `SP500`, `SP500_otc`)
- **NASDAQ** - NASDAQ Composite (Symbols: `NASUSD`, `NASUSD_otc`)
- **DOW JONES** - Dow Jones Industrial Average (Symbols: `DJI30`, `DJI30_otc`)
- **NIKKEI 225** - Japanese Stock Index (Symbols: `JPN225`, `JPN225_otc`)
- **DAX 30** - German Stock Index (Symbols: `D30EUR`, `D30EUR_otc`)
- **FTSE 100** - UK Stock Index (Symbols: `100GBP`, `100GBP_otc`)
- **CAC 40** - French Stock Index (Symbol: `CAC40`)
- **AUS 200** - Australian Stock Index (Symbols: `AUS200`, `AUS200_otc`)

### Individual Stocks

- **Apple** (Symbols: `#AAPL`, `#AAPL_otc`)
- **Microsoft** (Symbols: `#MSFT`, `#MSFT_otc`)
- **Amazon** (Symbol: `AMZN_otc`)
- **Tesla** (Symbols: `#TSLA`, `#TSLA_otc`)
- **Meta/Facebook** (Symbols: `#FB`, `#FB_otc`)
- **Netflix** (Symbols: `NFLX`, `NFLX_otc`)
- **Alibaba** (Symbols: `BABA`, `BABA_otc`)
- **Twitter** (Symbols: `TWITTER`, `TWITTER_otc`)
- And many more

## Complete Asset List

For a complete list of all available assets, see the [assets.txt](../data/assets.txt) file in the repository.

## Asset Symbol Format

Assets come in two variants:

- **Regular** - Standard trading hours (e.g., `EURUSD`, `BTCUSD`)
- **OTC** (Over-The-Counter) - Available outside standard trading hours (e.g., `EURUSD_otc`, `BTCUSD_otc`)

## Checking Asset Availability

To check if an asset is available for trading at a specific timeframe, use the asset validation methods:

### Python

```python
from binaryoptionstoolsv2 import PocketOptionAPI

async with PocketOptionAPI(ssid="your_ssid") as api:
    assets = await api.get_assets()
    asset = assets.get("EURUSD")

    # Check if asset supports 10 second timeframe
    try:
        asset.validate(10)  # Will raise error if not supported
        print("10s timeframe is supported")
    except Exception as e:
        print(f"Not supported: {e}")
```

### Rust

```rust
use binary_options_tools::pocketoption::PocketOption;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PocketOption::new("your_ssid").await?;
    let assets = client.get_assets().await?;

    if let Some(asset) = assets.get("EURUSD") {
        // Check if asset supports 10 second timeframe
        match asset.validate(10) {
            Ok(_) => println!("10s timeframe is supported"),
            Err(e) => println!("Not supported: {}", e),
        }
    }

    Ok(())
}
```

## Notes

- Asset availability may vary depending on market hours
- OTC assets are available 24/7 but may have different payout rates
- Some timeframes may only be available for specific assets
- Always validate asset and timeframe combination before placing trades

## Version Information

This documentation is for BinaryOptionsTools-v2. Features and supported assets may change in future releases.

For the latest updates, check the [GitHub repository](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2).
