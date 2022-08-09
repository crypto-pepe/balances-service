use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use tracing::error;

use crate::{api::error::ErrorResponse, node_clients::Chain, service::BalancesService};

#[tracing::instrument(skip(service))]
#[get("/balances/{chain}/{address}")]
pub async fn handler(
    path: web::Path<(String, String)>,
    service: web::Data<Box<dyn BalancesService + Send + Sync>>,
) -> Result<impl Responder, impl ResponseError> {
    let (chain, address) = path.into_inner();

    let chain = Chain::try_from(chain)
        .map_err(|e| ErrorResponse::bad_request(10002, e.to_string(), None))?;

    let balance = service.get_balance(chain, address).await.map_err(|e| {
        error!("{}", e);
        ErrorResponse::internal_server_error(20000, e.to_string())
    })?;

    let response = HttpResponse::Ok().json(&balance);

    Result::<HttpResponse, ErrorResponse>::Ok(response)
}
