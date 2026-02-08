use binary_options_tools::pocketoption::{
    candle::Candle as OriginalCandle,
    types::{
        Action as OriginalAction, Asset as OriginalAsset, AssetType as OriginalAssetType,
        CandleLength as OriginalCandleLength, Deal as OriginalDeal,
    },
};
use rust_decimal::prelude::ToPrimitive;

/// Represents the action to take in a trade.
///
/// This enum is used to specify whether a trade is a "Call" (buy) or a "Put" (sell).
/// It's a fundamental concept in binary options trading.
///
/// # Examples
///
/// ## Python
/// ```python
/// from binaryoptionstoolsuni import Action
///
/// buy_action = Action.CALL
/// sell_action = Action.PUT
/// ```
///
/// ## Swift
/// ```swift
/// import binaryoptionstoolsuni
///
/// let buyAction = Action.call
/// let sellAction = Action.put
/// ```
///
/// ## Kotlin
/// ```kotlin
/// import uniffi.binaryoptionstoolsuni.Action
///
/// val buyAction = Action.CALL
/// val sellAction = Action.PUT
/// ```
///
/// ## C#
/// ```csharp
/// using UniFFI.BinaryOptionsToolsUni;
///
/// var buyAction = Action.Call;
/// var sellAction = Action.Put;
/// ```
///
/// ## Go
/// ```go
/// import "github.com/your-repo/binaryoptionstoolsuni"
///
/// var buyAction = binaryoptionstoolsuni.ActionCall
/// var sellAction = binaryoptionstoolsuni.ActionPut
/// ```
#[derive(Debug, Clone, uniffi::Enum)]
pub enum Action {
    Call,
    Put,
}

impl From<OriginalAction> for Action {
    fn from(action: OriginalAction) -> Self {
        match action {
            OriginalAction::Call => Action::Call,
            OriginalAction::Put => Action::Put,
        }
    }
}

/// Represents the type of an asset.
///
/// This enum is used to categorize assets into different types, such as stocks, currencies, etc.
/// This information can be useful for filtering and organizing assets.
///
/// # Examples
///
/// ## Python
/// ```python
/// from binaryoptionstoolsuni import AssetType
///
/// asset_type = AssetType.CURRENCY
/// ```
#[derive(Debug, Clone, uniffi::Enum)]
pub enum AssetType {
    Stock,
    Currency,
    Commodity,
    Cryptocurrency,
    Index,
}

impl From<OriginalAssetType> for AssetType {
    fn from(asset_type: OriginalAssetType) -> Self {
        match asset_type {
            OriginalAssetType::Stock => AssetType::Stock,
            OriginalAssetType::Currency => AssetType::Currency,
            OriginalAssetType::Commodity => AssetType::Commodity,
            OriginalAssetType::Cryptocurrency => AssetType::Cryptocurrency,
            OriginalAssetType::Index => AssetType::Index,
        }
    }
}

/// Represents the duration of a candle.
///
/// This struct is a simple wrapper around a `u32` that represents the candle duration in seconds.
/// It is used in the `Asset` struct to specify the allowed candle lengths for an asset.
///
/// # Examples
///
/// ## Python
/// ```python
/// from binaryoptionstoolsuni import CandleLength
///
/// five_second_candle = CandleLength(time=5)
/// ```
#[derive(Debug, Clone, uniffi::Record)]
pub struct CandleLength {
    pub time: u32,
}

impl From<OriginalCandleLength> for CandleLength {
    fn from(candle_length: OriginalCandleLength) -> Self {
        Self {
            time: candle_length.duration(),
        }
    }
}

/// Represents a financial asset that can be traded.
///
/// This struct contains all the information about a specific asset, such as its name, symbol,
/// payout, and whether it's currently active.
///
/// # Examples
///
/// ## Python
/// ```python
/// from binaryoptionstoolsuni import Asset
///
/// # This is an example of how you might receive an Asset object
/// # from the API. You would not typically construct this yourself.
/// eurusd = Asset(id=1, name="EUR/USD", symbol="EURUSD_otc", is_otc=True, is_active=True, payout=85, allowed_candles=[], asset_type=AssetType.CURRENCY)
/// print(eurusd.name)
/// ```
#[derive(Debug, Clone, uniffi::Record)]
pub struct Asset {
    pub id: i32,
    pub name: String,
    pub symbol: String,
    pub is_otc: bool,
    pub is_active: bool,
    pub payout: i32,
    pub allowed_candles: Vec<CandleLength>,
    pub asset_type: AssetType,
}

