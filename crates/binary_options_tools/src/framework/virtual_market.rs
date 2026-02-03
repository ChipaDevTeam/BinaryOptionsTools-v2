use std::collections::HashMap;
use tokio::sync::Mutex;
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;
use crate::pocketoption::error::PocketResult;
use crate::pocketoption::types::{Deal, Action};
use crate::framework::market::Market;

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
        let mut balance = self.balance.lock().await;
        if *balance < amount {
             return Err(crate::pocketoption::error::PocketError::General("Insufficient virtual balance".into()));
        }
        *balance -= amount;

        let entry_price = *self.current_prices.lock().await.get(asset).unwrap_or(&0.0);
        let payout = *self.payouts.lock().await.get(asset).unwrap_or(&80);
        let id = Uuid::new_v4();
        
        let trade = VirtualTrade {
            id,
            asset: asset.to_string(),
            action: Action::Call,
            amount,
            entry_price,
            entry_time: Utc::now().timestamp(),
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
            open_timestamp: Utc::now(),
            close_timestamp: Utc::now() + chrono::Duration::seconds(time as i64),
            profit: 0.0,
            percent_profit: payout,
            percent_loss: 100,
            command: 0, // Call
            uid: 0,
            request_id: Some(id),
            open_time: Utc::now().to_rfc3339(),
            close_time: (Utc::now() + chrono::Duration::seconds(time as i64)).to_rfc3339(),
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
         let mut balance = self.balance.lock().await;
        if *balance < amount {
             return Err(crate::pocketoption::error::PocketError::General("Insufficient virtual balance".into()));
        }
        *balance -= amount;

        let entry_price = *self.current_prices.lock().await.get(asset).unwrap_or(&0.0);
        let payout = *self.payouts.lock().await.get(asset).unwrap_or(&80);
        let id = Uuid::new_v4();
        
        let trade = VirtualTrade {
            id,
            asset: asset.to_string(),
            action: Action::Put,
            amount,
            entry_price,
            entry_time: Utc::now().timestamp(),
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
            open_timestamp: Utc::now(),
            close_timestamp: Utc::now() + chrono::Duration::seconds(time as i64),
            profit: 0.0,
            percent_profit: payout,
            percent_loss: 100,
            command: 1, // Put
            uid: 0,
            request_id: Some(id),
            open_time: Utc::now().to_rfc3339(),
            close_time: (Utc::now() + chrono::Duration::seconds(time as i64)).to_rfc3339(),
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

    async fn result(&self, _trade_id: Uuid) -> PocketResult<Deal> {
        // This is tricky because we need to know the price at expiry.
        // For a simple paper trader, we might need a background task that checks expiry.
        Err(crate::pocketoption::error::PocketError::General("Not implemented for virtual market yet".into()))
    }
}