use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Cryptography error: {0}")]
    Crypto(#[from] crate::crypto::CryptoError),

    #[error("Cloudflare API request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Cloudflare API error: {0}")]
    CloudflareApi(String),

    #[error("Cloudflare API token verification failed or token inactive")]
    InvalidToken,

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Zone not found or no associated account: {0}")]
    ZoneNotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    status: u16,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(sqlx::Error::RowNotFound) => {
                (StatusCode::NOT_FOUND, "Resource not found in database".to_string())
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred".to_string())
            }
            AppError::Crypto(e) => {
                tracing::error!("Crypto error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Cryptographic operation failed".to_string())
            }
            AppError::Reqwest(e) => {
                tracing::error!("HTTP client error: {:?}", e);
                (StatusCode::BAD_GATEWAY, format!("Failed to communicate with Cloudflare: {}", e))
            }
            AppError::CloudflareApi(msg) => {
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            AppError::InvalidToken => {
                (StatusCode::BAD_REQUEST, "Cloudflare API token verification failed or token is inactive".to_string())
            }
            AppError::AccountNotFound(id) => {
                (StatusCode::NOT_FOUND, format!("Account not found: {}", id))
            }
            AppError::ZoneNotFound(id) => {
                (StatusCode::NOT_FOUND, format!("Zone not found or no associated account: {}", id))
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };

        let body = Json(ErrorResponse {
            error: message,
            status: status.as_u16(),
        });

        (status, body).into_response()
    }
}
