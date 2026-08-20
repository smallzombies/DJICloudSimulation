//! MQTT Manager - A web application for managing MQTT connections
//!
//! This application provides:
//! - Web UI for configuring MQTT broker credentials
//! - Persistent storage of MQTT configuration
//! - MQTT connection management
//! - RESTful API for integration with other services

mod config;
mod db;
mod handlers;
mod models;
mod services;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::handlers::{AppState, get_config, save_config, get_connection_status};
use crate::services::MqttService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mqtt_manager=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let app_config = config::AppConfig::from_env();
    
    tracing::info!("Starting MQTT Manager on {}:{}", 
        app_config.server.host, app_config.server.port);

    // Initialize database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&app_config.database_path)
        .await?;
    
    db::init_db(&pool).await?;
    tracing::info!("Database initialized at {}", app_config.database_path);

    // Initialize MQTT service
    let mqtt_service = MqttService::new();
    
    // Try to reconnect with saved config if exists
    if let Ok(Some(config)) = db::get_latest_config(&pool).await {
        tracing::info!("Found saved MQTT config, attempting to reconnect...");
        match mqtt_service.connect(&config).await {
            Ok(_) => tracing::info!("Successfully reconnected to MQTT broker"),
            Err(e) => tracing::warn!("Failed to reconnect to MQTT broker: {}", e),
        }
    }

    // Create application state
    let app_state = AppState::new(pool, mqtt_service);

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/config", get(get_config))
        .route("/api/config", post(save_config))
        .route("/api/status", get(get_connection_status))
        // Serve static files (frontend)
        .nest_service("/", ServeDir::new("static"))
        .layer(cors)
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", app_config.server.host, app_config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("Open http://{} in your browser", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
