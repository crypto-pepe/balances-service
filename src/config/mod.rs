use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use serde::de::{self};
use serde::{Deserialize, Serialize};

use crate::api::config::Config as ApiConfig;
use crate::error::Error as AppError;
use crate::node_clients::{
    evm::Config as EvmConfig, waves::Config as WavesConfig, Chain, Config as ChainConfig,
};

const DEFAULT_CONFIG: &str = include_str!("../../config.yaml");

type ConfigMap = HashMap<Chain, ChainConfig>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub api: ApiConfig,
    #[serde(deserialize_with = "deserialize_config_map")]
    pub chains: ConfigMap,
}

pub fn load() -> Result<Config, AppError> {
    pepe_config::load::<Config>(DEFAULT_CONFIG, pepe_config::FileFormat::Yaml)
        .map_err(|e| Arc::new(e).into())
}

fn deserialize_config_map<'de, D>(deserializer: D) -> Result<ConfigMap, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct ConfigMapVisitor {
        marker: PhantomData<fn() -> ConfigMap>,
    }

    impl ConfigMapVisitor {
        fn new() -> Self {
            Self {
                marker: PhantomData,
            }
        }
    }

    impl<'de> de::Visitor<'de> for ConfigMapVisitor {
        // The type that our Visitor is going to produce.
        type Value = ConfigMap;

        // Format a message stating what data this Visitor expects to receive.
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a very special map")
        }

        // Deserialize ConfigMap from an abstract "map" provided by the
        // Deserializer. The MapAccess input is a callback provided by
        // the Deserializer to let us see each entry in the map.
        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            let mut map = ConfigMap::with_capacity(access.size_hint().unwrap_or(0));

            // While there are entries remaining in the input, add them
            // into our map.
            while let Some(key) = access.next_key()? {
                let value = match key {
                    Chain::Ethereum => {
                        let c = access.next_value::<EvmConfig>()?;
                        ChainConfig::Ethereum(c)
                    }
                    Chain::BSC => {
                        let c = access.next_value::<EvmConfig>()?;
                        ChainConfig::Bsc(c)
                    }
                    Chain::Waves => {
                        let c = access.next_value::<WavesConfig>()?;
                        ChainConfig::Waves(c)
                    }
                };
                map.insert(key, value);
            }

            Ok(map)
        }
    }

    deserializer.deserialize_any(ConfigMapVisitor::new())
}
