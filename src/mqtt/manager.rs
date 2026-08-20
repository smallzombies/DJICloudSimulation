use crate::domain::mqtt::{MqttConfig, MqttState, MqttStatus};
use rumqttc::{AsyncClient, ConnectReturnCode, Event, MqttOptions, Packet};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{watch, Mutex},
    task::JoinHandle,
};

#[derive(Default)]
struct MqttRuntime {
    client: Option<AsyncClient>,
    task: Option<JoinHandle<()>>,
}

pub struct MqttManager {
    connect_lock: Mutex<()>,
    runtime: Mutex<MqttRuntime>,
    status_tx: watch::Sender<MqttStatus>,
    manual_disconnect: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
}

impl MqttManager {
    pub fn new() -> Self {
        let initial = MqttStatus {
            state: MqttState::Disconnected,
            message: Some("未连接".to_string()),
            host: None,
            port: None,
        };

        let (status_tx, _) = watch::channel(initial);

        Self {
            connect_lock: Mutex::new(()),
            runtime: Mutex::new(MqttRuntime::default()),
            status_tx,
            manual_disconnect: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn status(&self) -> MqttStatus {
        self.status_tx.borrow().clone()
    }

    pub async fn connect(&self, config: MqttConfig) -> Result<(), String> {
        let _guard = self.connect_lock.lock().await;

        if let Err(err) = config.validate() {
            return Err(err);
        }

        // 停止旧连接
        self.stop(true).await;

        // 每次连接生成新的 generation，避免旧连接事件影响新连接
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.manual_disconnect.store(false, Ordering::SeqCst);

        let client_id = config
            .client_id
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(generate_client_id);

        let mut options = MqttOptions::new(client_id, config.host.clone(), config.port);

        // 仅当用户名不为空时，才设置账号密码（支持匿名连接）
        if !config.username.trim().is_empty() {
            options.set_credentials(config.username.clone(), config.password.clone());
        }
        options.set_keep_alive(Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(options, 10);

        self.set_status(
            MqttState::Connecting,
            Some(format!("正在连接 {}:{}", config.host, config.port)),
            Some(&config),
        );

        let status_tx = self.status_tx.clone();
        let generation = self.generation.clone();
        let manual_disconnect = self.manual_disconnect.clone();
        let event_config = config.clone();

        let handle = tokio::spawn(async move {
            let send_status = |status: MqttStatus| {
                if generation.load(Ordering::SeqCst) == my_gen {
                    let _ = status_tx.send(status);
                }
            };

            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                        if matches!(connack.code, ConnectReturnCode::Success) {
                            send_status(make_status(
                                MqttState::Connected,
                                "已连接".to_string(),
                                &event_config,
                            ));
                        } else {
                            send_status(make_status(
                                MqttState::Disconnected,
                                format!("连接被拒绝: {:?}", connack.code),
                                &event_config,
                            ));
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        let message = if manual_disconnect.load(Ordering::SeqCst) {
                            "已断开连接".to_string()
                        } else {
                            format!("连接异常: {err}")
                        };

                        send_status(make_status(
                            MqttState::Disconnected,
                            message,
                            &event_config,
                        ));

                        break;
                    }
                }
            }
        });

        {
            let mut runtime = self.runtime.lock().await;
            runtime.client = Some(client);
            runtime.task = Some(handle);
        }

        match self.wait_connected(Duration::from_secs(8)).await {
            Ok(()) => Ok(()),
            Err(err) => {
                self.stop(true).await;
                self.set_status(MqttState::Disconnected, Some(err.clone()), Some(&config));
                Err(err)
            }
        }
    }

    pub async fn disconnect(&self) {
        self.stop(true).await;

        self.set_status(
            MqttState::Disconnected,
            Some("已断开连接".to_string()),
            None,
        );
    }

    async fn stop(&self, manual: bool) {
        if manual {
            self.manual_disconnect.store(true, Ordering::SeqCst);
        }

        let mut runtime = self.runtime.lock().await;

        if let Some(client) = runtime.client.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), client.disconnect()).await;
        }

        if let Some(task) = runtime.task.take() {
            task.abort();
        }
    }

    fn set_status(&self, state: MqttState, message: Option<String>, config: Option<&MqttConfig>) {
        let (host, port) = match config {
            Some(config) => (Some(config.host.clone()), Some(config.port)),
            None => (None, None),
        };

        let _ = self.status_tx.send(MqttStatus {
            state,
            message,
            host,
            port,
        });
    }

    async fn wait_connected(&self, timeout: Duration) -> Result<(), String> {
        let mut rx = self.status_tx.subscribe();
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let current = rx.borrow().clone();

            match current.state {
                MqttState::Connected => return Ok(()),
                MqttState::Disconnected => {
                    return Err(current.message.unwrap_or_else(|| "连接失败".to_string()))
                }
                MqttState::Connecting => {}
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());

            if remaining.is_zero() {
                return Err("连接超时".to_string());
            }

            match tokio::time::timeout(remaining, rx.changed()).await {
                Ok(Ok(())) => continue,
                Ok(Err(_)) => return Err("状态通道已关闭".to_string()),
                Err(_) => return Err("连接超时".to_string()),
            }
        }
    }
}

fn make_status(state: MqttState, message: String, config: &MqttConfig) -> MqttStatus {
    MqttStatus {
        state,
        message: Some(message),
        host: Some(config.host.clone()),
        port: Some(config.port),
    }
}

fn generate_client_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    format!("mqtt-web-{}-{ts}", std::process::id())
}