use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::{require_permission, AuthUser},
    routes::oauth::hash_client_secret,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_app).get(list_apps))
        .route("/{id}", get(get_app).delete(delete_app).patch(update_app))
        .route("/{id}/regenerate-secret", post(regenerate_secret))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AppRow {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub auth_type: String,
    pub db_scope: String,
    pub client_id: String,
    pub redirect_uris: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AppResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub auth_type: String,
    pub db_scope: String,
    pub client_id: String,
    pub redirect_uris: Vec<String>,
    pub created_at: String,
}

impl From<AppRow> for AppResponse {
    fn from(r: AppRow) -> Self {
        AppResponse {
            id: r.id.to_string(),
            name: r.name,
            owner_id: r.owner_id.to_string(),
            auth_type: r.auth_type,
            db_scope: r.db_scope,
            client_id: r.client_id,
            redirect_uris: serde_json::from_value(r.redirect_uris).unwrap_or_default(),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    /// "oauth" (default) or "token"
    #[serde(default = "default_auth_type")]
    pub auth_type: String,
    /// "isolated" (default) or "shared"
    #[serde(default = "default_db_scope")]
    pub db_scope: String,
}

fn default_auth_type() -> String {
    "oauth".to_string()
}

fn default_db_scope() -> String {
    "isolated".to_string()
}

#[derive(Debug, Serialize)]
pub struct CreateAppResponse {
    pub id: String,
    pub name: String,
    pub auth_type: String,
    pub db_scope: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
    pub created_at: String,
}

fn generate_client_id() -> String {
    let id = Uuid::new_v4().to_string().replace('-', "");
    format!("rxf_{}", &id[..16])
}

fn generate_client_secret() -> String {
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    hex::encode(bytes)
}

pub async fn create_app(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateAppRequest>,
) -> AppResult<Json<CreateAppResponse>> {
    require_permission(&user, "apps:create")?;

    if req.name.is_empty() {
        return Err(AppError::BadRequest("App name is required".to_string()));
    }

    if req.auth_type != "oauth" && req.auth_type != "token" {
        return Err(AppError::BadRequest(
            "auth_type must be 'oauth' or 'token'".to_string(),
        ));
    }

    if req.db_scope != "isolated" && req.db_scope != "shared" {
        return Err(AppError::BadRequest(
            "db_scope must be 'isolated' or 'shared'".to_string(),
        ));
    }

    let id = Uuid::new_v4();
    let client_id = generate_client_id();
    let client_secret_plain = generate_client_secret();
    let client_secret_hash = hash_client_secret(&client_secret_plain);

    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let redirect_uris_json = serde_json::to_value(&req.redirect_uris)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Serialization error: {e}")))?;

    let mut tx = state.db.begin().await?;

    let (created_at,): (DateTime<Utc>,) = sqlx::query_as(
        "INSERT INTO apps (id, name, owner_id, auth_type, db_scope, client_id, client_secret_hash, redirect_uris, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
         RETURNING created_at",
    )
    .bind(id)
    .bind(&req.name)
    .bind(owner_id)
    .bind(&req.auth_type)
    .bind(&req.db_scope)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(&redirect_uris_json)
    .fetch_one(&mut *tx)
    .await?;

    // OAuth apps get a matching oauth_clients row; token apps don't use OAuth flow.
    if req.auth_type == "oauth" {
        sqlx::query(
            "INSERT INTO oauth_clients (id, client_id, client_secret_hash, redirect_uris, scope, active, owner_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, '', true, $5, NOW(), NOW())",
        )
        .bind(id)
        .bind(&client_id)
        .bind(&client_secret_hash)
        .bind(&redirect_uris_json)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;
    }

    // Reserve a CouchDB db-prefix entry (actual provisioning on first sync).
    let db_prefix = format!("app_{}", id.simple());
    sqlx::query(
        "INSERT INTO app_db_reservations (app_id, db_prefix, provisioned, created_at)
         VALUES ($1, $2, false, NOW())",
    )
    .bind(id)
    .bind(&db_prefix)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(CreateAppResponse {
        id: id.to_string(),
        name: req.name,
        auth_type: req.auth_type,
        db_scope: req.db_scope,
        client_id,
        client_secret: client_secret_plain,
        redirect_uris: req.redirect_uris,
        created_at: created_at.to_rfc3339(),
    }))
}

pub async fn list_apps(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<AppResponse>>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let rows: Vec<AppRow> = sqlx::query_as(
        "SELECT id, name, owner_id, auth_type, db_scope, client_id, redirect_uris, created_at FROM apps WHERE owner_id = $1 ORDER BY created_at DESC",
    )
    .bind(owner_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows.into_iter().map(AppResponse::from).collect()))
}

