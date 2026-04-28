use axum::{
    body::Body,
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    linker::normalized_db_name,
    routes::tokens::hash_app_token,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{app_id}/pull", get(pull))
        .route("/{app_id}/push", post(push))
        .route("/{app_id}/stream", get(stream))
}

// ── SyncAuth extractor ────────────────────────────────────────────────────────

/// Resolved identity for sync endpoints.
/// Supports three token types:
/// - User JWT (regular login)  → ownership verified via DB
/// - App JWT (from exchange)   → app_id + owner_id embedded in claims
/// - Raw app token (rxft_...)  → read-only; app_id + owner_id from DB lookup
pub enum SyncAuth {
    User { user_id: Uuid },
    AppJwt { app_id: Uuid, owner_id: Uuid },
    RawToken { app_id: Uuid, owner_id: Uuid, token_id: Uuid },
}

/// Extract Bearer token from `Authorization` header or `?token=` query param.
/// The query-param fallback exists because the browser `EventSource` API
/// cannot set custom headers.
fn extract_bearer(parts: &Parts) -> Option<String> {
    if let Some(token) = parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        return Some(token.to_string());
    }

    // ?token=<value>
    parts.uri.query().and_then(|q| {
        q.split('&')
            .find(|seg| seg.starts_with("token="))
            .and_then(|seg| seg.strip_prefix("token="))
            .map(|t| t.to_string())
    })
}

impl FromRequestParts<AppState> for SyncAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts).ok_or_else(|| {
            AppError::Unauthorized(
                "Missing Authorization header or token query parameter".to_string(),
            )
            .into_response()
        })?;

        // Raw app token (prefix rxft_) — read-only access
        if token.starts_with("rxft_") {
            let token_hash = hash_app_token(&token);
            let row: Option<(Uuid, Uuid, Uuid, Vec<String>)> = sqlx::query_as(
                "SELECT t.id, t.app_id, a.owner_id, t.allowed_origins
                 FROM app_tokens t
                 JOIN apps a ON a.id = t.app_id
                 WHERE t.token_hash = $1 AND t.revoked_at IS NULL",
            )
            .bind(&token_hash)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")).into_response())?;

            let (token_id, app_id, owner_id, allowed_origins) = row.ok_or_else(|| {
                AppError::Unauthorized("Invalid or revoked token".to_string()).into_response()
            })?;

            check_origin(&parts.headers, &allowed_origins)
                .map_err(|e| e.into_response())?;

            // Update last_used_at without blocking the request
            let db = state.db.clone();
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE app_tokens SET last_used_at = NOW() WHERE id = $1")
                    .bind(token_id)
                    .execute(&db)
                    .await;
            });

            return Ok(SyncAuth::RawToken { app_id, owner_id, token_id });
        }

        // Try user JWT first (has `role` claim; app JWT has `app_id` instead)
        if let Ok(claims) = state.jwt.verify(&token) {
            let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
                AppError::Internal(anyhow::anyhow!("Invalid user ID")).into_response()
            })?;
            return Ok(SyncAuth::User { user_id });
        }

        // Try app-scoped JWT (from token exchange)
        if let Ok(claims) = state.jwt.verify_app_jwt(&token) {
            let app_id = Uuid::parse_str(&claims.app_id).map_err(|_| {
                AppError::Internal(anyhow::anyhow!("Invalid app_id in JWT")).into_response()
            })?;
            let owner_id = Uuid::parse_str(&claims.user_id).map_err(|_| {
                AppError::Internal(anyhow::anyhow!("Invalid user_id in JWT")).into_response()
            })?;
            return Ok(SyncAuth::AppJwt { app_id, owner_id });
        }

        Err(AppError::Unauthorized("Invalid or expired token".to_string()).into_response())
    }
}

/// Resolve the CouchDB owner_id for a given app_id + auth combo.
/// Returns Err if the auth doesn't grant access to this app.
async fn resolve_owner(
    state: &AppState,
    auth: &SyncAuth,
    app_id: Uuid,
) -> AppResult<Uuid> {
    match auth {
        SyncAuth::User { user_id } => {
            let row: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM apps WHERE id = $1 AND owner_id = $2")
                    .bind(app_id)
                    .bind(*user_id)
                    .fetch_optional(&state.db)
                    .await?;
            row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;
            Ok(*user_id)
        }
        SyncAuth::AppJwt {
            app_id: jwt_app_id,
            owner_id,
        } => {
            if *jwt_app_id != app_id {
                return Err(AppError::Forbidden(
                    "Token not valid for this app".to_string(),
                ));
            }
            Ok(*owner_id)
        }
        SyncAuth::RawToken {
            app_id: token_app_id,
            owner_id,
            ..
        } => {
            if *token_app_id != app_id {
                return Err(AppError::Forbidden(
                    "Token not valid for this app".to_string(),
                ));
            }
            Ok(*owner_id)
        }
    }
}

/// Check Origin header against allowed_origins list.
/// If the list is empty, all origins are permitted.
fn check_origin(headers: &HeaderMap, allowed_origins: &[String]) -> AppResult<()> {
    if allowed_origins.is_empty() {
        return Ok(());
    }
    let origin = headers
        .get("Origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !allowed_origins.iter().any(|o| o == origin) {
        return Err(AppError::Forbidden(
            "Request origin not allowed for this token".to_string(),
        ));
    }
    Ok(())
}

// ── Request/Response types ────────────────────────────────────────────────────

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

