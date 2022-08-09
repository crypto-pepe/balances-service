mod config;

use ethabi::{Function, Param, ParamType, StateMutability, Token};
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::types::transaction::eip2930::AccessList;
use ethers_core::types::{Address, Eip1559TransactionRequest, NameOrAddress, U256, U64};
use ethers_providers::{Http, Middleware, Provider};
use futures::stream::TryStreamExt;
use std::sync::Arc;
use std::{collections::HashSet, str::FromStr};

use crate::{
    error::Error as AppError,
    service::{AddressBalancesService, Balance, BalanceAmount, BalanceKind},
};

pub use config::Config;

static BALANCE_OF_FUNCTION_NAME: &str = "balanceOf";
static AGGREGATE_FUNCTION_NAME: &str = "aggregate";

mod dtos {
    use serde::Deserialize;

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
    provider: ethers_providers::Provider<Http>,
    native_token: String,
    chain_id: U64,
    supported_asset_ids: Option<HashSet<String>>,
    multicall_contract_address: Option<Address>,
}

impl NodeClient {
    pub fn try_new(
        base_url: impl AsRef<str>,
        native_token: impl AsRef<str>,
        chain_id: &U64,
        supported_asset_ids: &Option<&[impl AsRef<str>]>,
        multicall_contract_address: &Option<impl AsRef<str>>,
    ) -> Result<Self, AppError> {
        let provider = Provider::<Http>::try_from(base_url.as_ref())?;

        let multicall_contract_address = match multicall_contract_address {
            Some(multicall_contract_address) => {
                Address::from_str(multicall_contract_address.as_ref()).map(|s| Some(s))
            }
            _ => Ok(None),
        }?;

        Ok(Self {
            provider,
            native_token: native_token.as_ref().to_string(),
            chain_id: chain_id.clone(),
            supported_asset_ids: supported_asset_ids.map(|supported_asset_ids| {
                supported_asset_ids
                    .to_owned()
                    .iter()
                    .map(|a| a.as_ref().to_string())
                    .collect()
            }),
            multicall_contract_address,
        })
    }

    pub async fn address_native_balance(
        &self,
        address: impl Into<NameOrAddress> + Send + Sync,
    ) -> Result<U256, AppError> {
        let balance = self
            .provider
            .get_balance(address, None)
            .await
            .map_err(|e| Arc::new(e))?;

        Ok(balance)
    }

