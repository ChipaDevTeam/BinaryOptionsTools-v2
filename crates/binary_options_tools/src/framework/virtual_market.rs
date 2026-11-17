use crate::framework::market::Market;
use crate::pocketoption::error::PocketResult;
use crate::pocketoption::types::{Deal, RequestId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VirtualTrade {
    id: Uuid,
    asset: String,
    action: Action,
    amount: Decimal,
    entry_price: Decimal,
    entry_time: i64,
    duration: u32,
    payout_percent: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
enum Action {
    Call,
    Put,
}

pub struct VirtualMarket {
    balance: Mutex<Decimal>,
    open_trades: Mutex<HashMap<Uuid, VirtualTrade>>,
    /// Trades whose window has closed, with the outcome they settled at.
    /// Kept so `result()` can answer the same question twice.
    settled_trades: Mutex<HashMap<Uuid, Deal>>,
    current_prices: Mutex<HashMap<String, Decimal>>,
    payouts: Mutex<HashMap<String, i32>>,
}

impl VirtualMarket {
    pub fn new(initial_balance: Decimal) -> Self {
        Self {
            balance: Mutex::new(initial_balance),
            open_trades: Mutex::new(HashMap::new()),
            settled_trades: Mutex::new(HashMap::new()),
            current_prices: Mutex::new(HashMap::new()),
            payouts: Mutex::new(HashMap::new()),
        }
    }

    /// Records a price tick and settles anything whose window it closes.
    ///
    /// The tick is the only moment a price at expiry exists, so settlement
    /// happens here rather than when someone asks for the result.
    pub async fn update_price(&self, asset: &str, price: Decimal) {
        self.current_prices
            .lock()
            .await
            .insert(asset.to_string(), price);

        self.settle_expired(asset, price).await;
    }

    pub async fn set_payout(&self, asset: &str, payout: i32) {
        self.payouts.lock().await.insert(asset.to_string(), payout);
    }
}

#[async_trait]
impl Market for VirtualMarket {
    async fn buy(&self, asset: &str, amount: Decimal, time: u32) -> PocketResult<(Uuid, Deal)> {
        if amount <= dec!(0.0) {
            return Err(crate::pocketoption::error::PocketError::General(
                "Amount must be a positive number".into(),
            ));
        }

        // Acquire locks in order: balance -> current_prices -> payouts -> open_trades
        let mut balance = self.balance.lock().await;
        if *balance < amount {
            return Err(crate::pocketoption::error::PocketError::General(
                "Insufficient virtual balance".into(),
            ));
        }

        let entry_price = *self.current_prices.lock().await.get(asset).ok_or_else(|| {
            crate::pocketoption::error::PocketError::General(format!(
                "Price not found for asset: {}",
                asset
            ))
        })?;

        let payout = *self.payouts.lock().await.get(asset).unwrap_or(&80);

        *balance -= amount;

        let id = Uuid::new_v4();
        let entry_time = Utc::now();

        let trade = VirtualTrade {
            id,
            asset: asset.to_string(),
            action: Action::Call,
            amount,
            entry_price,
            entry_time: entry_time.timestamp(),
            duration: time,
            payout_percent: payout,
        };

        self.open_trades.lock().await.insert(id, trade);

        // Return a mock deal
        let deal = Deal {
            id,
            asset: asset.to_string(),
            amount,
            open_price: entry_price,
            close_price: dec!(0.0),
            open_timestamp: entry_time,
            close_timestamp: entry_time + chrono::Duration::seconds(time as i64),
            profit: dec!(0.0),
            percent_profit: payout,
            percent_loss: 100,
            command: 0, // Call
            uid: 0,
            request_id: Some(RequestId::Uuid(id)),
            open_time: entry_time.to_rfc3339(),
            close_time: (entry_time + chrono::Duration::seconds(time as i64)).to_rfc3339(),
            refund_time: None,
            refund_timestamp: None,
            is_demo: 1,
            copy_ticket: "".to_string(),
            open_ms: 0,
            close_ms: None,
            option_type: 100,
            is_rollover: None,
            is_copy_signal: None,
            is_ai: None,
            currency: "USD".to_string(),
            amount_usd: Some(amount),
            amount_usd2: Some(amount),
        };

        Ok((id, deal))
    }

    async fn sell(&self, asset: &str, amount: Decimal, time: u32) -> PocketResult<(Uuid, Deal)> {
        if amount <= dec!(0.0) {
            return Err(crate::pocketoption::error::PocketError::General(
                "Amount must be a positive number".into(),
            ));
        }

        // Acquire locks in order: balance -> current_prices -> payouts -> open_trades
        let mut balance = self.balance.lock().await;
        if *balance < amount {
            return Err(crate::pocketoption::error::PocketError::General(
                "Insufficient virtual balance".into(),
            ));
        }

        let entry_price = *self.current_prices.lock().await.get(asset).ok_or_else(|| {
            crate::pocketoption::error::PocketError::General(format!(
                "Price not found for asset: {}",
                asset
            ))
        })?;

        let payout = *self.payouts.lock().await.get(asset).unwrap_or(&80);

        *balance -= amount;

        let id = Uuid::new_v4();
        let entry_time = Utc::now();

        let trade = VirtualTrade {
            id,
            asset: asset.to_string(),
            action: Action::Put,
            amount,
            entry_price,
            entry_time: entry_time.timestamp(),
            duration: time,
            payout_percent: payout,
        };

        self.open_trades.lock().await.insert(id, trade);

        // Return a mock deal
        let deal = Deal {
            id,
            asset: asset.to_string(),
            amount,
            open_price: entry_price,
            close_price: dec!(0.0),
            open_timestamp: entry_time,
            close_timestamp: entry_time + chrono::Duration::seconds(time as i64),
            profit: dec!(0.0),
            percent_profit: payout,
            percent_loss: 100,
            command: 1, // Put
            uid: 0,
            request_id: Some(RequestId::Uuid(id)),
            open_time: entry_time.to_rfc3339(),
            close_time: (entry_time + chrono::Duration::seconds(time as i64)).to_rfc3339(),
            refund_time: None,
            refund_timestamp: None,
            is_demo: 1,
            copy_ticket: "".to_string(),
            open_ms: 0,
            close_ms: None,
            option_type: 100,
            is_rollover: None,
            is_copy_signal: None,
            is_ai: None,
            currency: "USD".to_string(),
            amount_usd: Some(amount),
            amount_usd2: Some(amount),
        };

        Ok((id, deal))
    }

    async fn balance(&self) -> Decimal {
        *self.balance.lock().await
    }

    /// The settled outcome of `trade_id`, or the still-open trade.
    ///
    /// A pure read: settlement happens on the tick, so asking twice cannot
    /// change the answer, and an expired trade is never repriced by the caller's
    /// arrival time.
    async fn result(&self, trade_id: Uuid) -> PocketResult<Deal> {
        if let Some(deal) = self.settled_trades.lock().await.get(&trade_id) {
            return Ok(deal.clone());
        }

        let open_trades = self.open_trades.lock().await;
        let trade = open_trades.get(&trade_id).ok_or_else(|| {
            crate::pocketoption::error::PocketError::General(format!(
                "Trade {} not found",
                trade_id
            ))
        })?;

        // Nothing has ticked at or after expiry yet, so there is no price to
        // settle at: the stake stays debited and the trade stays open.
        Ok(build_deal(trade, dec!(0.0), dec!(0.0)))
    }
}

impl VirtualMarket {
    /// Settles every trade on `asset` whose window has closed, at `price` — the
    /// first price observed at or after expiry.
    async fn settle_expired(&self, asset: &str, price: Decimal) {
        let now = Utc::now().timestamp();
        let expired: Vec<VirtualTrade> = {
            let mut open_trades = self.open_trades.lock().await;
            let ids: Vec<Uuid> = open_trades
                .iter()
                .filter(|(_, trade)| {
                    trade.asset == asset && now >= trade.entry_time + trade.duration as i64
                })
                .map(|(id, _)| *id)
                .collect();
            ids.into_iter()
                .filter_map(|id| open_trades.remove(&id))
                .collect()
        };
        if expired.is_empty() {
            return;
        }

        let mut balance = self.balance.lock().await;
        let mut settled = self.settled_trades.lock().await;
        for trade in expired {
            let profit = profit_for(&trade, price);
            let total_payout = trade.amount + profit;
            if total_payout > dec!(0.0) {
                *balance += total_payout;
            }
            settled.insert(trade.id, build_deal(&trade, price, profit));
        }
    }
}

/// Payoff of `trade` at `close_price`, mirroring the broker's rules.
fn profit_for(trade: &VirtualTrade, close_price: Decimal) -> Decimal {
    let draw = close_price == trade.entry_price;
    let win = !draw
        && match trade.action {
            Action::Call => close_price > trade.entry_price,
            Action::Put => close_price < trade.entry_price,
        };

    if win {
        trade.amount * Decimal::from(trade.payout_percent) / dec!(100.0)
    } else if draw {
        dec!(0.0)
    } else {
        -trade.amount
    }
}

/// Builds the `Deal` a caller sees for `trade` at `close_price`.
fn build_deal(trade: &VirtualTrade, close_price: Decimal, profit: Decimal) -> Deal {
    let entry_timestamp = DateTime::from_timestamp(trade.entry_time, 0).unwrap_or_default();
    let close_timestamp =
        DateTime::from_timestamp(trade.entry_time + trade.duration as i64, 0).unwrap_or_default();

    Deal {
        id: trade.id,
        asset: trade.asset.clone(),
        amount: trade.amount,
        open_price: trade.entry_price,
        close_price,
        open_timestamp: entry_timestamp,
        close_timestamp,
        profit,
        percent_profit: trade.payout_percent,
        percent_loss: 100,
        command: match trade.action {
            Action::Call => 0,
            Action::Put => 1,
        },
        uid: 0,
        request_id: Some(RequestId::Uuid(trade.id)),
        open_time: entry_timestamp.to_rfc3339(),
        close_time: close_timestamp.to_rfc3339(),
        refund_time: None,
        refund_timestamp: None,
        is_demo: 1,
        copy_ticket: "".to_string(),
        open_ms: 0,
        close_ms: None,
        option_type: 100,
        is_rollover: None,
        is_copy_signal: None,
        is_ai: None,
        currency: "USD".to_string(),
        amount_usd: Some(trade.amount),
        amount_usd2: Some(trade.amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;

    fn dec(value: f64) -> Decimal {
        Decimal::from_f64(value).unwrap()
    }

    #[tokio::test]
    async fn a_closed_window_settles_at_the_price_the_next_tick_carries() {
        let market = VirtualMarket::new(dec(100.0));
        market.update_price("A", dec(1.0)).await;

        let (id, _deal) = market.buy("A", dec(10.0), 1).await.unwrap();
        assert_eq!(
            market.balance().await,
            dec(90.0),
            "stake is debited up front"
        );

        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        // Past expiry, asking for the result changes nothing: the stake is still
        // out because no price has been observed at or after expiry yet.
        assert_eq!(market.balance().await, dec(90.0));
        let still_open = market.result(id).await.unwrap();
        assert_eq!(still_open.profit, dec!(0.0));

        // The first tick after expiry is the price the window closed at.
        market.update_price("A", dec(3.0)).await;
        assert_eq!(
            market.balance().await,
            dec(108.0),
            "stake returned plus the 80% payout"
        );

        let settled = market.result(id).await.unwrap();
        assert_eq!(settled.profit, dec!(8.0));
        assert_eq!(settled.close_price, dec!(3.0));

        // A later tick must not reprice a settled trade, and asking again must
        // give the same answer.
        market.update_price("A", dec(0.5)).await;
        assert_eq!(market.balance().await, dec(108.0));
        assert_eq!(market.result(id).await.unwrap().profit, dec!(8.0));
        assert_eq!(market.result(id).await.unwrap().close_price, dec!(3.0));
    }

    #[tokio::test]
    async fn an_unexpired_trade_is_untouched_by_ticks() {
        let market = VirtualMarket::new(dec(100.0));
        market.update_price("A", dec(1.0)).await;

        let (id, _deal) = market.buy("A", dec(10.0), 60).await.unwrap();
        market.update_price("A", dec(5.0)).await;

        assert_eq!(market.balance().await, dec(90.0));
        assert_eq!(market.result(id).await.unwrap().profit, dec!(0.0));
        assert_eq!(market.result(id).await.unwrap().close_price, dec!(0.0));
    }

    #[tokio::test]
    async fn an_unknown_trade_is_reported_as_not_found() {
        let market = VirtualMarket::new(dec(100.0));
        let err = market.result(Uuid::new_v4()).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
