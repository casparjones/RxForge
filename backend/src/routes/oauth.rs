use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Form, Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

/// Hash a client_secret with SHA-256. Used for storage + constant-time compare.
pub fn hash_client_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex::encode(digest)
}

fn verify_client_secret(secret: &str, stored_hash: &str) -> bool {
    // Support legacy bcrypt hashes (older rows) for forward compatibility,
    // and the new SHA-256 hex digest format.
    if stored_hash.starts_with("$2") {
        bcrypt::verify(secret, stored_hash).unwrap_or(false)
    } else {
        constant_time_eq(
            hash_client_secret(secret).as_bytes(),
            stored_hash.as_bytes(),
        )
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/authorize", get(authorize))
        .route("/client-info", get(client_info))
        .route("/token", post(token))
        .route("/revoke", post(revoke))
}

pub fn consent_router() -> Router<AppState> {
    Router::new()
        .route("/token", post(token))
        .route("/revoke", post(revoke))
        .route("/consent", post(consent_grant))
        .route("/consent/check", get(consent_check))
        .route("/rights", get(list_rights))
        .route("/rights/{client_id}", delete(revoke_right))
}

// ── Authorization Code Flow ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

pub async fn authorize(
    State(state): State<AppState>,
    Query(params): Query<AuthorizeQuery>,
) -> Response {
    if params.response_type != "code" {
        return (
            StatusCode::BAD_REQUEST,
            "unsupported_response_type: only 'code' is supported",
        )
            .into_response();
    }

    #[derive(sqlx::FromRow)]
    struct ClientRow {
        id: Uuid,
        redirect_uris: serde_json::Value,
    }

    let client_result = sqlx::query_as::<_, ClientRow>(
        "SELECT id, redirect_uris FROM apps WHERE client_id = $1 AND auth_type = 'oauth'",
    )
    .bind(&params.client_id)
    .fetch_optional(&state.db)
    .await;

    if let Err(ref e) = client_result {
        tracing::error!("authorize DB error for client_id={}: {e}", params.client_id);
    }

    let client = match client_result.unwrap_or(None) {
        Some(c) => c,
        None => {
            tracing::warn!("authorize: no oauth app found for client_id={}", params.client_id);
            return (StatusCode::BAD_REQUEST, "invalid_client").into_response();
        }
    };

    let allowed_uris: Vec<String> =
        serde_json::from_value(client.redirect_uris).unwrap_or_default();

    if !allowed_uris.contains(&params.redirect_uri) {
        return (StatusCode::BAD_REQUEST, "invalid_redirect_uri").into_response();
    }

    // Build consent page URL; the SPA handles login-check + consent UI.
    let mut consent_url = format!(
        "/oauth-consent?client_id={}&redirect_uri={}&response_type=code",
        urlencoding::encode(&params.client_id),
        urlencoding::encode(&params.redirect_uri),
    );
    if let Some(s) = &params.scope {
        consent_url.push_str(&format!("&scope={}", urlencoding::encode(s)));
    }
    if let Some(s) = &params.state {
        consent_url.push_str(&format!("&state={}", urlencoding::encode(s)));
    }

    Redirect::to(&consent_url).into_response()
}

// ── Public client info ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ClientInfoQuery {
    pub client_id: String,
}

#[derive(Debug, Serialize)]
pub struct ClientInfoResponse {
    pub client_id: String,
    pub app_name: String,
}

pub async fn client_info(
    State(state): State<AppState>,
    Query(params): Query<ClientInfoQuery>,
) -> AppResult<Json<ClientInfoResponse>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT a.name FROM oauth_clients oc
         JOIN apps a ON a.id = oc.id
         WHERE oc.client_id = $1 AND oc.active = true",
    )
    .bind(&params.client_id)
    .fetch_optional(&state.db)
    .await?;

    let (app_name,) = row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;

    Ok(Json(ClientInfoResponse {
        client_id: params.client_id,
        app_name,
    }))
}

// ── Consent endpoints (authenticated) ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConsentGrantRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConsentGrantResponse {
    pub redirect_url: String,
}

