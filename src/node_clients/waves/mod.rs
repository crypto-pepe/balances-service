mod config;

use reqwest::Client;
use std::collections::HashSet;
use std::sync::Arc;

use crate::{
    error::Error as AppError,
    service::{AddressBalancesService, Balance, BalanceAmount, BalanceKind},
};

pub use config::Config;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

static WAVES_ASSET_ID: &str = "WAVES";

mod dtos {
    use serde::Deserialize;

    #[derive(Clone, Debug, Deserialize)]
    pub struct AddressBalanceDetailsResponse {
        pub address: String,
        /// The amount of WAVES that belongs directly to the account (R)
        pub regular: u64,
        /// Regular balance w/o leased out (Lo) = R - Lo
        pub available: u64,
        /// Regular balance w/o leased out w/ leased in (Li) = R - Lo + Li
        pub effective: u64,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct AddressAssetBalance {
        #[serde(rename = "assetId")]
        pub asset_id: String,
        pub balance: u64,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct AddressAssetsBalancesResponse {
        pub address: String,
        pub balances: Vec<AddressAssetBalance>,
    }
}

pub struct NodeClient {
    http_client: Client,
    base_url: String,
    supported_asset_ids: Option<HashSet<String>>,
}

impl NodeClient {
    pub fn try_new(
        base_url: impl AsRef<str>,
        supported_asset_ids: &Option<&[impl AsRef<str>]>,
    ) -> Result<Self, AppError> {
        let http_client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()
            .map_err(|e| Arc::new(e))?;

        Ok(Self {
            http_client: http_client,
            base_url: base_url.as_ref().to_string(),
            supported_asset_ids: supported_asset_ids.map(|supported_asset_ids| {
                supported_asset_ids
                    .to_owned()
                    .iter()
                    .map(|a| a.as_ref().to_string())
                    .collect()
            }),
        })
    }

    pub async fn address_balance_details(
        &self,
        address: impl AsRef<str> + Send,
    ) -> Result<dtos::AddressBalanceDetailsResponse, AppError> {
        let url = format!(
            "{}/addresses/balance/details/{}",
            self.base_url,
            address.as_ref()
        );
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            AppError::Upstream(format!(
                "Failed while fetching address balance details, {}",
                e
            ))
        })?;

        if response.status() == 200 {
            let json = response
                .json::<dtos::AddressBalanceDetailsResponse>()
                .await
                .map_err(|e| AppError::UpstreamResponse(e.to_string()))?;

            Ok(json)
        } else {
            Err(AppError::UpstreamResponse(format!(
                "Failed to get address regular balance: GET {} -> {}",
                url,
                response.status()
            )))
        }
    }

    pub async fn address_assets_balances(
        &self,
        address: impl AsRef<str> + Send,
        asset_ids: &[impl AsRef<str>],
    ) -> Result<Vec<u64>, AppError> {
        let asset_ids = asset_ids
            .iter()
            .map(|id| format!("id={}", id.as_ref()))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!(
            "{}/assets/balance/{}?{}",
            self.base_url,
            address.as_ref(),
            asset_ids
        );
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            AppError::Upstream(format!(
                "Failed while fetching address regular balance, {}",
                e
            ))
        })?;

        if response.status() == 200 {
            let json = response
                .json::<dtos::AddressAssetsBalancesResponse>()
                .await
                .map_err(|e| AppError::UpstreamResponse(e.to_string()))?;

            Ok(json
                .balances
                .iter()
                .map(|asset_balance| asset_balance.balance)
                .collect())
        } else {
            Err(AppError::UpstreamResponse(format!(
                "Failed to get address regular balance: GET {} -> {}",
                url,
                response.status()
            )))
        }
    }
}

#[async_trait::async_trait]
impl AddressBalancesService for NodeClient {
    fn is_asset_supported(&self, asset_id: String) -> bool {
        self.supported_asset_ids
            .as_ref()
            .map_or(false, |supported_asset_ids| {
                supported_asset_ids.contains(&asset_id)
            })
    }

    async fn get_balance(&self, address: String) -> Result<crate::service::Balance, AppError> {
        let balance_details = self.address_balance_details(address).await?;

        let balance = Balance {
            asset_id: WAVES_ASSET_ID.to_string(),
            balances: vec![
                BalanceAmount {
                    kind: BalanceKind::Wallet,
                    amount: balance_details.regular,
                },
                BalanceAmount {
                    kind: BalanceKind::Available,
                    amount: balance_details.available,
                },
                BalanceAmount {
                    kind: BalanceKind::Effective,
                    amount: balance_details.effective,
                },
            ],
        };

        Ok(balance)
    }

    async fn get_assets_balances(
        &self,
        address: String,
        asset_ids: Vec<String>,
    ) -> Result<Vec<crate::service::Balance>, AppError> {
        let balances = self.address_assets_balances(address, &asset_ids).await?;

        let balances = asset_ids
            .iter()
            .enumerate()
            .map(|(idx, asset_id)| Balance::single(asset_id, &BalanceKind::Wallet, balances[idx]))
            .collect();

        Ok(balances)
    }
}
