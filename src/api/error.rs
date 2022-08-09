use actix_web::{body::BoxBody, HttpResponse, ResponseError};
use reqwest::StatusCode;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    #[serde(skip_serializing)]
    pub status_code: StatusCode,
    pub code: u16,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, String>>, // field name -> description,
}

impl ErrorResponse {
    pub fn bad_request(
        code: u16,
        reason: impl AsRef<str>,
        details: Option<BTreeMap<String, String>>,
    ) -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
            code: code,
            reason: reason.as_ref().to_string(),
            details: details,
        }
    }

    pub fn internal_server_error(code: u16, reason: impl AsRef<str>) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            code: code,
            reason: reason.as_ref().to_string(),
            details: None,
        }
    }
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| std::fmt::Error)?
                .as_str(),
        )
    }
}

impl ResponseError for ErrorResponse {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code()).json(self)
    }
}
