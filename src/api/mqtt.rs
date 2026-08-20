use crate::{
    domain::mqtt::{MqttConfig, MqttStatus},
    error::AppError,
    state::AppState,
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_config))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/status", get(status))
}

// --- 响应结构体 ---

#[derive(Serialize)]
struct ConfigResponse {
    saved: bool,
    config: Option<SafeMqttConfig>, // 使用安全的配置结构体
}

// 返回给前端的配置，不包含密码
#[derive(Serialize)]
struct SafeMqttConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub client_id: Option<String>,
}

#[derive(Serialize)]
struct LoginResponse {
    success: bool,
    message: String,
}

// --- 请求结构体 ---

#[derive(Deserialize)]
pub struct LoginRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>, // null -> None, "" -> Some("")
    pub client_id: Option<String>,
}

// --- 接口实现 ---

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ConfigResponse>, AppError> {
    let config = state.config_store.load().await?;

    // 转换为 SafeMqttConfig，丢弃密码
    let safe_config = config.map(|c| SafeMqttConfig {
        host: c.host,
        port: c.port,
        username: c.username,
        client_id: c.client_id,
    });

    Ok(Json(ConfigResponse {
        saved: safe_config.is_some(),
        config: safe_config,
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // 核心修复：区分 null (使用存储密码) 和 "" (空密码/匿名登录)
    let final_password = if req.password.is_none() {
        // 前端发送了 null，明确要求使用存储的密码
        if let Some(stored) = state.config_store.load().await? {
            if stored.host == req.host && stored.port == req.port && stored.username == req.username {
                stored.password
            } else {
                String::new() // 防御性编程：如果不匹配，降级为空密码
            }
        } else {
            String::new()
        }
    } else {
        // 前端发送了字符串（包括空字符串），直接使用
        req.password.unwrap_or_default()
    };

    let config = MqttConfig {
        host: req.host,
        port: req.port,
        username: req.username,
        password: final_password,
        client_id: req.client_id,
    };

    if let Err(message) = config.validate() {
        return Ok(Json(LoginResponse {
            success: false,
            message,
        }));
    }

    // 保存配置（会自动加密新密码或原密码）
    state.config_store.save(&config).await?;

    match state.mqtt_manager.connect(config).await {
        Ok(()) => Ok(Json(LoginResponse {
            success: true,
            message: "登录成功".to_string(),
        })),
        Err(message) => Ok(Json(LoginResponse {
            success: false,
            message,
        })),
    }
}

async fn logout(State(state): State<AppState>) -> Json<LoginResponse> {
    state.mqtt_manager.disconnect().await;
    Json(LoginResponse {
        success: true,
        message: "已断开连接".to_string(),
    })
}

async fn status(State(state): State<AppState>) -> Json<MqttStatus> {
    Json(state.mqtt_manager.status())
}