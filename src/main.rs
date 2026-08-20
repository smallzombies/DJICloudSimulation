mod api;
mod domain;
mod error;
mod mqtt;
mod state;
mod storage;

use state::AppState;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // MQTT 配置保存位置
    let state = AppState::new("data/mqtt_config.json");

    let app = api::router()
        .with_state(state)
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("绑定端口失败");

    tracing::info!("访问前端页面: http://localhost:3000");

    axum::serve(listener, app)
        .await
        .expect("服务启动失败");
}