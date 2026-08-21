mod api;
mod domain;
mod error;
mod mqtt;
mod state;
mod storage;

use axum::Router;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::create_router;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dji_cloud_mqtt_simulator=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get working directory
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    
    // Setup paths
    let data_dir = cwd.join("data");
    let static_dir = cwd.join("static");

    // Ensure directories exist
    tokio::fs::create_dir_all(&data_dir)
        .await
        .expect("Failed to create data directory");
    tokio::fs::create_dir_all(&static_dir)
        .await
        .expect("Failed to create static directory");

    // Initialize application state
    let app_state = AppState::new(data_dir);

    // Create API router
    let api_router = create_router().nest("/api/mqtt", create_router());

    // Create static file service with SPA fallback
    let serve_dir = ServeDir::new(&static_dir).append_index_html_on_directories(true);

    // Combine routers - API routes take precedence
    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(serve_dir)
        .with_state(app_state);

    // Start server
    let addr = "0.0.0.0:3000";
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");
    
    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
