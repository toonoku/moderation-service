use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("validation error")]
    Validation(String),
    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("regex compile error: {0}")]
    Regex(String),
    #[error("not found")]
    NotFound,
    #[error("internal error")]
    Internal,
    #[error("unauthorized")]
    Unauthorized,
}

#[derive(Serialize)]
struct ErrorBody {
    success: bool,
    message: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Error::Validation(m) => (StatusCode::BAD_REQUEST, m.to_string()),
            Error::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Error::Regex(m) => (StatusCode::BAD_REQUEST, m.to_string()),
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal".to_string()),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
        };
        (
            status,
            Json(ErrorBody {
                success: false,
                message: msg,
            }),
        )
            .into_response()
    }
}
