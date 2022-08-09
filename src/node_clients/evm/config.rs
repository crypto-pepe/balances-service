use ethabi::ethereum_types::U64;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub chain_id: U64,
    pub base_url: String,
    pub supported_asset_ids: Option<Vec<String>>,
    pub multicall_contract_address: Option<String>,
}
