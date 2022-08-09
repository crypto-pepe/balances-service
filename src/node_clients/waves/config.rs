use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub chain_id: u16,
    pub base_url: String,
    pub supported_asset_ids: Option<Vec<String>>,
}
