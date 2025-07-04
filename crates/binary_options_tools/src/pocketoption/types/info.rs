use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};

use binary_options_tools_core::general::traits::MessageInformation;

use super::base::RawWebsocketMessage;

#[derive(Debug, Deserialize_enum_str, Serialize_enum_str, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum MessageInfo {
    OpenOrder,
    UpdateStream,
    UpdateHistoryNew,
    UpdateHistoryNewFast,
    UpdateAssets,
    UpdateBalance,
    SuccesscloseOrder,
    Auth,
    ChangeSymbol,
    SuccessupdateBalance,
    SuccessupdatePending,
    Successauth,
    UpdateOpenedDeals,
    UpdateClosedDeals,
    SuccessopenOrder,
    // UpdateCharts,
    SubscribeSymbol,
    LoadHistoryPeriod,
    FailopenOrder,
    GetCandles,
    OpenPendingOrder,
    SuccessopenPendingOrder,
    FailopenPendingOrder,
    None,

    #[serde(other)]
    Raw(String),
}

impl MessageInfo {
    pub fn get_raw(&self) -> Option<RawWebsocketMessage> {
        if let Self::Raw(raw) = self {
            Some(RawWebsocketMessage::from(raw.to_owned()))
        } else {
            None
        }
    }
}

impl MessageInformation for MessageInfo {}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_parse_message_info() -> Result<(), Box<dyn Error>> {
        dbg!(serde_json::to_string(&MessageInfo::OpenOrder)?);
        Ok(())
    }
}
