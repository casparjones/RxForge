use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{app_id}/pull", get(pull))
        .route("/{app_id}/push", post(push))
        .route("/{app_id}/stream", get(stream))
}

#[derive(Debug, Deserialize)]
pub struct PullQuery {
    pub checkpoint: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PullResponse {
    pub documents: Vec<serde_json::Value>,
    pub checkpoint: serde_json::Value,
}

/// Pull handler – proxies to CouchDB with checkpoint-based pagination.
pub async fn pull(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
    Query(query): Query<PullQuery>,
) -> AppResult<Json<PullResponse>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let app: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM apps WHERE id = $1 AND owner_id = $2")
            .bind(app_id)
            .bind(owner_id)
            .fetch_optional(&state.db)
            .await?;

    if app.is_none() {
        return Err(AppError::NotFound("App not found".to_string()));
    }

    let db_name = format!("app_{}_user_{}", app_id, owner_id);
    let limit = query.limit.unwrap_or(100).min(1000);

    let changes = state
        .couchdb
        .get_changes(&db_name, query.checkpoint.as_deref(), limit)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("CouchDB error: {e}")))?;

    Ok(Json(PullResponse {
        documents: changes.documents,
        checkpoint: changes.checkpoint,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub documents: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub written: usize,
    pub conflicts: Vec<String>,
}

/// Push handler – proxies bulk_docs to CouchDB.
pub async fn push(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
    Json(req): Json<PushRequest>,
) -> AppResult<Json<PushResponse>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let app: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM apps WHERE id = $1 AND owner_id = $2")
            .bind(app_id)
            .bind(owner_id)
            .fetch_optional(&state.db)
            .await?;

    if app.is_none() {
        return Err(AppError::NotFound("App not found".to_string()));
    }

    let db_name = format!("app_{}_user_{}", app_id, owner_id);

    let result = state
        .couchdb
        .bulk_docs(&db_name, req.documents)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("CouchDB error: {e}")))?;

    Ok(Json(PushResponse {
        written: result.written,
        conflicts: result.conflicts,
    }))
}

/// SSE stream – pulls live _changes feed from CouchDB.
pub async fn stream(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
) -> Response {
    let owner_id = match Uuid::parse_str(user.user_id()) {
        Ok(id) => id,
        Err(_) => {
            return AppError::Internal(anyhow::anyhow!("Invalid user ID")).into_response()
        }
    };

    let db_name = format!("app_{}_user_{}", app_id, owner_id);
    let byte_stream = state.couchdb.changes_stream(db_name).await;

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("text/event-stream"),
    );
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers.insert("X-Accel-Buffering", HeaderValue::from_static("no"));

    (StatusCode::OK, headers, Body::from_stream(byte_stream)).into_response()
}
