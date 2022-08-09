mod api;
mod config;
pub mod error;
pub mod node_clients;
mod service;
mod tracing;

use ::tracing::info;

use crate::error::Error as AppError;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing::init_tracing()?;

    let config = config::load()?;
    info!("config loaded: {:?}", &config);

    let node_clients = config
        .chains
        .into_iter()
        .map(|(chain, config)| node_clients::new(&config).map(|node_client| (chain, node_client)))
        .collect::<Result<_, AppError>>()?;

    let service = service::Service::new(node_clients);

    let api = api::server::Server::try_new(&config.api, Box::new(service))?;

    api.run().await?;

    Ok(())
}
