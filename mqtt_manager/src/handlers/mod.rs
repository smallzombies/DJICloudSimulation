//! HTTP request handlers

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::json;
use sqlx::SqlitePool;
use crate::models::{MqttConfigRequest, MqttConfigResponse, ApiResponse, ConnectionStatus};
use crate::services::MqttService;
use crate::db;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub mqtt_service: MqttService,
}

impl AppState {
    pub fn new(db: SqlitePool, mqtt_service: MqttService) -> Self {
        Self {
            db,
            mqtt_service,
        }
    }
}

/// Get the saved MQTT configuration
pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MqttConfigResponse>>, (StatusCode, String)> {
    match db::get_latest_config(&state.db).await {
        Ok(Some(config)) => {
            let response = MqttConfigResponse::from(config);
            Ok(Json(ApiResponse::success(response, "Configuration retrieved successfully")))
        }
        Ok(None) => {
            Ok(Json(ApiResponse::error("No configuration found")))
        }
        Err(e) => {
            tracing::error!("Failed to get config: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ))
        }
    }
}

/// Save MQTT configuration and attempt to connect
pub async fn save_config(
    State(state): State<AppState>,
    Json(req): Json<MqttConfigRequest>,
) -> Result<Json<ApiResponse<MqttConfigResponse>>, (StatusCode, String)> {
    // Validate input
    if req.host.is_empty() {
        return Ok(Json(ApiResponse::error("Host cannot be empty")));
    }
    
    if req.username.is_empty() {
        return Ok(Json(ApiResponse::error("Username cannot be empty")));
    }
    
    if req.password.is_empty() {
        return Ok(Json(ApiResponse::error("Password cannot be empty")));
    }

    // Save to database
    match db::save_config(&state.db, &req).await {
        Ok(config) => {
            // Try to connect to MQTT broker
            match state.mqtt_service.connect(&config).await {
                Ok(_) => {
                    let response = MqttConfigResponse::from(config);
                    Ok(Json(ApiResponse::success(response, "Configuration saved and connected successfully")))
                }
                Err(e) => {
                    // Config saved but connection failed
                    let response = MqttConfigResponse::from(config);
                    tracing::warn!("MQTT connection failed: {}", e);
                    
                    // Return success for config save, but indicate connection issue
                    Ok(Json(ApiResponse {
                        success: true,
                        message: format!("Configuration saved, but connection failed: {}", e),
                        data: Some(response),
                    }))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to save config: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ))
        }
    }
}

/// Get current MQTT connection status
pub async fn get_connection_status(
    State(state): State<AppState>,
) -> Result<Json<ConnectionStatus>, (StatusCode, String)> {
    let mqtt_state = state.mqtt_service.get_state().await;
    
    let config = match db::get_latest_config(&state.db).await {
        Ok(Some(cfg)) => Some(MqttConfigResponse::from(cfg)),
        Ok(None) => None,
        Err(e) => {
            tracing::error!("Failed to get config for status: {}", e);
            None
        }
    };

    let error = if mqtt_state.connected {
        None
    } else {
        mqtt_state.error.clone()
    };

    Ok(Json(ConnectionStatus {
        connected: mqtt_state.connected,
        config,
        error,
    }))
}

/// Test MQTT connection with provided credentials (without saving)
pub async fn test_connection(
    State(state): State<AppState>,
    Json(req): Json<MqttConfigRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, String)> {
    // Validate input
    if req.host.is_empty() {
        return Ok(Json(ApiResponse::error("Host cannot be empty")));
    }
    
    if req.username.is_empty() {
        return Ok(Json(ApiResponse::error("Username cannot be empty")));
    }
    
    if req.password.is_empty() {
        return Ok(Json(ApiResponse::error("Password cannot be empty")));
    }

    // Create a temporary config for testing
    let temp_config = crate::models::MqttConfig {
        id: 0,
        host: req.host,
        port: req.port as i32,
        username: req.username,
        password: req.password,
        created_at: chrono::Utc::now().naive_utc().to_string(),
        updated_at: chrono::Utc::now().naive_utc().to_string(),
    };

    // Try to connect
    match state.mqtt_service.connect(&temp_config).await {
        Ok(_) => {
            // Disconnect after successful test
            state.mqtt_service.disconnect().await;
            Ok(Json(ApiResponse::success(
                json!({"test": "successful"}),
                "Connection test successful"
            )))
        }
        Err(e) => {
            Ok(Json(ApiResponse::error(&format!("Connection test failed: {}", e))))
        }
    }
}
