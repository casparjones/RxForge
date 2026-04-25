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

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub documents: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub written: usize,
    pub conflicts: Vec<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Ensure the CouchDB database exists, creating it if necessary.
/// Idempotent: a 412 (already exists) from CouchDB is treated as success.
async fn ensure_db(state: &AppState, db_name: &str) -> AppResult<()> {
    state
        .couchdb
        .provision_db(db_name)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("CouchDB provision error: {e}")))
}

/// Resolve the CouchDB database name for the given app + authenticated user.
/// - db_scope = "shared"  → app_{app_id}          (all users share one DB)
/// - db_scope = "isolated" → app_{app_id}_user_{owner_id} (per-user DB)
async fn resolve_db_name(
    state: &AppState,
    app_id: Uuid,
    owner_id: Uuid,
) -> AppResult<String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT db_scope FROM apps WHERE id = $1")
            .bind(app_id)
            .fetch_optional(&state.db)
            .await?;
    let db_scope = row.map(|(s,)| s).unwrap_or_else(|| "isolated".to_string());
    Ok(if db_scope == "shared" {
        format!("app_{}", app_id)
    } else {
        format!("app_{}_user_{}", app_id, owner_id)
    })
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
        .couchdb
        .get_changes(&db_name, checkpoint, limit)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("CouchDB error: {e}")))?;

    Ok(Json(PullResponse {
        documents: changes.documents,
        checkpoint: changes.checkpoint,
    }))
}

pub async fn push(
    State(state): State<AppState>,
    auth: SyncAuth,
    Path(app_id): Path<Uuid>,
    Json(req): Json<PushRequest>,
) -> AppResult<Json<PushResponse>> {
    // Raw app tokens are read-only; the client must exchange for a JWT first.
    if matches!(auth, SyncAuth::RawToken { .. }) {
        return Err(AppError::Forbidden(
            "Write access requires a JWT. Exchange your token at POST /api/v1/auth/token/exchange"
                .to_string(),
        ));
    }

    let owner_id = resolve_owner(&state, &auth, app_id).await?;
    let db_name = resolve_db_name(&state, app_id, owner_id).await?;
    ensure_db(&state, &db_name).await?;

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

pub async fn stream(
    State(state): State<AppState>,
    auth: SyncAuth,
    Path(app_id): Path<Uuid>,
) -> AppResult<Response> {
    let owner_id = resolve_owner(&state, &auth, app_id).await?;
    let db_name = resolve_db_name(&state, app_id, owner_id).await?;
    ensure_db(&state, &db_name).await?;
    let byte_stream = state.couchdb.changes_stream(db_name).await;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("text/event-stream"));
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers.insert("X-Accel-Buffering", HeaderValue::from_static("no"));

    Ok((StatusCode::OK, headers, Body::from_stream(byte_stream)).into_response())
}
