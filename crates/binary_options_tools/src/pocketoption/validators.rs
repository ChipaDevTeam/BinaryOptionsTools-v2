use uuid::Uuid;

use super::parser::message::WebSocketMessage;

pub fn order_validator(order_index: u64) -> impl Fn(&WebSocketMessage) -> bool + Send + Sync {
    move |message| {
        if let WebSocketMessage::SuccessopenOrder(order) = message {
            if order.request_id.is_some_and(|id| id == order_index) {
                return true;
            }
        }
        false
    }
}

pub fn candle_validator(index: u64) -> impl Fn(&WebSocketMessage) -> bool + Send + Sync {
    move |message| {
        if let WebSocketMessage::LoadHistoryPeriod(history) = message {
            if history
                .index
                .div_euclid(100)
                .abs_diff(index.div_euclid(100))
                <= 1
            {
                return true;
            }
        }
        false
    }
}

pub fn order_result_validator(order_id: Uuid) -> impl Fn(&WebSocketMessage) -> bool + Send + Sync {
    move |message| {
        if let WebSocketMessage::SuccesscloseOrder(orders) = message {
            if orders.deals.iter().any(|o| o == &order_id) {
                return true;
            }
        }
        false
    }
}

pub fn history_validator(
    asset: String,
    period: i64,
) -> impl Fn(&WebSocketMessage) -> bool + Send + Sync {
    move |message| {
        if let WebSocketMessage::UpdateHistoryNewFast(history) = message {
            if history.asset == asset && history.period == period {
                return true;
            }
        }
        false
    }
}
