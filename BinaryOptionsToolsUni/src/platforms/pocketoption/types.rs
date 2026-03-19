use binary_options_tools::pocketoption::{
    candle::Candle as OriginalCandle,
    types::{
        Action as OriginalAction, Asset as OriginalAsset, AssetType as OriginalAssetType,
        CandleLength as OriginalCandleLength, Deal as OriginalDeal,
        PendingOrder as OriginalPendingOrder,
    },
};
use bo2_macros::uniffi_doc;
use rust_decimal::prelude::ToPrimitive;

#[uniffi_doc(name = "Action", path = "BinaryOptionsToolsUni/docs_json/types.json")]
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

#[uniffi_doc(
    name = "AssetType",
    path = "BinaryOptionsToolsUni/docs_json/types.json"
)]
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

#[uniffi_doc(
    name = "CandleLength",
    path = "BinaryOptionsToolsUni/docs_json/types.json"
)]
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

#[uniffi_doc(name = "Asset", path = "BinaryOptionsToolsUni/docs_json/types.json")]
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

#[uniffi_doc(name = "Deal", path = "BinaryOptionsToolsUni/docs_json/types.json")]
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
            amount: deal.amount.to_f64().unwrap_or_default(),
            profit: deal.profit.to_f64().unwrap_or_default(),
            percent_profit: deal.percent_profit,
            percent_loss: deal.percent_loss,
            open_price: deal.open_price.to_f64().unwrap_or_default(),
            close_price: deal.close_price.to_f64().unwrap_or_default(),
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
            amount_usd: deal.amount_usd.and_then(|v| v.to_f64()),
            amount_usd2: deal.amount_usd2.and_then(|v| v.to_f64()),
        }
    }
}

#[uniffi_doc(
    name = "PendingOrder",
    path = "BinaryOptionsToolsUni/docs_json/types.json"
)]
#[derive(Debug, Clone, uniffi::Record)]
pub struct PendingOrder {
    pub ticket: String,
    pub open_type: u32,
    pub amount: f64,
    pub symbol: String,
    pub open_time: String,
    pub open_price: f64,
    pub timeframe: u32,
    pub min_payout: u32,
    pub command: u32,
    pub date_created: String,
    pub id: u64,
}

impl From<OriginalPendingOrder> for PendingOrder {
    fn from(order: OriginalPendingOrder) -> Self {
        Self {
            ticket: order.ticket.to_string(),
            open_type: order.open_type,
            amount: order.amount.to_f64().unwrap_or_default(),
            symbol: order.symbol,
            open_time: order.open_time,
            open_price: order.open_price.to_f64().unwrap_or_default(),
            timeframe: order.timeframe,
            min_payout: order.min_payout,
            command: order.command,
            date_created: order.date_created,
            id: order.id,
        }
    }
}

#[uniffi_doc(name = "Candle", path = "BinaryOptionsToolsUni/docs_json/types.json")]
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
            timestamp: candle.timestamp,
            open: candle.open.to_f64().unwrap_or_default(),
            high: candle.high.to_f64().unwrap_or_default(),
            low: candle.low.to_f64().unwrap_or_default(),
            close: candle.close.to_f64().unwrap_or_default(),
            volume: candle.volume.and_then(|v| v.to_f64()),
        }
    }
}