pub async fn consent_grant(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ConsentGrantRequest>,
) -> AppResult<Json<ConsentGrantResponse>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    #[derive(sqlx::FromRow)]
    struct ClientRow {
        id: Uuid,
        redirect_uris: serde_json::Value,
    }

    let client: Option<ClientRow> = sqlx::query_as(
        "SELECT id, redirect_uris FROM apps WHERE client_id = $1 AND auth_type = 'oauth'",
    )
    .bind(&req.client_id)
    .fetch_optional(&state.db)
    .await?;

    let client = client.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;

    let allowed_uris: Vec<String> =
        serde_json::from_value(client.redirect_uris).unwrap_or_default();
    if !allowed_uris.contains(&req.redirect_uri) {
        return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
    }

    // Save consent (upsert so duplicate grant is harmless).
    sqlx::query(
        "INSERT INTO oauth_consents (id, user_id, client_id, created_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (user_id, client_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(client.id)
    .execute(&state.db)
    .await?;

    let code = generate_code();
    let expires_at = Utc::now() + Duration::minutes(10);

    sqlx::query(
        "INSERT INTO oauth_codes (id, client_id, user_id, code, redirect_uri, scope, expires_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(client.id)
    .bind(user_id)
    .bind(&code)
    .bind(&req.redirect_uri)
    .bind(req.scope.as_deref().unwrap_or(""))
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let mut redirect_url = format!("{}?code={}", req.redirect_uri, code);
    if let Some(s) = &req.state {
        redirect_url.push_str(&format!("&state={s}"));
    }

    Ok(Json(ConsentGrantResponse { redirect_url }))
}

#[derive(Debug, Deserialize)]
pub struct ConsentCheckQuery {
    pub client_id: String,
}

#[derive(Debug, Serialize)]
pub struct ConsentCheckResponse {
    pub consented: bool,
}

pub async fn consent_check(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ConsentCheckQuery>,
) -> AppResult<Json<ConsentCheckResponse>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT true FROM oauth_consents oc
         JOIN oauth_clients cl ON cl.id = oc.client_id
         WHERE oc.user_id = $1 AND cl.client_id = $2",
    )
    .bind(user_id)
    .bind(&params.client_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(ConsentCheckResponse {
        consented: row.is_some(),
    }))
}

// ── Rights endpoints (authenticated) ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RightItem {
    pub client_id: String,
    pub app_name: String,
    pub granted_at: String,
}

pub async fn list_rights(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<RightItem>>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    #[derive(sqlx::FromRow)]
    struct Row {
        client_id: String,
        app_name: String,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT cl.client_id, a.name AS app_name, oc.created_at
         FROM oauth_consents oc
         JOIN oauth_clients cl ON cl.id = oc.client_id
         JOIN apps a ON a.id = cl.id
         WHERE oc.user_id = $1
         ORDER BY oc.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| RightItem {
                client_id: r.client_id,
                app_name: r.app_name,
                granted_at: r.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

pub async fn revoke_right(
    State(state): State<AppState>,
    user: AuthUser,
    Path(client_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    sqlx::query(
        "DELETE FROM oauth_consents
         WHERE user_id = $1
           AND client_id = (SELECT id FROM oauth_clients WHERE client_id = $2)",
    )
    .bind(user_id)
    .bind(&client_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "message": "Access revoked" })))
}

// ── Token ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

pub async fn token(State(state): State<AppState>, Form(req): Form<TokenRequest>) -> Response {
    match req.grant_type.as_str() {
        "authorization_code" => handle_auth_code(state, req).await,
        "client_credentials" => handle_client_credentials(state, req).await,
        _ => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "unsupported_grant_type"})),
        )
            .into_response(),
    }
}

#[derive(sqlx::FromRow)]
struct CodeRow {
    id: Uuid,
    client_id: Uuid,
    user_id: Option<Uuid>,
    redirect_uri: String,
    scope: String,
    used: bool,
    expires_at: DateTime<Utc>,
}

