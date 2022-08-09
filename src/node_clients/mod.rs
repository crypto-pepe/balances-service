mod config;
pub mod evm;
pub mod waves;

use serde::{Deserialize, Serialize};

use crate::{error::Error as AppError, service::AddressBalancesService};

pub use config::Config;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Waves,
    Evm,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Waves,
    Ethereum,
    BSC,
}

impl From<Chain> for String {
    fn from(v: Chain) -> Self {
        serde_json::to_string(&v).expect("Failed to serialize Chain")
    }
}

impl From<&Chain> for String {
    fn from(v: &Chain) -> Self {
        v.to_owned().into()
    }
}

impl TryFrom<String> for Chain {
    type Error = AppError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "waves" => Ok(Self::Waves),
            "ethereum" => Ok(Self::Ethereum),
            "bsc" => Ok(Self::BSC),
            _ => Err(AppError::UnexpectedChain(value)),
        }
    }
}

pub fn new(config: &Config) -> Result<Box<dyn AddressBalancesService + Send + Sync>, AppError> {
    match config {
        Config::Waves(chain_config) => {
            let client = waves::NodeClient::try_new(
                &chain_config.base_url,
                &chain_config
                    .supported_asset_ids
                    .as_ref()
                    .map(|v| v.as_slice()),
            )?;
            Ok(Box::new(client))
        }
        Config::Ethereum(chain_config) | Config::Bsc(chain_config) => {
            let client = evm::NodeClient::try_new(
                &chain_config.base_url,
                &config.native_token(),
                &chain_config.chain_id,
                &chain_config
                    .supported_asset_ids
                    .as_ref()
                    .map(|v| v.as_slice()),
                &chain_config.multicall_contract_address,
            )?;
            Ok(Box::new(client))
        }
    }
}