impl From<OriginalAsset> for Asset {
    fn from(asset: OriginalAsset) -> Self {
        Self {
            id: asset.id,
            name: asset.name,
            symbol: asset.symbol,
            is_otc: asset.is_otc,
            is_active: asset.is_active,
            payout: asset.payout,
            allowed_candles: asset
                .allowed_candles
                .into_iter()
                .map(CandleLength::from)
                .collect(),
            asset_type: AssetType::from(asset.asset_type),
        }
    }
}

/// Represents a completed trade.
///
/// This struct contains all the information about a trade that has been opened and subsequently closed.
/// It includes details such as the open and close prices, profit, and timestamps.
///
/// # Examples
///
/// ## Python
/// ```python
/// from binaryoptionstoolsuni import Deal
///
/// # This is an example of how you might receive a Deal object
/// # from the API after a trade is completed.
/// # You would not typically construct this yourself.
/// deal = ... # receive from api.result()
/// print(f"Trade {deal.id} on {deal.asset} resulted in a profit of {deal.profit}")
/// ```
#[derive(Debug, Clone, uniffi::Record)]
pub struct Deal {
    pub id: String,
    pub open_time: String,
    pub close_time: String,
    pub open_timestamp: i64,
    pub close_timestamp: i64,
    pub uid: u64,
    pub request_id: Option<String>,
    pub amount: f64,
    pub profit: f64,
    pub percent_profit: i32,
    pub percent_loss: i32,
    pub open_price: f64,
    pub close_price: f64,
    pub command: i32,
    pub asset: String,
    pub is_demo: u32,
    pub copy_ticket: String,
    pub open_ms: i32,
    pub close_ms: Option<i32>,
    pub option_type: i32,
    pub is_rollover: Option<bool>,
    pub is_copy_signal: Option<bool>,
    pub is_ai: Option<bool>,
    pub currency: String,
    pub amount_usd: Option<f64>,
    pub amount_usd2: Option<f64>,
}

impl From<OriginalDeal> for Deal {
    fn from(deal: OriginalDeal) -> Self {
        Self {
            id: deal.id.to_string(),
            open_time: deal.open_time,
            close_time: deal.close_time,
            open_timestamp: deal.open_timestamp.timestamp(),
            close_timestamp: deal.close_timestamp.timestamp(),
            uid: deal.uid,
            request_id: deal.request_id.map(|id| id.to_string()),
            amount: deal.amount,
            profit: deal.profit,
            percent_profit: deal.percent_profit,
            percent_loss: deal.percent_loss,
            open_price: deal.open_price,
            close_price: deal.close_price,
            command: deal.command,
            asset: deal.asset,
            is_demo: deal.is_demo,
            copy_ticket: deal.copy_ticket,
            open_ms: deal.open_ms,
            close_ms: deal.close_ms,
            option_type: deal.option_type,
            is_rollover: deal.is_rollover,
            is_copy_signal: deal.is_copy_signal,
            is_ai: deal.is_ai,
            currency: deal.currency,
            amount_usd: deal.amount_usd,
            amount_usd2: deal.amount_usd2,
        }
    }
}

/// Represents a single candle in a price chart.
///
/// A candle represents the price movement of an asset over a specific time period.
/// It contains the open, high, low, and close (OHLC) prices for that period.
///
/// # Examples
///
/// ## Python
/// ```python
/// from binaryoptionstoolsuni import Candle
///
/// # This is an example of how you might receive a Candle object
/// # from the API.
/// candle = ... # receive from api.get_candles() or stream.next()
/// print(f"Candle for {candle.symbol} at {candle.timestamp}: O={candle.open}, H={candle.high}, L={candle.low}, C={candle.close}")
/// ```
#[derive(Debug, Clone, uniffi::Record)]
pub struct Candle {
    pub symbol: String,
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
}

impl From<OriginalCandle> for Candle {
    fn from(candle: OriginalCandle) -> Self {
        Self {
            symbol: candle.symbol,
            timestamp: candle.timestamp as i64,
            open: candle.open.to_f64().unwrap_or_default(),
            high: candle.high.to_f64().unwrap_or_default(),
            low: candle.low.to_f64().unwrap_or_default(),
            close: candle.close.to_f64().unwrap_or_default(),
            volume: candle.volume.and_then(|v| v.to_f64()),
        }
    }
}
