use std::collections::BTreeMap;

use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use serde::Deserialize;
use tracing::error;

use crate::{api::error::ErrorResponse, node_clients::Chain, service::BalancesService};

#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(rename = "id")]
    pub ids: Vec<String>,
}

#[tracing::instrument(skip(service))]
#[get("/balances/{chain}/{address}/assets")]
pub async fn handler(
    path: web::Path<(String, String)>,
    request: serde_qs::actix::QsQuery<Request>,
    service: web::Data<Box<dyn BalancesService + Send + Sync>>,
) -> Result<impl Responder, impl ResponseError> {
    let (chain, address) = path.into_inner();

    let chain = Chain::try_from(chain)
        .map_err(|e| ErrorResponse::bad_request(10002, e.to_string(), None))?;

    let not_supported_assets = service
        .check_assets_support(chain.clone(), request.ids.clone())
        .map_err(|e| {
            error!("{}", e);
            ErrorResponse::internal_server_error(20000, e.to_string())
        })?;

    if not_supported_assets.len() > 0 {
        let details = BTreeMap::from([(
            "not_supported_assets".to_string(),
            not_supported_assets.join(","),
        )]);
        return Err(ErrorResponse::bad_request(
            20001,
            "Requests contains not supported assets",
            Some(details),
        ));
    }

    let balance = service
        .get_assets_balances(chain, address, request.ids.clone())
        .await
        .map_err(|e| {
            error!("{}", e);
            ErrorResponse::internal_server_error(20000, e.to_string())
        })?;

    let response = HttpResponse::Ok().json(&balance);

    Ok(response)
}
