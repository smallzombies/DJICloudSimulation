use crate::domain::mqtt::MqttConfig;
use super::crypto;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// 专门用于 JSON 持久化的结构体（包含加密后的密码）
#[derive(Serialize, Deserialize)]
struct StoredConfig {
    host: String,
    port: u16,
    username: String,
    encrypted_password: String,
    client_id: Option<String>,
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub async fn load(&self) -> std::io::Result<Option<MqttConfig>> {
        match tokio::fs::read(&self.path).await {
            Ok(bytes) => {
                let stored: StoredConfig = serde_json::from_slice(&bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                // 读取时解密密码
                let password = crypto::decrypt(&stored.encrypted_password)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                Ok(Some(MqttConfig {
                    host: stored.host,
                    port: stored.port,
                    username: stored.username,
                    password,
                    client_id: stored.client_id,
                }))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn save(&self, config: &MqttConfig) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // 保存时加密密码
        let encrypted_password = crypto::encrypt(&config.password)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let stored = StoredConfig {
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            encrypted_password,
            client_id: config.client_id.clone(),
        };

        let bytes = serde_json::to_vec_pretty(&stored)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        tokio::fs::write(&self.path, bytes).await?;

        Ok(())
    }
}