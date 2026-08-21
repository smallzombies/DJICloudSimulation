use std::sync::Arc;

use tokio::sync::{watch, Mutex};

use crate::domain::{ConnectionState, StatusResponse};
use crate::mqtt::MqttManager;
use crate::storage::MqttStorage;

/// Global application state
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Mutex<MqttStorage>>,
    pub mqtt_manager: Arc<Mutex<MqttManager>>,
    pub status_tx: watch::Sender<StatusResponse>,
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        let storage = Arc::new(Mutex::new(MqttStorage::new(data_dir)));

        // Initialize status channel with disconnected state
        let (status_tx, _status_rx) = watch::channel(StatusResponse {
            state: ConnectionState::Disconnected,
            message: "未连接".to_string(),
            host: None,
            port: None,
        });

        let mqtt_manager = Arc::new(Mutex::new(MqttManager::new(status_tx.clone())));

        Self {
            storage,
            mqtt_manager,
            status_tx,
        }
    }
}
