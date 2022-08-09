use std::sync::Arc;

use actix_web::{
    error,
    middleware::Logger,
    web::{self, Data},
    App, HttpServer, ResponseError,
};
use reqwest::StatusCode;
use tracing::info;
use tracing_actix_web::TracingLogger;

use super::{config::Config, error::ErrorResponse, routes};
use crate::{error::Error as AppError, service::BalancesService};

pub struct Server {
    pub server: actix_server::Server,
}

impl Server {
    pub fn try_new(
        cfg: &Config,
        service: Box<dyn BalancesService + Send + Sync>,
    ) -> Result<Server, AppError> {
        let service = Data::new(service);

        let srv = HttpServer::new(move || {
            App::new()
                .configure(server_config())
                .app_data(service.clone())
                .wrap(Logger::default())
                .wrap(TracingLogger::default())
        });

        let server = srv
            .bind((cfg.host.clone(), cfg.port))
            .map_err(|e| AppError::ApiServerBind(Arc::new(e)))?
            .run();

        info!("API Server listens {}:{}", cfg.host, cfg.port);

        Ok(Server { server })
    }

    pub async fn run(self) -> Result<(), AppError> {
        self.server
            .await
            .map_err(|e| AppError::ApiServerRun(Arc::new(e).into()))
    }
}

fn server_config() -> Box<dyn Fn(&mut web::ServiceConfig)> {
    Box::new(move |cfg| {
        let json_cfg = web::JsonConfig::default()
            .content_type(|mime| mime == mime::APPLICATION_JSON)
            .error_handler(|err, _| {
                let reason = err.to_string();
                error::InternalError::from_response(
                    err,
                    ErrorResponse {
                        status_code: StatusCode::BAD_REQUEST,
                        code: 10000,
                        reason,
                        details: None,
                    }
                    .error_response(),
                )
                .into()
            });

        let query_cfg = web::QueryConfig::default().error_handler(|err, _| {
            let reason = err.to_string();
            error::InternalError::from_response(
                err,
                ErrorResponse {
                    status_code: StatusCode::BAD_REQUEST,
                    code: 10100,
                    reason,
                    details: None,
                }
                .error_response(),
            )
            .into()
        });

        cfg.app_data(json_cfg)
            .app_data(query_cfg)
            .service(routes::custom_asset_balances::handler)
            .service(routes::native_asset_balances::handler);
    })
}
