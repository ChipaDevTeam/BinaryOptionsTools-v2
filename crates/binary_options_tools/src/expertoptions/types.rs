use std::collections::HashMap;

use crate::utils::serialize::bool2int;
use binary_options_tools_core_pre::traits::Rule;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct Asset {
    pub id: u32,
    pub symbol: String,
    pub name: String,
    #[serde(with = "bool2int")]
    pub is_active: bool,
    #[serde(flatten)]
    _extra: HashMap<String, Value>
}

pub struct Assets(pub HashMap<String, Asset>);

pub struct MultiRule {
    rules: Vec<Box<dyn Rule + Send + Sync>>,
}

impl MultiRule {
    pub fn new(rules: Vec<Box<dyn Rule + Send + Sync>>) -> Self {
        Self { rules }
    }
}

impl Rule for MultiRule {
    fn call(&self, msg: &binary_options_tools_core_pre::reimports::Message) -> bool {
        for rule in &self.rules {
            if rule.call(msg) {
                return true;
            }
        }
        false
    }

    fn reset(&self) {
        for rule in &self.rules {
            rule.reset();
        }
    }
}

impl Asset {
    fn is_valid(&self) -> bool {
        !self.symbol.is_empty() && self.id > 0 && self.id != 20000 // Id of asset nos supported by client
    }
}

impl Assets {
    pub fn new(assets: Vec<Asset>) -> Self {
        Assets(HashMap::from_iter(assets.into_iter().filter(|asset| asset.is_valid()).map(|a| (a.symbol.clone(), a))))
    }

    pub fn id(&self, asset: &str) -> Option<u32> {
        self.0.get(asset).map(|a| a.id)
    }
}