async fn handle_auth_code(state: AppState, req: TokenRequest) -> Response {
    let code = match req.code {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing code"})),
            )
                .into_response()
        }
    };

    let redirect_uri = match req.redirect_uri {
        Some(u) => u,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing redirect_uri"})),
            )
                .into_response()
        }
    };

    let code_row: Option<CodeRow> = sqlx::query_as(
        "SELECT id, client_id, user_id, redirect_uri, scope, used, expires_at
         FROM oauth_codes
         WHERE code = $1",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let code_row = match code_row {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_grant"})),
            )
                .into_response()
        }
    };

    if code_row.used {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "code already used"})),
        )
            .into_response();
    }

    if code_row.expires_at < Utc::now() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "code expired"})),
        )
            .into_response();
    }

    if code_row.redirect_uri != redirect_uri {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "redirect_uri mismatch"})),
        )
            .into_response();
    }

    let client_id_str = req.client_id.as_deref().unwrap_or("");

    #[derive(sqlx::FromRow)]
    struct ClientRow {
        id: Uuid,
        client_secret_hash: String,
    }

    let client: Option<ClientRow> = sqlx::query_as(
        "SELECT id, client_secret_hash FROM apps WHERE client_id = $1 AND id = $2 AND auth_type = 'oauth'",
    )
    .bind(client_id_str)
    .bind(code_row.client_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let client = match client {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid_client"})),
            )
                .into_response()
        }
    };

    // Public client flow: if client_secret is omitted or empty, skip verification.
    // Confidential clients (with a backend) may still pass the secret for extra assurance.
    if let Some(secret) = req.client_secret.as_deref().filter(|s| !s.is_empty()) {
        if !verify_client_secret(secret, &client.client_secret_hash) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid_client"})),
            )
                .into_response();
        }
    }

    let _ = sqlx::query("UPDATE oauth_codes SET used = true WHERE id = $1")
        .bind(code_row.id)
        .execute(&state.db)
        .await;

    // Issue an app-scoped JWT: app_id = the OAuth client's app UUID,
    // user_id = the user who granted consent (so sync uses their namespace).
    // Falls back to client.id if legacy code row has no user_id.
    let consenting_user_id = code_row.user_id.unwrap_or(client.id);
    let token_str = match state.jwt.issue_app_jwt(
        &code_row.id.to_string(),
        &client.id.to_string(),
        &consenting_user_id.to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("JWT error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "server_error").into_response();
        }
    };

    let _ = sqlx::query(
        "INSERT INTO oauth_tokens (id, client_id, token, scope, expires_at, created_at)
         VALUES ($1, $2, $3, $4, NOW() + INTERVAL '1 hour', NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(client.id)
    .bind(&token_str)
    .bind(&code_row.scope)
    .execute(&state.db)
    .await;

    let scope = if code_row.scope.is_empty() {
        None
    } else {
        Some(code_row.scope)
    };

    Json(TokenResponse {
        access_token: token_str,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope,
    })
    .into_response()
}

async fn handle_client_credentials(state: AppState, req: TokenRequest) -> Response {
    let client_id_str = match &req.client_id {
        Some(id) => id.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing client_id"})),
            )
                .into_response()
        }
    };

    let client_secret_str = match &req.client_secret {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "missing client_secret"})),
            )
                .into_response()
        }
    };

    #[derive(sqlx::FromRow)]
    struct ClientRow {
        id: Uuid,
        client_secret_hash: String,
    }

    let client: Option<ClientRow> = sqlx::query_as(
        "SELECT id, client_secret_hash FROM oauth_clients WHERE client_id = $1 AND active = true",
    )
    .bind(&client_id_str)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let client = match client {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid_client"})),
            )
                .into_response()
        }
    };

    let valid = verify_client_secret(&client_secret_str, &client.client_secret_hash);
    if !valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_client"})),
        )
            .into_response();
    }

    let token_str =
        match state
            .jwt
            .issue_access_token(&client.id.to_string(), "oauth_client", "user")
        {
            Ok(t) => t,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "server_error").into_response(),
        };

    let _ = sqlx::query(
        "INSERT INTO oauth_tokens (id, client_id, token, scope, expires_at, created_at)
         VALUES ($1, $2, $3, $4, NOW() + INTERVAL '1 hour', NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(client.id)
    .bind(&token_str)
    .bind(req.scope.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await;

    Json(TokenResponse {
        access_token: token_str,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: req.scope,
    })
    .into_response()
}

// ── Revoke ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
}

pub async fn revoke(State(state): State<AppState>, Form(req): Form<RevokeRequest>) -> Response {
    let _ = sqlx::query("UPDATE oauth_tokens SET revoked = true WHERE token = $1")
        .bind(&req.token)
        .execute(&state.db)
        .await;

    StatusCode::OK.into_response()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn generate_code() -> String {
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    hex::encode(bytes)
}
