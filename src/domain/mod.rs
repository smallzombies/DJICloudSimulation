use serde::{Deserialize, Serialize};

/// MQTT configuration domain model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub password: String, // Plaintext in memory only
    pub client_id: Option<String>,
}

impl MqttConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("MQTT 地址不可为空".to_string());
        }
        if self.port == 0 {
            return Err("端口号不能为 0".to_string());
        }
        Ok(())
    }

    /// Generate a unique client ID if not provided
    pub fn generate_client_id(&mut self) {
        if self.client_id.is_none() || self.client_id.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            let pid = std::process::id();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            self.client_id = Some(format!("mqtt-web-{}-{}", pid, timestamp));
        }
    }
}

/// Request model for login API
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>, // null means reuse saved password
    pub client_id: Option<String>,
}

/// Response model for config API (sanitized - no password)
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub saved: bool,
    pub config: Option<SanitizedConfig>,
}

#[derive(Debug, Serialize)]
pub struct SanitizedConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub client_id: Option<String>,
}

/// Response model for status API
#[derive(Debug, Serialize, Clone)]
pub struct StatusResponse {
    pub state: ConnectionState,
    pub message: String,
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connected,
    Connecting,
    Disconnected,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Connected => write!(f, "已连接"),
            ConnectionState::Connecting => write!(f, "连接中"),
            ConnectionState::Disconnected => write!(f, "已断开"),
        }
    }
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
}