pub async fn get_app(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AppResponse>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let row: Option<AppRow> = sqlx::query_as(
        "SELECT id, name, owner_id, auth_type, db_scope, client_id, redirect_uris, created_at FROM apps WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await?;

    let row = row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;
    Ok(Json(AppResponse::from(row)))
}

pub async fn delete_app(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let result = sqlx::query("DELETE FROM apps WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("App not found".to_string()));
    }

    Ok(Json(serde_json::json!({"deleted": true})))
}

#[derive(Debug, Serialize)]
pub struct RegenerateSecretResponse {
    pub client_secret: String,
}

pub async fn regenerate_secret(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<RegenerateSecretResponse>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let new_secret = generate_client_secret();
    let new_hash = hash_client_secret(&new_secret);

    let mut tx = state.db.begin().await?;

    let result = sqlx::query(
        "UPDATE apps SET client_secret_hash = $1, updated_at = NOW() WHERE id = $2 AND owner_id = $3",
    )
    .bind(&new_hash)
    .bind(id)
    .bind(owner_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("App not found".to_string()));
    }

    sqlx::query(
        "UPDATE oauth_clients SET client_secret_hash = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(&new_hash)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(RegenerateSecretResponse {
        client_secret: new_secret,
    }))
}

// ── Update app ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateAppRequest {
    pub name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub auth_type: Option<String>,
    pub db_scope: Option<String>,
}

pub async fn update_app(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAppRequest>,
) -> AppResult<Json<AppResponse>> {
    require_permission(&user, "apps:create")?; // same gate as create

    if let Some(ref t) = req.auth_type {
        if t != "oauth" && t != "token" {
            return Err(AppError::BadRequest(
                "auth_type must be 'oauth' or 'token'".to_string(),
            ));
        }
    }

    if let Some(ref s) = req.db_scope {
        if s != "isolated" && s != "shared" {
            return Err(AppError::BadRequest(
                "db_scope must be 'isolated' or 'shared'".to_string(),
            ));
        }
    }

    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let current: Option<AppRow> = sqlx::query_as(
        "SELECT id, name, owner_id, auth_type, db_scope, client_id, redirect_uris, created_at
         FROM apps WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await?;

    let current = current.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;

    let new_name = req.name.unwrap_or(current.name);
    let new_auth_type = req.auth_type.unwrap_or(current.auth_type.clone());
    let new_db_scope = req.db_scope.unwrap_or(current.db_scope.clone());
    let new_redirect_uris: Vec<String> = req.redirect_uris.unwrap_or_else(|| {
        serde_json::from_value(current.redirect_uris.clone()).unwrap_or_default()
    });
    let redirect_uris_json = serde_json::to_value(&new_redirect_uris)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Serialization error: {e}")))?;

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "UPDATE apps SET name = $1, auth_type = $2, db_scope = $3, redirect_uris = $4, updated_at = NOW()
         WHERE id = $5 AND owner_id = $6",
    )
    .bind(&new_name)
    .bind(&new_auth_type)
    .bind(&new_db_scope)
    .bind(&redirect_uris_json)
    .bind(id)
    .bind(owner_id)
    .execute(&mut *tx)
    .await?;

    // Keep oauth_clients in sync with auth_type changes
    if new_auth_type == "oauth" {
        // Upsert: ensure an active oauth_client exists
        sqlx::query(
            "INSERT INTO oauth_clients (id, client_id, client_secret_hash, redirect_uris, scope, active, owner_id, created_at, updated_at)
             SELECT id, client_id, client_secret_hash, $1, '', true, owner_id, NOW(), NOW()
             FROM apps WHERE id = $2
             ON CONFLICT (id) DO UPDATE
               SET active = true, redirect_uris = $1, updated_at = NOW()",
        )
        .bind(&redirect_uris_json)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    } else {
        // Deactivate oauth_client when switching to token auth
        sqlx::query(
            "UPDATE oauth_clients SET active = false, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let updated: AppRow = sqlx::query_as(
        "SELECT id, name, owner_id, auth_type, db_scope, client_id, redirect_uris, created_at
         FROM apps WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(AppResponse::from(updated)))
}
