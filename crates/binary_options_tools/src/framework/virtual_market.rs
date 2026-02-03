use std::collections::HashMap;
use tokio::sync::Mutex;
use async_trait::async_trait;
use uuid::Uuid;
use chrono::{Utc, DateTime};
use crate::pocketoption::error::PocketResult;
use crate::pocketoption::types::{Deal, Action};
use crate::framework::market::Market;

#[derive(Clone)]
struct VirtualTrade {
    pub id: Uuid,
    pub asset: String,
    pub action: Action,
    pub amount: f64,
    pub entry_price: f64,
    pub entry_time: i64,
    pub duration: u32,
    pub payout_percent: i32,
}

pub struct VirtualMarket {
    balance: Mutex<f64>,
    open_trades: Mutex<HashMap<Uuid, VirtualTrade>>,
    current_prices: Mutex<HashMap<String, f64>>,
    payouts: Mutex<HashMap<String, i32>>,
}

impl VirtualMarket {
    pub fn new(initial_balance: f64) -> Self {
        Self {
            balance: Mutex::new(initial_balance),
            open_trades: Mutex::new(HashMap::new()),
            current_prices: Mutex::new(HashMap::new()),
            payouts: Mutex::new(HashMap::new()),
        }
    }

    pub async fn update_price(&self, asset: &str, price: f64) {
        self.current_prices.lock().await.insert(asset.to_string(), price);
    }

    pub async fn set_payout(&self, asset: &str, payout: i32) {
        self.payouts.lock().await.insert(asset.to_string(), payout);
    }
}

#[async_trait]
impl Market for VirtualMarket {
    async fn buy(&self, asset: &str, amount: f64, time: u32) -> PocketResult<(Uuid, Deal)> {
        // Acquire locks in order: balance -> current_prices -> payouts -> open_trades
        let mut balance = self.balance.lock().await;
        if *balance < amount {
            return Err(crate::pocketoption::error::PocketError::General(
                "Insufficient virtual balance".into(),
            ));
        }

        let entry_price = *self
            .current_prices
            .lock()
            .await
            .get(asset)
            .ok_or_else(|| {
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
            close_price: 0.0,
            open_timestamp: entry_time,
            close_timestamp: entry_time + chrono::Duration::seconds(time as i64),
            profit: 0.0,
            percent_profit: payout,
            percent_loss: 100,
            command: 0, // Call
            uid: 0,
            request_id: Some(id),
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

    async fn sell(&self, asset: &str, amount: f64, time: u32) -> PocketResult<(Uuid, Deal)> {
        // Acquire locks in order: balance -> current_prices -> payouts -> open_trades
        let mut balance = self.balance.lock().await;
        if *balance < amount {
            return Err(crate::pocketoption::error::PocketError::General(
                "Insufficient virtual balance".into(),
            ));
        }

        let entry_price = *self
            .current_prices
            .lock()
            .await
            .get(asset)
            .ok_or_else(|| {
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
            close_price: 0.0,
            open_timestamp: entry_time,
            close_timestamp: entry_time + chrono::Duration::seconds(time as i64),
            profit: 0.0,
            percent_profit: payout,
            percent_loss: 100,
            command: 1, // Put
            uid: 0,
            request_id: Some(id),
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

    async fn balance(&self) -> f64 {
        *self.balance.lock().await
    }

    async fn result(&self, trade_id: Uuid) -> PocketResult<Deal> {
        let trade = {
            let open_trades = self.open_trades.lock().await;
            open_trades
                .get(&trade_id)
                .ok_or_else(|| {
                    crate::pocketoption::error::PocketError::General(format!(
                        "Trade {} not found",
                        trade_id
                    ))
                })?
                .clone() // Clone to drop the lock
        }; // open_trades lock dropped here

        // Now acquire locks in correct order if needed, but we mainly need current_prices later.
        // The check for expiry depends on time, which is constant for the trade.

        let current_time = Utc::now().timestamp();
        let expiry_time = trade.entry_time + trade.duration as i64;
        let entry_timestamp = DateTime::from_timestamp(trade.entry_time, 0).unwrap_or_default();
        let close_timestamp = DateTime::from_timestamp(expiry_time, 0).unwrap_or_default();

        if current_time < expiry_time {
            // Trade still open
            return Ok(Deal {
                id: trade.id,
                asset: trade.asset.clone(),
                amount: trade.amount,
                open_price: trade.entry_price,
                close_price: 0.0,
                open_timestamp: entry_timestamp,
                close_timestamp,
                profit: 0.0,
                percent_profit: trade.payout_percent,
                percent_loss: 100,
                command: match trade.action {
                    Action::Call => 0,
                    Action::Put => 1,
                },
                uid: 0,
                request_id: Some(trade.id),
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
            });
        }

        // Trade closed - need price
        // Lock order: balance -> current_prices -> payouts -> open_trades
        // We need balance (to add profit) and current_prices.
        // We already have the trade info.

        let mut balance = self.balance.lock().await;
        let close_price = *self
            .current_prices
            .lock()
            .await
            .get(&trade.asset)
            .unwrap_or(&trade.entry_price);

        let win = match trade.action {
            Action::Call => close_price > trade.entry_price,
            Action::Put => close_price < trade.entry_price,
        };

        let profit = if win {
            trade.amount * (1.0 + trade.payout_percent as f64 / 100.0)
        } else if close_price == trade.entry_price {
            trade.amount // Draw
        } else {
            0.0
        };

        if profit > 0.0 {
            *balance += profit;
        }

        // Finally remove from open_trades
        self.open_trades.lock().await.remove(&trade_id);

        let deal = Deal {
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
            request_id: Some(trade.id),
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
        };

        Ok(deal)
    }
}