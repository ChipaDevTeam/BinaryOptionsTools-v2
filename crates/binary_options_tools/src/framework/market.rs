use crate::pocketoption::error::PocketResult;
use crate::pocketoption::types::Deal;
use async_trait::async_trait;
use rust_decimal::Decimal;
use uuid::Uuid;

/// The Market trait abstracts trading operations.
/// This allows strategies to run against live accounts, demo accounts, or local simulations (backtesting).
#[async_trait]
pub trait Market: Send + Sync {
    /// Executes a BUY (CALL) order.
    async fn buy(&self, asset: &str, amount: Decimal, time: u32) -> PocketResult<(Uuid, Deal)>;

    /// Executes a SELL (PUT) order.
    async fn sell(&self, asset: &str, amount: Decimal, time: u32) -> PocketResult<(Uuid, Deal)>;

    /// Returns the current balance.
    async fn balance(&self) -> Decimal;

    /// Checks the result of a trade.
    async fn result(&self, trade_id: Uuid) -> PocketResult<Deal>;
}