/// One row in an RxDB push request.
#[derive(Debug, Deserialize)]
pub struct PushRow {
    /// What the client believes the server currently holds (null = new document).
    pub assumed_master_state: Option<serde_json::Value>,
    /// The document state the client wants to write.
    pub new_document_state: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub rows: Vec<PushRow>,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    /// Full server documents for rows where the assumed state didn't match.
    pub conflicts: Vec<serde_json::Value>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Strip storage-internal fields (`_id`, `_rev`, `_seq`) from a document.
/// Keeps `_deleted` because RxDB uses it as the soft-delete marker.
fn strip_internal(doc: &serde_json::Value) -> serde_json::Value {
    let mut d = doc.clone();
    if let Some(obj) = d.as_object_mut() {
        obj.remove("_id");
        obj.remove("_rev");
        obj.remove("_seq");
    }
    d
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn ensure_db(state: &AppState, db_name: &str) -> AppResult<()> {
    state
        .linker
        .ensure_db(db_name)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("storage ensure error: {e}")))
}

async fn resolve_db_name(
    state: &AppState,
    app_id: Uuid,
    owner_id: Uuid,
) -> AppResult<String> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT a.name, a.db_scope, COALESCE(u.email, '')
         FROM apps a
         LEFT JOIN users u ON u.id = a.owner_id
         WHERE a.id = $1 AND a.owner_id = $2",
    )
    .bind(app_id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await?;

    let (app_name, db_scope, email) =
        row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;

    Ok(normalized_db_name(
        &app_name,
        &app_id,
        &db_scope,
        (!email.is_empty()).then_some(email.as_str()),
        &owner_id,
    ))
}

pub async fn pull(
    State(state): State<AppState>,
    auth: SyncAuth,
    Path(app_id): Path<Uuid>,
    Query(query): Query<PullQuery>,
) -> AppResult<Json<PullResponse>> {
    let owner_id = resolve_owner(&state, &auth, app_id).await?;
    let db_name = resolve_db_name(&state, app_id, owner_id).await?;
    ensure_db(&state, &db_name).await?;
    let limit = query.limit.unwrap_or(100).min(1000);
    let checkpoint = query.checkpoint.as_deref().filter(|s| !s.is_empty());

    let changes = state
        .linker
        .get_changes(&db_name, checkpoint, limit)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("storage error: {e}")))?;

    Ok(Json(PullResponse {
        documents: changes.documents.iter().map(strip_internal).collect(),
        checkpoint: changes.checkpoint,
    }))
}

pub async fn push(
    State(state): State<AppState>,
    auth: SyncAuth,
    Path(app_id): Path<Uuid>,
    Json(req): Json<PushRequest>,
) -> AppResult<Json<PushResponse>> {
    if matches!(auth, SyncAuth::RawToken { .. }) {
        return Err(AppError::Forbidden(
            "Write access requires a JWT. Exchange your token at POST /api/v1/auth/token/exchange"
                .to_string(),
        ));
    }

    let owner_id = resolve_owner(&state, &auth, app_id).await?;
    let db_name = resolve_db_name(&state, app_id, owner_id).await?;
    ensure_db(&state, &db_name).await?;

    let mut conflicts: Vec<serde_json::Value> = Vec::new();
    let mut docs_to_write: Vec<serde_json::Value> = Vec::new();

    for row in req.rows {
        let doc_id = row.new_document_state
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::BadRequest("Document missing 'id' field".to_string()))?
            .to_string();

        // Fetch current server state for conflict detection
        let current = state.linker.get_doc(&db_name, &doc_id).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("storage read error: {e}")))?;
        let current_clean = current.as_ref().map(strip_internal);

        // RxDB conflict semantics: assumed_master_state must equal actual server state
        let has_conflict = match (&row.assumed_master_state, &current_clean) {
            (None, None)             => false, // new doc, server empty → OK
            (None, Some(_))          => true,  // client says new, server has doc → conflict
            (Some(_), None)          => false, // client updating, server gone → allow
            (Some(assumed), Some(actual)) => assumed != actual,
        };

        if has_conflict {
            if let Some(master) = current_clean {
                conflicts.push(master);
            }
            continue;
        }

        // Prepare doc for the storage backend: add _id = doc.id, carry current _rev for CouchDB
        let mut stored = row.new_document_state.clone();
        if let Some(obj) = stored.as_object_mut() {
            obj.insert("_id".to_string(), serde_json::Value::String(doc_id.clone()));
            if let Some(cur) = &current {
                if let Some(rev) = cur.get("_rev") {
                    obj.insert("_rev".to_string(), rev.clone());
                }
            }
        }
        docs_to_write.push(stored);
    }

    if !docs_to_write.is_empty() {
        state.linker.bulk_docs(&db_name, docs_to_write).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("storage write error: {e}")))?;
    }

    Ok(Json(PushResponse { conflicts }))
}

pub async fn stream(
    State(state): State<AppState>,
    auth: SyncAuth,
    Path(app_id): Path<Uuid>,
) -> AppResult<Response> {
    let owner_id = resolve_owner(&state, &auth, app_id).await?;
    let db_name = resolve_db_name(&state, app_id, owner_id).await?;
    ensure_db(&state, &db_name).await?;
    let byte_stream = state.linker.changes_stream(db_name);

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("text/event-stream"));
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers.insert("X-Accel-Buffering", HeaderValue::from_static("no"));

    Ok((StatusCode::OK, headers, Body::from_stream(byte_stream)).into_response())
}
