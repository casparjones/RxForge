use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    linker::{normalized_db_name, DeletedFilter},
    middleware::auth::{require_permission, AuthUser},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users",                    get(list_users))
        .route("/users/{id}/role",          patch(update_user_role))
        .route("/users/{id}/lock",          put(set_locked))
        .route("/users/{id}/permissions",   put(update_permissions))
        .route("/users/{id}/apps",          get(list_user_apps))
        .route(
            "/users/{user_id}/apps/{app_id}/db/docs",
            get(admin_list_docs).delete(admin_delete_all),
        )
        .route(
            "/users/{user_id}/apps/{app_id}/db/docs/{doc_id}",
            get(admin_get_doc).delete(admin_delete_doc),
        )
}

#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub locked: bool,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserAdminResponse {
    pub id: String,
    pub email: String,
    pub role: String,
    pub locked: bool,
    pub permissions: Vec<String>,
    pub created_at: String,
}

pub async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<UserAdminResponse>>> {
    require_permission(&user, "users:manage")?;

    let rows: Vec<UserRow> = sqlx::query_as(
        "SELECT id, email, role, locked, permissions, created_at FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let users = rows
        .into_iter()
        .map(|r| UserAdminResponse {
            id: r.id.to_string(),
            email: r.email,
            role: r.role,
            locked: r.locked,
            permissions: r.permissions,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(users))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

pub async fn update_user_role(
    State(state): State<AppState>,
    user: AuthUser,
    Path(target_id): Path<Uuid>,
    Json(req): Json<UpdateRoleRequest>,
) -> AppResult<Json<MessageResponse>> {
    require_permission(&user, "users:manage")?;

    let valid_roles = ["user", "admin", "superadmin"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Invalid role '{}'. Must be one of: user, admin, superadmin",
            req.role
        )));
    }

    if req.role == "superadmin" && !user.has_role("superadmin") {
        return Err(AppError::Forbidden(
            "Only superadmin can assign superadmin role".to_string(),
        ));
    }

    let result = sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
        .bind(&req.role)
        .bind(target_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    Ok(Json(MessageResponse {
        message: format!("User role updated to '{}'", req.role),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SetLockedRequest {
    pub locked: bool,
}

pub async fn set_locked(
    State(state): State<AppState>,
    user: AuthUser,
    Path(target_id): Path<Uuid>,
    Json(req): Json<SetLockedRequest>,
) -> AppResult<Json<MessageResponse>> {
    require_permission(&user, "users:manage")?;

    let result = sqlx::query(
        "UPDATE users SET locked = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(req.locked)
    .bind(target_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    Ok(Json(MessageResponse {
        message: if req.locked { "Account locked.".into() } else { "Account unlocked.".into() },
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionsRequest {
    pub permissions: Vec<String>,
}

pub async fn update_permissions(
    State(state): State<AppState>,
    user: AuthUser,
    Path(target_id): Path<Uuid>,
    Json(req): Json<UpdatePermissionsRequest>,
) -> AppResult<Json<MessageResponse>> {
    require_permission(&user, "users:manage")?;

    let result = sqlx::query(
        "UPDATE users SET permissions = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(&req.permissions)
    .bind(target_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    Ok(Json(MessageResponse {
        message: "Permissions updated.".into(),
    }))
}

#[derive(Debug, sqlx::FromRow)]
struct AppRow {
    pub id: Uuid,
    pub name: String,
    pub auth_type: String,
    pub db_scope: String,
    pub relationship: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AppAdminResponse {
    pub id: String,
    pub name: String,
    pub auth_type: String,
    pub db_scope: String,
    pub relationship: String,
    pub created_at: String,
}

pub async fn list_user_apps(
    State(state): State<AppState>,
    user: AuthUser,
    Path(target_id): Path<Uuid>,
) -> AppResult<Json<Vec<AppAdminResponse>>> {
    require_permission(&user, "users:manage")?;

    let rows: Vec<AppRow> = sqlx::query_as(
        "SELECT a.id, a.name, a.auth_type, a.db_scope, 'owner' AS relationship, a.created_at
         FROM apps a
         WHERE a.owner_id = $1
         UNION
         SELECT a.id, a.name, a.auth_type, a.db_scope, 'consented' AS relationship, oc.created_at
         FROM oauth_consents oc
         JOIN apps a ON a.id = oc.client_id
         WHERE oc.user_id = $1
         ORDER BY created_at DESC",
    )
    .bind(target_id)
    .fetch_all(&state.db)
    .await?;

    let apps = rows
        .into_iter()
        .map(|r| AppAdminResponse {
            id: r.id.to_string(),
            name: r.name,
            auth_type: r.auth_type,
            db_scope: r.db_scope,
            relationship: r.relationship,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(apps))
}

async fn resolve_admin_db(state: &AppState, app_id: Uuid, target_user_id: Uuid) -> AppResult<String> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT a.name, a.db_scope, COALESCE(u.email, '')
         FROM apps a
         LEFT JOIN users u ON u.id = a.owner_id
         WHERE a.id = $1",
    )
    .bind(app_id)
    .fetch_optional(&state.db)
    .await?;

    let (app_name, db_scope, _owner_email) =
        row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;

    let user_email_row: Option<(String,)> = sqlx::query_as(
        "SELECT email FROM users WHERE id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(&state.db)
    .await?;

    let user_email = user_email_row.map(|(e,)| e);

    Ok(normalized_db_name(
        &app_name,
        &app_id,
        &db_scope,
        user_email.as_deref(),
        &target_user_id,
    ))
}

#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    /// Free-text search matched against each document's JSON representation.
    pub search: Option<String>,
    /// Tombstone filter: `active` (default), `deleted`, or `all`.
    pub deleted: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminDeleteDocQuery {
    pub rev: String,
}

#[derive(Debug, Serialize)]
pub struct AdminListResponse {
    pub docs: Vec<serde_json::Value>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub pages: u64,
}

pub async fn admin_list_docs(
    State(state): State<AppState>,
    user: AuthUser,
    Path((user_id, app_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<AdminListQuery>,
) -> AppResult<Json<AdminListResponse>> {
    require_permission(&user, "users:manage")?;

    let db_name = resolve_admin_db(&state, app_id, user_id).await?;

    let per_page = query.per_page.unwrap_or(20).clamp(1, 100) as u32;
    let page = query.page.unwrap_or(1).max(1);
    let skip = ((page - 1) * per_page as u64) as u32;

    state.linker.ensure_db(&db_name).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let deleted = DeletedFilter::from_query(query.deleted.as_deref());
    let (docs, total) = state
        .linker
        .list_docs(&db_name, per_page, skip, query.search.as_deref(), deleted)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    let pages = if total == 0 { 1 } else { (total + per_page as u64 - 1) / per_page as u64 };

    Ok(Json(AdminListResponse { docs, total, page, per_page: per_page as u64, pages }))
}

pub async fn admin_get_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path((user_id, app_id, doc_id)): Path<(Uuid, Uuid, String)>,
) -> AppResult<Json<serde_json::Value>> {
    require_permission(&user, "users:manage")?;

    let db_name = resolve_admin_db(&state, app_id, user_id).await?;

    let doc = state.linker.get_doc(&db_name, &doc_id).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    Ok(Json(doc))
}

pub async fn admin_delete_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path((user_id, app_id, doc_id)): Path<(Uuid, Uuid, String)>,
    Query(query): Query<AdminDeleteDocQuery>,
) -> AppResult<Json<serde_json::Value>> {
    require_permission(&user, "users:manage")?;

    let db_name = resolve_admin_db(&state, app_id, user_id).await?;

    state.linker.delete_doc(&db_name, &doc_id, &query.rev).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn admin_delete_all(
    State(state): State<AppState>,
    user: AuthUser,
    Path((user_id, app_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    require_permission(&user, "users:manage")?;

    let db_name = resolve_admin_db(&state, app_id, user_id).await?;

    let deleted = state.linker.delete_all_docs(&db_name).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

#[cfg(test)]
mod tests {
    use crate::middleware::auth::{AuthUser, Claims};

    fn make_user(role: &str) -> AuthUser {
        AuthUser {
            claims: Claims {
                sub: uuid::Uuid::new_v4().to_string(),
                email: "test@example.com".to_string(),
                role: role.to_string(),
                exp: 9999999999,
                iat: 0,
                jti: uuid::Uuid::new_v4().to_string(),
            },
        }
    }

    #[test]
    fn test_admin_has_role_user() {
        let user = make_user("admin");
        assert!(user.has_role("user"));
        assert!(user.has_role("admin"));
        assert!(!user.has_role("superadmin"));
    }

    #[test]
    fn test_superadmin_has_all_roles() {
        let user = make_user("superadmin");
        assert!(user.has_role("user"));
        assert!(user.has_role("admin"));
        assert!(user.has_role("superadmin"));
    }

    #[test]
    fn test_user_has_only_user_role() {
        let user = make_user("user");
        assert!(user.has_role("user"));
        assert!(!user.has_role("admin"));
    }
}
