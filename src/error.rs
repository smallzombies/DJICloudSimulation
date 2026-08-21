use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MQTT error: {0}")]
    Mqtt(#[from] rumqttc::ClientError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Disconnect timeout")]
    DisconnectTimeout,

    #[error("No active connection")]
    NoActiveConnection,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Mqtt(_) => StatusCode::BAD_GATEWAY,
            AppError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Encryption(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ConnectionTimeout => StatusCode::GATEWAY_TIMEOUT,
            AppError::DisconnectTimeout => StatusCode::GATEWAY_TIMEOUT,
            AppError::NoActiveConnection => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({
            "error": self.to_string()
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
