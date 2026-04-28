use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    linker::normalized_db_name,
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
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT a.name, a.db_scope, COALESCE(u.email, '')
         FROM apps a
         LEFT JOIN users u ON u.id = a.owner_id
         WHERE a.id = $1 AND a.owner_id = $2",
    )
    .bind(app_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    let (app_name, db_scope, email) =
        row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;

    Ok(normalized_db_name(
        &app_name,
        &app_id,
        &db_scope,
        (!email.is_empty()).then_some(email.as_str()),
        &user_id,
    ))
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

    state.linker.ensure_db(&db_name).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let (docs, total) = state.linker.list_docs(&db_name, per_page, skip).await
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

    let doc = state.linker.get_doc(&db_name, &doc_id).await
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

    let result = state.linker.put_doc(&db_name, &doc_id, body).await
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

    state.linker.delete_doc(&db_name, &doc_id, &query.rev).await
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

    let deleted = state.linker.delete_all_docs(&db_name).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
