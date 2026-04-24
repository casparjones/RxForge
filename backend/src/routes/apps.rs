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
        .route("/{id}", get(get_app).delete(delete_app))
        .route("/{id}/regenerate-secret", post(regenerate_secret))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AppRow {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub client_id: String,
    pub redirect_uris: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AppResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
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
            client_id: r.client_id,
            redirect_uris: serde_json::from_value(r.redirect_uris).unwrap_or_default(),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateAppResponse {
    pub id: String,
    pub name: String,
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
        "INSERT INTO apps (id, name, owner_id, client_id, client_secret_hash, redirect_uris, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
         RETURNING created_at",
    )
    .bind(id)
    .bind(&req.name)
    .bind(owner_id)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(&redirect_uris_json)
    .fetch_one(&mut *tx)
    .await?;

    // Mint the matching OAuth client row (Ticket 1 Round 3).
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
        "SELECT id, name, owner_id, client_id, redirect_uris, created_at FROM apps WHERE owner_id = $1 ORDER BY created_at DESC",
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
        "SELECT id, name, owner_id, client_id, redirect_uris, created_at FROM apps WHERE id = $1 AND owner_id = $2",
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
