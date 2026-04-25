use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{app_id}/tokens", post(create_token).get(list_tokens))
        .route("/{app_id}/tokens/{token_id}", delete(revoke_token))
}

pub fn hash_app_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_app_token() -> String {
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    format!("rxft_{}", hex::encode(bytes))
}

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct AppTokenRow {
    pub id: Uuid,
    pub app_id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub allowed_origins: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AppTokenResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub token_prefix: String,
    pub allowed_origins: Vec<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked: bool,
}

impl From<AppTokenRow> for AppTokenResponse {
    fn from(r: AppTokenRow) -> Self {
        AppTokenResponse {
            id: r.id.to_string(),
            app_id: r.app_id.to_string(),
            name: r.name,
            token_prefix: r.token_prefix,
            allowed_origins: r.allowed_origins,
            created_at: r.created_at.to_rfc3339(),
            last_used_at: r.last_used_at.map(|t| t.to_rfc3339()),
            revoked: r.revoked_at.is_some(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: Option<String>,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    /// Plaintext token — shown exactly once, never stored.
    pub token: String,
    pub token_prefix: String,
    pub allowed_origins: Vec<String>,
    pub created_at: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn create_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
    Json(req): Json<CreateTokenRequest>,
) -> AppResult<Json<CreateTokenResponse>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let app: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM apps WHERE id = $1 AND owner_id = $2 AND auth_type = 'token'",
    )
    .bind(app_id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await?;

    if app.is_none() {
        return Err(AppError::NotFound(
            "App not found or not a token-based app".to_string(),
        ));
    }

    let token_plain = generate_app_token();
    let token_hash = hash_app_token(&token_plain);
    let token_prefix = token_plain[..16].to_string(); // "rxft_" + 11 hex chars
    let token_id = Uuid::new_v4();
    let name = req.name.unwrap_or_else(|| "Default".to_string());

    let (created_at,): (DateTime<Utc>,) = sqlx::query_as(
        "INSERT INTO app_tokens (id, app_id, name, token_hash, token_prefix, allowed_origins, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW())
         RETURNING created_at",
    )
    .bind(token_id)
    .bind(app_id)
    .bind(&name)
    .bind(&token_hash)
    .bind(&token_prefix)
    .bind(&req.allowed_origins)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(CreateTokenResponse {
        id: token_id.to_string(),
        app_id: app_id.to_string(),
        name,
        token: token_plain,
        token_prefix,
        allowed_origins: req.allowed_origins,
        created_at: created_at.to_rfc3339(),
    }))
}

pub async fn list_tokens(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Vec<AppTokenResponse>>> {
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

    let rows: Vec<AppTokenRow> = sqlx::query_as(
        "SELECT id, app_id, name, token_prefix, allowed_origins, created_at, last_used_at, revoked_at
         FROM app_tokens WHERE app_id = $1 ORDER BY created_at DESC",
    )
    .bind(app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(AppTokenResponse::from).collect()))
}

pub async fn revoke_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path((app_id, token_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let result = sqlx::query(
        "UPDATE app_tokens SET revoked_at = NOW()
         WHERE id = $1 AND app_id = $2 AND revoked_at IS NULL
           AND EXISTS (SELECT 1 FROM apps WHERE id = $2 AND owner_id = $3)",
    )
    .bind(token_id)
    .bind(app_id)
    .bind(owner_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Token not found or already revoked".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({ "revoked": true })))
}

// ── Token Exchange (mounted in auth router) ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExchangeRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,
}

pub async fn exchange_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ExchangeRequest>,
) -> AppResult<Json<ExchangeResponse>> {
    if !req.token.starts_with("rxft_") {
        return Err(AppError::BadRequest("Invalid token format".to_string()));
    }

    let token_hash = hash_app_token(&req.token);

    let row: Option<(Uuid, Uuid, Uuid, Vec<String>)> = sqlx::query_as(
        "SELECT t.id, t.app_id, a.owner_id, t.allowed_origins
         FROM app_tokens t
         JOIN apps a ON a.id = t.app_id
         WHERE t.token_hash = $1 AND t.revoked_at IS NULL",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;

    let (token_id, app_id, owner_id, allowed_origins) = row
        .ok_or_else(|| AppError::Unauthorized("Invalid or revoked token".to_string()))?;

    // Origin-Binding: wenn allowed_origins konfiguriert, muss Origin übereinstimmen
    if !allowed_origins.is_empty() {
        let request_origin = headers
            .get("Origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !allowed_origins.iter().any(|o| o == request_origin) {
            return Err(AppError::Forbidden(
                "Request origin not allowed for this token".to_string(),
            ));
        }
    }

    sqlx::query("UPDATE app_tokens SET last_used_at = NOW() WHERE id = $1")
        .bind(token_id)
        .execute(&state.db)
        .await?;

    let jwt = state
        .jwt
        .issue_app_jwt(
            &token_id.to_string(),
            &app_id.to_string(),
            &owner_id.to_string(),
        )
        .map_err(|e| AppError::Internal(e))?;

    Ok(Json(ExchangeResponse {
        access_token: jwt,
        token_type: "Bearer".to_string(),
        expires_in: 900,
    }))
}
