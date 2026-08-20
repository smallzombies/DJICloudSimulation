pub mod mqtt;

use crate::state::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new().nest("/api/mqtt", mqtt::router())
}