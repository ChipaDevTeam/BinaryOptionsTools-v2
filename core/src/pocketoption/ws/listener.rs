use tokio::sync::mpsc::Sender;

use async_trait::async_trait;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::pocketoption::{error::{PocketOptionError, PocketResult}, parser::message::{self, WebSocketMessage}, types::{base::ChangeSymbol, data::Data, info::MessageInfo}};

use super::ssid::Ssid;

#[async_trait]
pub trait EventListener: Clone + Send + Sync + 'static {
    fn on_raw_message(&self, message: Message) -> PocketResult<Message> {
        Ok(message)
    }

    fn on_message(&self, message: WebSocketMessage) -> PocketResult<WebSocketMessage> {
        Ok(message)
    }

    async fn process_message(&self, message: &Message, previous: &MessageInfo, sender: &Sender<Message>, data: &Data) -> PocketResult<(Option<MessageInfo>, bool)> {
        Ok((None,false))
    }

    async fn on_raw_message_async(&self, message: Message) -> PocketResult<Message> {
        Ok(message)
    }

    async fn on_message_async(&self, message: WebSocketMessage) -> PocketResult<WebSocketMessage> {
        Ok(message)
    }
}


#[derive(Clone)]
pub struct Handler {
    ssid: Ssid
}

impl Handler {
    pub fn new(ssid: Ssid) -> Self {
        Self { ssid }
    }

    pub fn handle_binary_msg(&self, bytes: &Vec<u8>, previous: &MessageInfo) -> PocketResult<WebSocketMessage> {
        let msg = String::from_utf8(bytes.to_owned())?;
        let message = WebSocketMessage::parse_with_context(msg, previous)?;

        Ok(message)
    }

    pub async fn handle_text_msg(&self, text: &str, sender: &Sender<Message>) -> PocketResult<Option<MessageInfo>> {
        match text {
            _ if text.starts_with('0') && text.contains("sid") => {
                sender.send(Message::Text("40".into())).await?;
            },
            _ if text.starts_with("40") && text.contains("sid") => {
                sender.send(Message::Text(self.ssid.to_string())).await?;
            },
            _ if text == "2" => {
                sender.send(Message::Text("3".into())).await?;
                // write.send(Message::Text("3".into())).await.unwrap();
                // write.flush().await.unwrap();
            },
            _ if text.starts_with("451-") => {
                let msg = text.strip_prefix("451-").unwrap();
                let (info, _): (MessageInfo, Value) = serde_json::from_str(msg)?;
                if info == MessageInfo::UpdateClosedDeals {
                    sender.send(Message::Text(WebSocketMessage::ChangeSymbol(ChangeSymbol { asset: "AUDNZD_otc".into(), period: 60 }).to_string())).await?;
                }
                return Ok(Some(info));
            }
            _ => {}
        }
        
        Ok(None)
    }
}

#[async_trait::async_trait]
impl EventListener for Handler {
    async fn process_message(&self, message: &Message, previous: &MessageInfo, sender: &Sender<Message>, data: &Data) -> PocketResult<(Option<MessageInfo> ,bool)> {
        match message {
            Message::Binary(binary) => {
                let msg = self.handle_binary_msg(binary, previous)?;
                if let WebSocketMessage::UpdateStream(stream) = &msg {
                    match stream.0.first() {
                        Some(item) => data.update_server_time(item.time.timestamp()).await,
                        None => warn!("Missing data in 'updateStream' message")
                    }
                }
                if let Some(sender) = data.get_request(&msg).await? {
                    sender.send(msg)?;
                }
            },
            Message::Text(text) => {
                let res = self.handle_text_msg(text, sender).await?;
                println!("{:?}", res);
                return Ok((res, false))
            },
            Message::Frame(_) => {}, // TODO:
            Message::Ping(_) => {}, // TODO:
            Message::Pong(_) => {}, // TODO:
            Message::Close(_) => return Ok((None, true)),
        } 
        Ok((None, false))
    }   
}