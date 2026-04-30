use axum::{routing::get, Json, Router};
use serde_json::json;

use crate::state::AppState;

pub mod admin;
pub mod analytics;
pub mod apps;
pub mod auth;
pub mod db_browser;
pub mod oauth;
pub mod sync;
pub mod tester;
pub mod tokens;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1/apps", apps::router())
        .nest("/api/v1/apps", db_browser::router())
        .nest("/api/v1/apps", tokens::router())
        .nest("/api/v1/sync", sync::router())
        .nest("/api/v1/admin", admin::router())
        .nest("/api/v1/analytics", analytics::router())
        .nest("/api/v1/oauth", oauth::consent_router())
        .nest("/oauth", oauth::router())
        .merge(tester::router())
        .route("/health", get(health_check))
        .route("/healthz", get(health_check))
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "rxforge-backend",
    }))
}
