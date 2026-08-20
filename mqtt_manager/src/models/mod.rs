//! Data models

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// MQTT connection configuration
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MqttConfig {
    pub id: i32,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Request model for MQTT config (without ID and timestamps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfigRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

/// Response model for MQTT config (excludes password for security)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfigResponse {
    pub id: i32,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MqttConfig> for MqttConfigResponse {
    fn from(config: MqttConfig) -> Self {
        Self {
            id: config.id,
            host: config.host,
            port: config.port as u16,
            username: config.username,
            created_at: config.created_at.clone(),
            updated_at: config.updated_at.clone(),
        }
    }
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self 
    where
        T: Clone,
    {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub config: Option<MqttConfigResponse>,
    pub error: Option<String>,
}
