use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use tokio::sync::Mutex;
use tracing::info;

use crate::domain::{ConfigResponse, ConnectionState, LoginRequest, SanitizedConfig, SuccessResponse, StatusResponse};
use crate::error::{AppError, Result};
use crate::mqtt::MqttManager;
use crate::state::AppState;
use crate::storage::MqttStorage;

pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/config", get(get_config))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/status", get(get_status))
}

/// GET /api/mqtt/config - Get saved configuration (sanitized)
async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let storage = state.storage.lock().await;
    
    match storage.load_config().await {
        Ok(Some(config)) => {
            drop(storage);
            (StatusCode::OK, Json(ConfigResponse {
                saved: true,
                config: Some(SanitizedConfig {
                    host: config.host,
                    port: config.port,
                    username: config.username,
                    client_id: config.client_id,
                }),
            })).into_response()
        }
        Ok(None) => (StatusCode::OK, Json(ConfigResponse {
            saved: false,
            config: None,
        })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// POST /api/mqtt/login - Connect to MQTT broker
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    info!("Login request received for host: {}:{}", req.host, req.port);

    // Validate basic parameters
    if req.host.is_empty() {
        return Json(SuccessResponse::new("MQTT 地址不可为空")).into_response();
    }
    if req.port == 0 {
        return Json(SuccessResponse::new("端口号不能为 0")).into_response();
    }

    // Handle password logic
    let mut config = {
        let storage = state.storage.lock().await;
        
        match req.password {
            // null - reuse saved password
            None => {
                match storage.load_config().await {
                    Ok(Some(saved_config)) => {
                        // Verify host/port/username match
                        if saved_config.host != req.host 
                            || saved_config.port != req.port 
                            || saved_config.username != req.username 
                        {
                            // Mismatch - use empty password
                            crate::domain::MqttConfig {
                                host: req.host,
                                port: req.port,
                                username: req.username,
                                password: String::new(),
                                client_id: req.client_id,
                            }
                        } else {
                            // Match - use saved password
                            crate::domain::MqttConfig {
                                host: req.host,
                                port: req.port,
                                username: req.username,
                                password: saved_config.password,
                                client_id: req.client_id.or(saved_config.client_id),
                            }
                        }
                    }
                    _ => {
                        // No saved config - use empty password
                        crate::domain::MqttConfig {
                            host: req.host,
                            port: req.port,
                            username: req.username,
                            password: String::new(),
                            client_id: req.client_id,
                        }
                    }
                }
            }
            // "" or "string" - use provided password (empty string means anonymous)
            Some(pwd) => crate::domain::MqttConfig {
                host: req.host,
                port: req.port,
                username: req.username,
                password: pwd,
                client_id: req.client_id,
            },
        }
    };

    // Generate client ID if not provided
    config.generate_client_id();

    // Validate configuration
    if let Err(e) = config.validate() {
        return Json(SuccessResponse::new(e)).into_response();
    }

    // Save configuration
    let storage = state.storage.lock().await;
    if let Err(e) = storage.save_config(&config).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    drop(storage);

    // Connect to MQTT broker
    let mut mqtt_manager = state.mqtt_manager.lock().await;
    match mqtt_manager.connect(&config).await {
        Ok(()) => {
            drop(mqtt_manager);
            Json(SuccessResponse::new("登录成功")).into_response()
        }
        Err(e) => {
            drop(mqtt_manager);
            // Update status to disconnected on failure
            let _ = state.status_tx.send(StatusResponse {
                state: ConnectionState::Disconnected,
                message: format!("连接失败：{}", e),
                host: None,
                port: None,
            });
            (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))).into_response()
        }
    }
}

/// POST /api/mqtt/logout - Disconnect from MQTT broker
async fn logout(State(state): State<AppState>) -> impl IntoResponse {
    let mut mqtt_manager = state.mqtt_manager.lock().await;
    
    if !mqtt_manager.is_connected() {
        return Json(SuccessResponse::new("当前未连接")).into_response();
    }

    match mqtt_manager.disconnect().await {
        Ok(()) => Json(SuccessResponse::new("已断开连接")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

/// GET /api/mqtt/status - Get current connection status
async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let mqtt_manager = state.mqtt_manager.lock().await;
    Json(mqtt_manager.get_status()).into_response()
}
