use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use crate::domain::MqttConfig;
use crate::error::{AppError, Result};

/// Encrypted password storage format
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedPassword {
    nonce: String, // Base64 encoded
    ciphertext: String, // Base64 encoded
}

/// Storage key - in production, this should be from environment variable
const STORAGE_KEY: [u8; 32] = *b"dji-cloud-mqtt-simulator-key-32!";

pub struct MqttStorage {
    data_dir: PathBuf,
    config_path: PathBuf,
}

impl MqttStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        let config_path = data_dir.join("mqtt_config.json");
        Self {
            data_dir,
            config_path,
        }
    }

    /// Encrypt password using AES-256-GCM
    fn encrypt_password(password: &str) -> Result<String> {
        let cipher = Aes256Gcm::new_from_slice(STORAGE_KEY)
            .map_err(|e| AppError::Encryption(format!("Failed to initialize cipher: {}", e)))?;

        // Generate random 12-byte nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, password.as_bytes())
            .map_err(|e| AppError::Encryption(format!("Encryption failed: {}", e)))?;

        // Encode nonce and ciphertext as base64
        let encrypted = EncryptedPassword {
            nonce: BASE64.encode(nonce_bytes),
            ciphertext: BASE64.encode(ciphertext),
        };

        serde_json::to_string(&encrypted)
            .map_err(|e| AppError::Encryption(format!("Failed to serialize encrypted data: {}", e)))
    }

    /// Decrypt password from stored format
    fn decrypt_password(encrypted_data: &str) -> Result<String> {
        let encrypted: EncryptedPassword = serde_json::from_str(encrypted_data)
            .map_err(|e| AppError::Encryption(format!("Failed to deserialize encrypted data: {}", e)))?;

        // Decode base64
        let nonce_bytes = BASE64.decode(&encrypted.nonce)
            .map_err(|e| AppError::Encryption(format!("Failed to decode nonce: {}", e)))?;
        let ciphertext = BASE64.decode(&encrypted.ciphertext)
            .map_err(|e| AppError::Encryption(format!("Failed to decode ciphertext: {}", e)))?;

        if nonce_bytes.len() != 12 {
            return Err(AppError::Encryption("Invalid nonce length".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(STORAGE_KEY)
            .map_err(|e| AppError::Encryption(format!("Failed to initialize cipher: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| AppError::Encryption(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::Encryption(format!("Invalid UTF-8 in decrypted data: {}", e)))
    }

    /// Save configuration to disk (with encrypted password)
    pub async fn save_config(&self, config: &MqttConfig) -> Result<()> {
        // Ensure data directory exists
        fs::create_dir_all(&self.data_dir).await?;

        // Read existing file to preserve encrypted password if needed
        let mut encrypted_password: Option<String> = None;
        if let Ok(existing_content) = fs::read_to_string(&self.config_path).await {
            if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&existing_content) {
                if let Some(ep) = existing.get("encrypted_password").and_then(|v| v.as_str()) {
                    encrypted_password = Some(ep.to_string());
                }
            }
        }

        // Encrypt the password
        let enc_pwd = if config.password.is_empty() {
            None
        } else {
            Some(Self::encrypt_password(&config.password)?)
        };

        // Create serializable format
        #[derive(Serialize)]
        struct ConfigFile {
            host: String,
            port: u16,
            username: String,
            encrypted_password: Option<String>,
            client_id: Option<String>,
        }

        let config_file = ConfigFile {
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            encrypted_password: enc_pwd.or(encrypted_password),
            client_id: config.client_id.clone(),
        };

        let content = serde_json::to_string_pretty(&config_file)?;
        fs::write(&self.config_path, content).await?;

        Ok(())
    }

    /// Load configuration from disk (with decrypted password)
    pub async fn load_config(&self) -> Result<Option<MqttConfig>> {
        let content = match fs::read_to_string(&self.config_path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        #[derive(Deserialize)]
        struct ConfigFile {
            host: String,
            port: u16,
            username: String,
            encrypted_password: Option<String>,
            client_id: Option<String>,
        }

        let file: ConfigFile = serde_json::from_str(&content)?;

        let password = match &file.encrypted_password {
            Some(enc) => Self::decrypt_password(enc)?,
            None => String::new(),
        };

        Ok(Some(MqttConfig {
            host: file.host,
            port: file.port,
            username: file.username,
            password,
            client_id: file.client_id,
        }))
    }

    /// Check if configuration exists
    pub async fn has_config(&self) -> bool {
        fs::try_exists(&self.config_path).await.unwrap_or(false)
    }
}
