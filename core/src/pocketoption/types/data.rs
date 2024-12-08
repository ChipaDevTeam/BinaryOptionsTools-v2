use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::pocketoption::parser::message::WebSocketMessage;

use super::{order::{Deal, UpdateClosedDeals, UpdateOpenedDeals}, update::{UpdateAssets, UpdateBalance}};

#[derive(Default, Clone)]
pub struct Data {
    balance: Arc<Mutex<UpdateBalance>>,
    opened_deals: Arc<Mutex<UpdateOpenedDeals>>,
    closed_deals: Arc<Mutex<UpdateClosedDeals>>,
    payout_data: Arc<Mutex<HashMap<String, i32>>>,
    pending_requests: Arc<Mutex<HashMap<Uuid, tokio::sync::oneshot::Sender<WebSocketMessage>>>>
}

impl From<UpdateAssets> for HashMap<String, i32> {
    fn from(value: UpdateAssets) -> Self {
        value.0.iter().map(|a| (a.symbol.clone(), a.payout)).collect()
    }
}

impl Data {
    pub async fn update_balance(&self, balance: UpdateBalance)  {
        let mut blnc = self.balance.lock().await;
        *blnc = balance;
    }

    pub async fn get_balance(&self) -> UpdateBalance {
        self.balance.lock().await.clone()
    }

    pub async fn update_opened_deals(&self, deals: UpdateOpenedDeals) {
        let mut opened = self.opened_deals.lock().await;
        *opened = deals;
    }

    pub async fn get_opened_deals(&self) -> Vec<Deal> {
        self.opened_deals.lock().await.clone().0
    }

    pub async fn update_closed_deals(&self, deals: UpdateClosedDeals) {
        let mut closed = self.closed_deals.lock().await;
        *closed = deals;
    }

    pub async fn get_closed_deals(&self) -> Vec<Deal> {
        self.closed_deals.lock().await.clone().0
    }

    pub async fn update_payout_data(&self, payout: UpdateAssets) {
        let mut data = self.payout_data.lock().await;
        *data = payout.into();
    }

    pub async fn get_full_payout(&self) -> HashMap<String, i32> {
        self.payout_data.lock().await.clone()
    }

    pub async fn get_payout(&self, asset: impl ToString) -> Option<i32> {
        self.payout_data.lock().await.get(&asset.to_string()).cloned()
    }

    pub async fn add_user_request(&self, id: Uuid, sender: tokio::sync::oneshot::Sender<WebSocketMessage>) {
        let mut requests = self.pending_requests.lock().await;
        requests.insert(id, sender);
    }

    pub async fn get_request(&self, id: Uuid) -> Option<tokio::sync::oneshot::Sender<WebSocketMessage>> {
        let mut requests = self.pending_requests.lock().await;
        requests.remove(&id)
    }
}