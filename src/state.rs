use crate::{mqtt::MqttManager, storage::ConfigStore};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub config_store: Arc<ConfigStore>,
    pub mqtt_manager: Arc<MqttManager>,
}

impl AppState {
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_store: Arc::new(ConfigStore::new(config_path)),
            mqtt_manager: Arc::new(MqttManager::new()),
        }
    }
}