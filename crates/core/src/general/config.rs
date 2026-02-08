use std::{collections::HashSet, time::Duration};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::constants::{MAX_ALLOWED_LOOPS, RECONNECT_CALLBACK, SLEEP_INTERVAL, TIMEOUT_TIME};

use super::{
    traits::{DataHandler, InnerConfig, MessageTransfer},
    types::Callback,
};
use binary_options_tools_macros::Config;

#[derive(Serialize, Deserialize, Config)]
pub struct _Config<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig> {
    pub max_allowed_loops: u32,
    pub sleep_interval: u64,
    #[config(extra(iterator(dtype = "Url", add_fn = "insert")))]
    pub default_connection_url: HashSet<Url>,
    pub reconnect_time: u64,
    #[serde(default = "default_reconnect_base_delay")]
    pub reconnect_base_delay: u64,
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
    #[serde(skip)]
    #[config(extra(iterator(dtype = "Callback<T, Transfer, U>")))]
    pub callbacks: Vec<Callback<T, Transfer, U>>,
    pub connection_initialization_timeout: Duration,
    pub timeout: Duration, // General timeout
    #[serde(bound = "U: Serialize + for<'d> Deserialize<'d>")]
    pub extra: U,
    // #[serde(skip)]
    // pub callbacks: Arc<Vec<Arc<dyn Callback>>>
}

fn default_reconnect_base_delay() -> u64 {
    1
}

fn default_max_reconnect_attempts() -> u32 {
    10
}

impl<T: DataHandler, Transfer: MessageTransfer, U: InnerConfig> _Config<T, Transfer, U> {
    pub fn new(
        initialization_timeout: Duration,
        callbacks: Vec<Callback<T, Transfer, U>>,
        extra: U,
    ) -> Self {
        Self {
            max_allowed_loops: MAX_ALLOWED_LOOPS,
            sleep_interval: SLEEP_INTERVAL,
            default_connection_url: HashSet::new(),
            reconnect_time: RECONNECT_CALLBACK,
            reconnect_base_delay: 1,
            max_reconnect_attempts: 10,
            callbacks,
            timeout: Duration::from_secs(TIMEOUT_TIME),
            connection_initialization_timeout: initialization_timeout,
            extra,
        }
    }
}
