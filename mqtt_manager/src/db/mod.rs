//! Database operations

use sqlx::{SqlitePool, Row};
use crate::models::{MqttConfig, MqttConfigRequest};
use chrono::Utc;

/// Initialize the database and create tables if they don't exist
pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mqtt_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get the latest MQTT config (most recently updated)
pub async fn get_latest_config(pool: &SqlitePool) -> Result<Option<MqttConfig>, sqlx::Error> {
    let config = sqlx::query_as::<_, MqttConfig>(
        "SELECT * FROM mqtt_configs ORDER BY updated_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    Ok(config)
}

/// Save or update MQTT config
/// If a config exists, it will be updated. Otherwise, a new one is inserted.
pub async fn save_config(
    pool: &SqlitePool,
    req: &MqttConfigRequest,
) -> Result<MqttConfig, sqlx::Error> {
    let existing = get_latest_config(pool).await?;

    let config = match existing {
        Some(_) => {
            // Update existing config
            sqlx::query_as::<_, MqttConfig>(
                r#"
                UPDATE mqtt_configs 
                SET host = ?, port = ?, username = ?, password = ?, updated_at = ?
                WHERE id = (SELECT id FROM mqtt_configs ORDER BY updated_at DESC LIMIT 1)
                RETURNING *
                "#,
            )
            .bind(&req.host)
            .bind(req.port as i32)
            .bind(&req.username)
            .bind(&req.password)
            .bind(Utc::now().naive_utc())
            .fetch_one(pool)
            .await?
        }
        None => {
            // Insert new config
            sqlx::query_as::<_, MqttConfig>(
                r#"
                INSERT INTO mqtt_configs (host, port, username, password, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                RETURNING *
                "#,
            )
            .bind(&req.host)
            .bind(req.port as i32)
            .bind(&req.username)
            .bind(&req.password)
            .bind(Utc::now().naive_utc())
            .bind(Utc::now().naive_utc())
            .fetch_one(pool)
            .await?
        }
    };

    Ok(config)
}
