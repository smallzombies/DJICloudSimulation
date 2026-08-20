//! MQTT service for managing MQTT connections

use rumqttc::{AsyncClient, MqttOptions};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::models::MqttConfig;

/// MQTT connection state
#[derive(Debug, Clone)]
pub struct MqttConnectionState {
    pub connected: bool,
    pub error: Option<String>,
}

/// MQTT Service - manages the MQTT client connection
#[derive(Clone)]
pub struct MqttService {
    client: Arc<RwLock<Option<AsyncClient>>>,
    state: Arc<RwLock<MqttConnectionState>>,
}

impl MqttService {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(MqttConnectionState {
                connected: false,
                error: None,
            })),
        }
    }

    /// Connect to MQTT broker with the given configuration
    pub async fn connect(&self, config: &MqttConfig) -> Result<(), String> {
        // Create MQTT options
        let mut mqtt_options = MqttOptions::new(
            format!("mqtt_manager_{}", uuid::Uuid::new_v4()),
            &config.host,
            config.port as u16,
        );

        mqtt_options.set_credentials(&config.username, &config.password);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(60));

        // Create client and event loop
        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

        // Spawn task to handle events
        let state_clone = self.state.clone();
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(_) => {
                        // Connection successful, update state
                        let mut state = state_clone.write().await;
                        state.connected = true;
                        state.error = None;
                    }
                    Err(e) => {
                        // Connection failed or lost
                        let mut state = state_clone.write().await;
                        state.connected = false;
                        state.error = Some(e.to_string());
                        
                        // Log the error
                        tracing::error!("MQTT event loop error: {}", e);
                        
                        // Wait before retrying
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        // Store the client
        *self.client.write().await = Some(client);

        // Wait a bit to check initial connection
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Check if connection was successful
        let state = self.state.read().await;
        if state.connected {
            Ok(())
        } else {
            Err(state.error.clone().unwrap_or_else(|| "Unknown connection error".to_string()))
        }
    }

    /// Disconnect from MQTT broker
    pub async fn disconnect(&self) {
        let mut client_guard = self.client.write().await;
        if let Some(client) = client_guard.take() {
            let _ = client.disconnect().await;
        }
        
        let mut state = self.state.write().await;
        state.connected = false;
        state.error = None;
    }

    /// Get current connection state
    pub async fn get_state(&self) -> MqttConnectionState {
        self.state.read().await.clone()
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        self.state.read().await.connected
    }

    /// Get the client (for publishing/subscribing in other services)
    pub async fn get_client(&self) -> Option<AsyncClient> {
        self.client.read().await.clone()
    }
}

impl Default for MqttService {
    fn default() -> Self {
        Self::new()
    }
}
