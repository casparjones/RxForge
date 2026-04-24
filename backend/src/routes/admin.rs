use axum::{
    extract::{Path, State},
    routing::{get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::{require_permission, AuthUser},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/role", patch(update_user_role))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserAdminResponse {
    pub id: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

pub async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<UserAdminResponse>>> {
    require_permission(&user, "users:manage")?;

    let rows: Vec<UserRow> =
        sqlx::query_as("SELECT id, email, role, created_at FROM users ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;

    let users = rows
        .into_iter()
        .map(|r| UserAdminResponse {
            id: r.id.to_string(),
            email: r.email,
            role: r.role,
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
