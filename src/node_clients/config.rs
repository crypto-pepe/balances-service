use serde::{Deserialize, Serialize};

use super::{evm, waves, Chain};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", untagged)]
pub enum Config {
    Ethereum(evm::Config),
    Bsc(evm::Config),
    Waves(waves::Config),
}

impl Config {
    pub fn chain(&self) -> Chain {
        match self {
            Self::Ethereum(_) => Chain::Ethereum,
            Self::Bsc(_) => Chain::BSC,
            Self::Waves(_) => Chain::Waves,
        }
    }

    pub fn native_token(&self) -> String {
        match self {
            Self::Ethereum(_) => "ETHEREUM".to_string(),
            Self::Bsc(_) => "BNB".to_string(),
            Self::Waves(_) => "WAVES".to_string(),
        }
    }
}
