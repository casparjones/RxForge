use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{app_id}/db/docs", get(list_docs).delete(delete_all))
        .route("/{app_id}/db/docs/{doc_id}", get(get_doc).put(put_doc).delete(delete_doc))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteDocQuery {
    pub rev: String,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub docs: Vec<Value>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub pages: u64,
}

async fn resolve_app_db(state: &AppState, app_id: Uuid, user_id: Uuid) -> AppResult<String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT db_scope FROM apps WHERE id = $1 AND owner_id = $2")
            .bind(app_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;

    let (db_scope,) = row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;

    Ok(if db_scope == "shared" {
        format!("app_{}", app_id)
    } else {
        format!("app_{}_user_{}", app_id, user_id)
    })
}

pub async fn list_docs(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ListResponse>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;
    let db_name = resolve_app_db(&state, app_id, user_id).await?;

    let per_page = query.per_page.unwrap_or(20).clamp(1, 100) as u32;
    let page = query.page.unwrap_or(1).max(1);
    let skip = ((page - 1) * per_page as u64) as u32;

    state.couchdb.provision_db(&db_name).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let (docs, total) = state.couchdb.list_docs(&db_name, per_page, skip).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let pages = if total == 0 { 1 } else { (total + per_page as u64 - 1) / per_page as u64 };

    Ok(Json(ListResponse { docs, total, page, per_page: per_page as u64, pages }))
}

pub async fn get_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path((app_id, doc_id)): Path<(Uuid, String)>,
) -> AppResult<Json<Value>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;
    let db_name = resolve_app_db(&state, app_id, user_id).await?;

    let doc = state.couchdb.get_doc(&db_name, &doc_id).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    Ok(Json(doc))
}

pub async fn put_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path((app_id, doc_id)): Path<(Uuid, String)>,
    Json(body): Json<Value>,
) -> AppResult<Json<Value>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;
    let db_name = resolve_app_db(&state, app_id, user_id).await?;

    let result = state.couchdb.put_doc(&db_name, &doc_id, body).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(Json(result))
}

pub async fn delete_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path((app_id, doc_id)): Path<(Uuid, String)>,
    Query(query): Query<DeleteDocQuery>,
) -> AppResult<Json<Value>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;
    let db_name = resolve_app_db(&state, app_id, user_id).await?;

    state.couchdb.delete_doc(&db_name, &doc_id, &query.rev).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn delete_all(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;
    let db_name = resolve_app_db(&state, app_id, user_id).await?;

    let deleted = state.couchdb.delete_all_docs(&db_name).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
