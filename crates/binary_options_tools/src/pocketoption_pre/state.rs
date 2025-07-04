use binary_options_tools_core_pre::traits::AppState;

use crate::pocketoption_pre::{error::{PocketError, PocketResult}, ssid::Ssid};

pub struct State {
    pub ssid: Ssid,
    pub default_connection_url: Option<String>,
    pub default_symbol: String,
}

#[derive(Default)]
pub struct StateBuilder {
    ssid: Option<Ssid>,
    default_connection_url: Option<String>,
    default_symbol: Option<String>,
}

impl StateBuilder {
    pub fn ssid(mut self, ssid: Ssid) -> Self {
        self.ssid = Some(ssid);
        self
    }

    pub fn default_connection_url(mut self, url: String) -> Self {
        self.default_connection_url = Some(url);
        self
    }

    pub fn default_symbol(mut self, symbol: String) -> Self {
        self.default_symbol = Some(symbol);
        self
    }

    pub fn build(self) -> PocketResult<State> {
        Ok(State {
                    ssid: self.ssid.ok_or(PocketError::StateBuilder("SSID is required".into()))?,
                    default_connection_url: self.default_connection_url,
                    default_symbol: self.default_symbol.unwrap_or_else(|| "EURUSD_otc".to_string()),
                })
    }
}

impl AppState for State {
    fn clear_temporal_data(&self) {
        // Clear any temporary data associated with the state
    }
}
