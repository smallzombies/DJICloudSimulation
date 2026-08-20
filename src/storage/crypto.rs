use aes_gcm::{
    aead::{Aead, KeyInit, OsRng, generic_array::GenericArray},
    Aes256Gcm,
};
use aes_gcm::aead::consts::U12;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;

// 32字节的固定密钥（实际生产中建议从环境变量读取）
const KEY: &[u8; 32] = b"DJI_CLOUD_SIMULATION_SECRET_KEY!";

pub fn encrypt(password: &str) -> Result<String, String> {
    if password.is_empty() {
        return Ok(String::new());
    }
    let cipher = Aes256Gcm::new(GenericArray::from_slice(KEY));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = GenericArray::<u8, U12>::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, password.as_bytes()).map_err(|e| e.to_string())?;

    // 将 nonce 和密文拼接后 Base64 编码存储
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(combined))
}

pub fn decrypt(encrypted: &str) -> Result<String, String> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }
    let combined = BASE64.decode(encrypted).map_err(|e| e.to_string())?;
    if combined.len() < 12 {
        return Err("Invalid encrypted data".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = GenericArray::<u8, U12>::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(GenericArray::from_slice(KEY));
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())?;

    String::from_utf8(plaintext).map_err(|e| e.to_string())
}