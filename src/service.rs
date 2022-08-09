use serde::Serialize;
use std::collections::HashMap;

use crate::error::Error as AppError;
use crate::node_clients::Chain;

pub struct Service {
    chain_clients: HashMap<Chain, Box<dyn AddressBalancesService + Send + Sync>>,
}

impl Service {
    pub fn new(chain_clients: HashMap<Chain, Box<dyn AddressBalancesService + Send + Sync>>) -> Self {
        Self {
            chain_clients: chain_clients
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceKind {
    /// Common
    Wallet,

    /// Only for WAVES
    Available,
    Effective,
}

#[derive(Clone, Debug, Serialize)]
pub struct BalanceAmount {
    pub kind: BalanceKind,
    pub amount: u64,
}

impl BalanceAmount {
    pub fn new(kind: &BalanceKind, amount: u64) -> Self {
        Self {
            kind: kind.to_owned(),
            amount,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Balance {
    pub asset_id: String,
    pub balances: Vec<BalanceAmount>,
}

impl Balance {
    pub fn single(asset_id: impl AsRef<str>, kind: &BalanceKind, amount: u64) -> Self {
        Self {
            asset_id: asset_id.as_ref().to_string(),
            balances: vec![BalanceAmount::new(kind, amount)],
        }
    }
}

#[async_trait::async_trait]
pub trait BalancesService {
    /// Checks whether provided `asset_ids` are supported by the service
    ///
    /// Returns vector of not supported assets
    fn check_assets_support(
        &self,
        chain: Chain,
        asset_ids: Vec<String>,
    ) -> Result<Vec<String>, AppError>;

    async fn get_balance(&self, chain: Chain, address: String) -> Result<Balance, AppError>;

    async fn get_assets_balances(
        &self,
        chain: Chain,
        address: String,
        asset_ids: Vec<String>,
    ) -> Result<Vec<Balance>, AppError>;
}

#[async_trait::async_trait]
impl BalancesService for Service {
    fn check_assets_support(
        &self,
        chain: Chain,
        asset_ids: Vec<String>,
    ) -> Result<Vec<String>, AppError> {
        let chain_client = self
            .chain_clients
            .get(&chain)
            .ok_or(AppError::NodeClientWasNotProvided(chain.clone().into()))?;

        let not_supported_asset_ids = asset_ids
            .iter()
            .filter(|asset_id| !chain_client.is_asset_supported(asset_id.to_string()))
            .cloned()
            .collect();

        Ok(not_supported_asset_ids)
    }

    async fn get_balance(&self, chain: Chain, address: String) -> Result<Balance, AppError> {
        let chain_client = self
            .chain_clients
            .get(&chain)
            .ok_or(AppError::NodeClientWasNotProvided(chain.clone().into()))?;

        chain_client.get_balance(address).await
    }

    async fn get_assets_balances(
        &self,
        chain: Chain,
        address: String,
        asset_ids: Vec<String>,
    ) -> Result<Vec<Balance>, AppError> {
        let chain_client = self
            .chain_clients
            .get(&chain)
            .ok_or(AppError::NodeClientWasNotProvided(chain.clone().into()))?;

        chain_client.get_assets_balances(address, asset_ids).await
    }
}

#[async_trait::async_trait]
pub trait AddressBalancesService {
    fn is_asset_supported(&self, asset_id: String) -> bool;

    async fn get_balance(&self, address: String) -> Result<Balance, AppError>;

    async fn get_assets_balances(
        &self,
        address: String,
        asset_ids: Vec<String>,
    ) -> Result<Vec<Balance>, AppError>;
}