    pub async fn address_assets_balances(
        &self,
        address: Address,
        asset_contract_addresses: &[impl AsRef<str>],
        multicall_contract_address: Option<impl Into<Address>>,
    ) -> Result<Vec<U256>, AppError> {
        let balance_of = Function {
            name: BALANCE_OF_FUNCTION_NAME.to_owned(),
            inputs: vec![Param {
                name: "address".to_owned(),
                kind: ParamType::Address,
                internal_type: None,
            }],
            outputs: vec![Param {
                name: BALANCE_OF_FUNCTION_NAME.to_owned(),
                kind: ParamType::Uint(256),
                internal_type: None,
            }],
            constant: None,
            state_mutability: StateMutability::View,
        };

        let aggregate = Function {
            name: AGGREGATE_FUNCTION_NAME.to_owned(),
            inputs: vec![Param {
                name: "calls".to_owned(),
                kind: ParamType::Array(Box::new(ParamType::Tuple(vec![
                    ParamType::Address,
                    ParamType::Bytes,
                ]))),
                internal_type: None,
            }],
            outputs: vec![
                Param {
                    name: "blockNumber".to_owned(),
                    kind: ParamType::Uint(256),
                    internal_type: None,
                },
                Param {
                    name: "returnData".to_owned(),
                    kind: ParamType::Array(Box::new(ParamType::Bytes)),
                    internal_type: None,
                },
            ],
            constant: None,
            state_mutability: StateMutability::View,
        };

        match multicall_contract_address {
            Some(multicall_contract_address) => {
                let input_tokens = asset_contract_addresses.iter().try_fold(
                    vec![],
                    |mut acc, asset_contract_address| {
                        let call_data = balance_of
                            .encode_input(&[Token::Address(address)])
                            .map_err(|e| Arc::new(e))?;

                        let asset_contract_address =
                            Address::from_str(asset_contract_address.as_ref())?;

                        let token = Token::Tuple(vec![
                            Token::Address(asset_contract_address),
                            Token::Bytes(call_data),
                        ]);

                        acc.push(token);

                        Result::<Vec<Token>, AppError>::Ok(acc)
                    },
                )?;

                let call_data = aggregate
                    .encode_input(&[Token::Array(input_tokens)])
                    .map_err(|e| Arc::new(e))?
                    .into();

                let req = Eip1559TransactionRequest {
                    from: None,
                    to: Some(NameOrAddress::Address(multicall_contract_address.into())),
                    gas: None,
                    value: None,
                    data: Some(call_data),
                    nonce: None,
                    access_list: AccessList(vec![]),
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                    chain_id: Some(self.chain_id),
                };
                let tx = TypedTransaction::Eip1559(req);

                let response = self
                    .provider
                    .call(&tx, None)
                    .await
                    .map_err(|e| AppError::EthersProvider(Arc::new(e)))?;

                let output_tokens = aggregate
                    .decode_output(&response.0)
                    .map_err(|e| Arc::new(e))?;

                let balances_token =
                    output_tokens
                        .get(1)
                        .cloned()
                        .ok_or(AppError::MissingAbiOutputToken(
                            AGGREGATE_FUNCTION_NAME.to_owned(),
                        ))?;

                let balances = match balances_token {
                    Token::Array(results) => {
                        let balances = results
                            .iter()
                            .map(|result_token| match result_token {
                                Token::Bytes(bytes) => balance_of
                                    .decode_output(&bytes)
                                    .map_err(|e| AppError::Ethabi(Arc::new(e)))
                                    .and_then(|result| {
                                        result.get(0).and_then(|t| t.clone().into_uint()).ok_or(
                                            AppError::MissingAbiOutputToken(
                                                BALANCE_OF_FUNCTION_NAME.to_owned(),
                                            ),
                                        )
                                    }),
                                _ => Err(AppError::UnexpectedOutputToken(result_token.to_string())),
                            })
                            .collect::<Result<Vec<U256>, AppError>>()?;
                        Ok(balances)
                    }
                    _ => Err(AppError::UnexpectedOutputToken(balances_token.to_string())),
                }?;

                Ok(balances)
            }
            None => {
                let stream = futures::stream::iter(
                    asset_contract_addresses
                        .iter()
                        .map(|c| Result::<String, AppError>::Ok(c.as_ref().to_owned())),
                );
                let balances = stream
                    .try_fold(vec![], |mut acc, asset_contract_address| {
                        let address = address.clone();
                        let balance_of = balance_of.clone();
                        async move {
                            let asset_contract_address =
                                Address::from_str(asset_contract_address.as_ref())?;

                            let call_data = balance_of
                                .encode_input(&[Token::Address(address)])
                                .map_err(|e| Arc::new(e))?
                                .into();

                            let req = Eip1559TransactionRequest {
                                from: None,
                                to: Some(NameOrAddress::Address(asset_contract_address)),
                                gas: None,
                                value: None,
                                data: Some(call_data),
                                nonce: None,
                                access_list: AccessList(vec![]),
                                max_priority_fee_per_gas: None,
                                max_fee_per_gas: None,
                                chain_id: Some(self.chain_id),
                            };
                            let tx = TypedTransaction::Eip1559(req);

                            let response = self
                                .provider
                                .call(&tx, None)
                                .await
                                .map_err(|e| AppError::EthersProvider(Arc::new(e)))?;

                            let result = balance_of
                                .decode_output(&response.0)
                                .map_err(|e| Arc::new(e))?;

                            let balance = result.get(0).and_then(|t| t.clone().into_uint()).ok_or(
                                AppError::MissingAbiOutputToken(
                                    BALANCE_OF_FUNCTION_NAME.to_owned(),
                                ),
                            )?;

                            acc.push(balance);

                            Ok(acc)
                        }
                    })
                    .await?;

                Ok(balances)
            }
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
        let address = Address::from_str(&address)?;
        let balance = self
            .address_native_balance(NameOrAddress::Address(address))
            .await?;

        let balance = Balance {
            asset_id: self.native_token.clone(),
            balances: vec![BalanceAmount {
                kind: BalanceKind::Wallet,
                amount: balance.as_u64(),
            }],
        };

        Ok(balance)
    }

    async fn get_assets_balances(
        &self,
        address: String,
        asset_ids: Vec<String>,
    ) -> Result<Vec<crate::service::Balance>, AppError> {
        let address = Address::from_str(&address)?;
        let balances = self
            .address_assets_balances(address, &asset_ids, self.multicall_contract_address)
            .await?;

        let balances = asset_ids
            .iter()
            .enumerate()
            .map(|(idx, asset_id)| {
                Balance::single(asset_id, &BalanceKind::Wallet, balances[idx].as_u64())
            })
            .collect();

        Ok(balances)
    }
}
