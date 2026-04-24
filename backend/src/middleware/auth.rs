use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, state::AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // user id (UUID)
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,       // JWT ID for revocation
}

/// Authenticated user extracted from JWT Bearer token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: Claims,
}

impl AuthUser {
    pub fn user_id(&self) -> &str {
        &self.claims.sub
    }

    pub fn role(&self) -> &str {
        &self.claims.role
    }

    pub fn has_role(&self, role: &str) -> bool {
        match self.claims.role.as_str() {
            "superadmin" => true,
            "admin" => role != "superadmin",
            r => r == role,
        }
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::Unauthorized("Missing Authorization header".to_string()).into_response()
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("Invalid Authorization header format".to_string())
                .into_response()
        })?;

        let claims = state.jwt.verify(token).map_err(|e| {
            AppError::Unauthorized(format!("Invalid token: {e}")).into_response()
        })?;

        Ok(AuthUser { claims })
    }
}

/// Require a specific role (or higher).
/// Usage: `RequireRole("admin")` as an extractor.
pub struct RequireRole(pub &'static str);

impl RequireRole {
    pub fn check(&self, user: &AuthUser) -> Result<(), AppError> {
        if !user.has_role(self.0) {
            return Err(AppError::Forbidden(format!(
                "Role '{}' required",
                self.0
            )));
        }
        Ok(())
    }
}

/// Permission-based ACL check helper.
///
/// `apps:*` are granted to any standard user role (`user`, `admin`, `superadmin`).
/// Management/analytics permissions are admin-only. Unknown or downgraded roles
/// (e.g. a hypothetical `readonly`/`guest`) are denied across the board.
pub fn require_permission(user: &AuthUser, permission: &str) -> Result<(), AppError> {
    let allowed = match permission {
        "apps:create" | "apps:read" | "apps:delete" => user.has_role("user"),
        "users:manage" => user.has_role("admin"),
        "analytics:read" => user.has_role("admin"),
        "analytics:global" => user.has_role("admin"),
        "admin:access" => user.has_role("admin"),
        _ => false,
    };

    if !allowed {
        return Err(AppError::Forbidden(format!(
            "Permission '{permission}' required"
        )));
    }

    Ok(())
}
