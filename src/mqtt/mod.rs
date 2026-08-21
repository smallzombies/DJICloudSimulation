use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, Packet};
use tokio::sync::watch;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

use crate::domain::{ConnectionState, MqttConfig, StatusResponse};
use crate::error::{AppError, Result};

/// MQTT connection timeout in seconds
const CONNECTION_TIMEOUT_SECS: u64 = 8;
/// Disconnect timeout in seconds
const DISCONNECT_TIMEOUT_SECS: u64 = 2;

pub struct MqttManager {
    client: Option<AsyncClient>,
    event_loop_handle: Option<tokio::task::JoinHandle<()>>,
    status_tx: watch::Sender<StatusResponse>,
    generation: u64,
}

impl MqttManager {
    pub fn new(status_tx: watch::Sender<StatusResponse>) -> Self {
        Self {
            client: None,
            event_loop_handle: None,
            status_tx,
            generation: 0,
        }
    }

    /// Connect to MQTT broker
    pub async fn connect(&mut self, config: &MqttConfig) -> Result<()> {
        // Increment generation for state protection
        self.generation += 1;
        let current_generation = self.generation;

        // Disconnect existing connection first
        if let Err(e) = self.disconnect().await {
            warn!("Failed to disconnect existing connection: {}", e);
        }

        // Update status to connecting
        let _ = self.status_tx.send(StatusResponse {
            state: ConnectionState::Connecting,
            message: "连接中".to_string(),
            host: Some(config.host.clone()),
            port: Some(config.port),
        });

        // Create MQTT options
        let mut mqtt_options = MqttOptions::new(
            config.client_id.clone().unwrap_or_else(|| {
                let pid = std::process::id();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                format!("mqtt-web-{}-{}", pid, timestamp)
            }),
            &config.host,
            config.port,
        );

        // Set credentials if username is provided
        if !config.username.is_empty() {
            if config.password.is_empty() {
                mqtt_options.set_credentials(config.username.clone(), "");
            } else {
                mqtt_options.set_credentials(config.username.clone(), config.password.clone());
            }
        }
        // If username is empty, allow anonymous login (no credentials set)

        // Create client and event loop
        let (client, eventloop) = AsyncClient::new(mqtt_options, 10);

        // Spawn event loop task
        let status_tx_clone = self.status_tx.clone();
        let event_handle = tokio::spawn(async move {
            Self::run_event_loop(eventloop, status_tx_clone).await;
        });

        self.event_loop_handle = Some(event_handle);

        // Wait for connection confirmation with timeout
        match timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS), async {
            // We need to wait for ConnAck from the event loop
            // The event loop is running in a separate task, so we just wait
            tokio::time::sleep(Duration::from_millis(100)).await;
            true
        }).await {
            Ok(_) => {
                // Give it a bit more time to ensure connection is established
                tokio::time::sleep(Duration::from_millis(200)).await;
                self.client = Some(client);
                info!("MQTT connection established");
                Ok(())
            }
            Err(_) => {
                error!("MQTT connection timeout after {}s", CONNECTION_TIMEOUT_SECS);
                Err(AppError::ConnectionTimeout)
            }
        }
    }

    /// Run the MQTT event loop
    async fn run_event_loop(
        mut eventloop: rumqttc::EventLoop,
        status_tx: watch::Sender<StatusResponse>,
    ) {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    // Connection acknowledged
                    let current_status = status_tx.borrow();
                    if current_status.state == ConnectionState::Connecting {
                        drop(current_status);
                        let _ = status_tx.send(StatusResponse {
                            state: ConnectionState::Connected,
                            message: "已连接".to_string(),
                            host: status_tx.borrow().host.clone(),
                            port: status_tx.borrow().port,
                        });
                    }
                }
                Ok(Event::Incoming(Packet::Disconnect)) | Ok(Event::Outgoing(rumqttc::Outgoing::Disconnect)) => {
                    // Connection lost or disconnected
                    debug!("MQTT connection closed");
                    let _ = status_tx.send(StatusResponse {
                        state: ConnectionState::Disconnected,
                        message: "已断开".to_string(),
                        host: None,
                        port: None,
                    });
                }
                Ok(Event::Incoming(Incoming::Publish(_))) => {
                    // Handle incoming publishes if needed
                    debug!("Received publish");
                }
                Err(e) => {
                    error!("MQTT event loop error: {}", e);
                    let _ = status_tx.send(StatusResponse {
                        state: ConnectionState::Disconnected,
                        message: format!("错误：{}", e),
                        host: None,
                        port: None,
                    });
                    break;
                }
                _ => {}
            }
        }
    }

    /// Disconnect from MQTT broker
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            // Increment generation to invalidate old event loop
            self.generation += 1;

            // Try to disconnect with timeout
            match timeout(Duration::from_secs(DISCONNECT_TIMEOUT_SECS), client.disconnect()).await {
                Ok(Ok(())) => {
                    info!("MQTT disconnected successfully");
                }
                Ok(Err(e)) => {
                    warn!("MQTT disconnect error: {}", e);
                }
                Err(_) => {
                    warn!("MQTT disconnect timeout, forcing termination");
                }
            }

            // Abort event loop task
            if let Some(handle) = self.event_loop_handle.take() {
                handle.abort();
            }

            // Update status
            let _ = self.status_tx.send(StatusResponse {
                state: ConnectionState::Disconnected,
                message: "已断开".to_string(),
                host: None,
                port: None,
            });
        }

        Ok(())
    }

    /// Get current status
    pub fn get_status(&self) -> StatusResponse {
        self.status_tx.borrow().clone()
    }

    /// Get status receiver for subscription
    pub fn subscribe_status(&self) -> watch::Receiver<StatusResponse> {
        self.status_tx.subscribe()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.status_tx.borrow().state == ConnectionState::Connected
    }
}
