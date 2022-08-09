use std::sync::Arc;

use tracing::{dispatcher::SetGlobalDefaultError, log::SetLoggerError};

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("LogTracerInit: {0}")]
    LogTracerInit(#[from] Arc<SetLoggerError>),

    #[error("SetGlobalDefault: {0}")]
    SetGlobalDefault(#[from] Arc<SetGlobalDefaultError>),

    #[error("LoadConfig: {0}")]
    LoadConfig(#[from] Arc<pepe_config::ConfigError>),

    #[error("ReqwestBuild: {0}")]
    ReqwestBuild(#[from] Arc<reqwest::Error>),

    #[error("Upstream: {0}")]
    Upstream(String),

    #[error("UpstreamResponse: {0}")]
    UpstreamResponse(String),

    #[error("ApiServerBind: {0}")]
    ApiServerBind(Arc<std::io::Error>),

    #[error("ApiServerRun: {0}")]
    ApiServerRun(Arc<std::io::Error>),

    #[error("UnexpectedChain: {0}")]
    UnexpectedChain(String),

    #[error("NodeClientWasNotProvided: {0}")]
    NodeClientWasNotProvided(String),

    #[error("UrlParse: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("EthersProvider: {0}")]
    EthersProvider(#[from] Arc<ethers_providers::ProviderError>),

    #[error("ParseHex: {0}")]
    ParseHex(#[from] rustc_hex::FromHexError),

    #[error("Ethabi: {0}")]
    Ethabi(#[from] Arc<ethabi::Error>),

    #[error("MissingAbiOutputToken: {0}")]
    MissingAbiOutputToken(String),

    #[error("UnexpectedOutputToken: {0}")]
    UnexpectedOutputToken(String),
}
