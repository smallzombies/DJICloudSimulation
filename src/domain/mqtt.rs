use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,

    #[serde(default)]
    pub client_id: Option<String>,
}

impl MqttConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.trim().is_empty() {
            return Err("MQTT地址不可为空".to_string());
        }

        if self.port == 0 {
            return Err("MQTT端口不可为0".to_string());
        }

        // 用户名和密码允许为空，移除了原有的非空校验
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MqttState {
    Connected,
    Connecting,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttStatus {
    pub state: MqttState,
    pub message: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
}