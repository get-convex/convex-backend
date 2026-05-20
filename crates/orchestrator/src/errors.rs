use axum::{
    http::StatusCode,
    response::{
        IntoResponse,
        Response,
    },
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("host capacity exceeded: need {needed_mb} MB, {free_mb} MB free")]
    HostCapacityExceeded { needed_mb: u64, free_mb: u64 },
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NotFound",
            Self::Unauthorized => "Unauthorized",
            Self::Forbidden => "Forbidden",
            Self::Conflict(_) => "Conflict",
            Self::HostCapacityExceeded { .. } => "host_capacity_exceeded",
            Self::BadRequest(_) => "BadRequest",
            Self::NotImplemented(_) => "NotImplemented",
            Self::Internal(_) => "InternalServerError",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::HostCapacityExceeded { .. } => StatusCode::CONFLICT,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if matches!(self, Self::Internal(_)) {
            tracing::error!(error = %self, "internal server error");
        }
        // HostCapacityExceeded gets a richer body with numeric fields.
        if let Self::HostCapacityExceeded { needed_mb, free_mb } = self {
            let body = json!({
                "code": "host_capacity_exceeded",
                "neededMb": needed_mb,
                "freeMb": free_mb,
            });
            return (StatusCode::CONFLICT, Json(body)).into_response();
        }
        let status = self.status();
        let code = self.code();
        let message = self.to_string();
        let body = json!({
            "code": code,
            "message": message,
        });
        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
