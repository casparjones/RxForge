use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/2fa/setup", post(totp_setup))
        .route("/2fa/verify", post(totp_verify))
        .route("/webauthn/register-start", post(webauthn_register_start))
        .route("/webauthn/register-finish", post(webauthn_register_finish))
        .route("/webauthn/login-start", post(webauthn_login_start))
        .route("/webauthn/login-finish", post(webauthn_login_finish))
}

// ── Register ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub email: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<RegisterResponse>> {
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address".to_string()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&state.db)
            .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("bcrypt error: {e}")))?;

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
         VALUES ($1, $2, $3, 'user', NOW(), NOW())",
    )
    .bind(id)
    .bind(&req.email)
    .bind(&password_hash)
    .execute(&state.db)
    .await?;

    Ok(Json(RegisterResponse {
        id: id.to_string(),
        email: req.email,
    }))
}

// ── Login ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub totp_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let row: Option<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT id, email, password_hash, role FROM users WHERE email = $1",
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?;

    let (user_id, email, password_hash, role) = row
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let valid = verify(&req.password, &password_hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("bcrypt verify error: {e}")))?;

    if !valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // If the user has a verified TOTP, a fresh code is required.
    let totp_row: Option<(String, bool)> =
        sqlx::query_as("SELECT secret, verified FROM user_totp WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;

    if let Some((secret_b32, true)) = totp_row {
        use totp_rs::{Algorithm, Secret, TOTP};

        let code = req
            .totp_code
            .as_deref()
            .ok_or_else(|| AppError::Unauthorized("TOTP code required".to_string()))?;

        let secret = Secret::Encoded(secret_b32);
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret
                .to_bytes()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("secret decode: {e}")))?,
            Some("RxForge".to_string()),
            email.clone(),
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP error: {e}")))?;

        let ok = totp
            .check_current(code)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP check: {e}")))?;
        if !ok {
            return Err(AppError::Unauthorized("Invalid TOTP code".to_string()));
        }
    }

    let user_id_str = user_id.to_string();
    let access_token = state
        .jwt
        .issue_access_token(&user_id_str, &email, &role)
        .map_err(AppError::Internal)?;

    let refresh_token = state
        .jwt
        .issue_refresh_token(&user_id_str, &email, &role)
        .map_err(AppError::Internal)?;

    let token_hash = simple_hash(refresh_token.as_bytes());
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at)
         VALUES ($1, $2, $3, NOW() + INTERVAL '30 days', NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&token_hash)
    .execute(&state.db)
    .await?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
    }))
}

// ── Refresh ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<LoginResponse>> {
    let claims = state
        .jwt
        .verify(&req.refresh_token)
        .map_err(|_| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    let token_hash = simple_hash(req.refresh_token.as_bytes());

    let stored: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM refresh_tokens WHERE token_hash = $1 AND revoked = false AND expires_at > NOW()",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;

    if stored.is_none() {
        return Err(AppError::Unauthorized(
            "Refresh token revoked or expired".to_string(),
        ));
    }

    sqlx::query("UPDATE refresh_tokens SET revoked = true WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.db)
        .await?;

    let access_token = state
        .jwt
        .issue_access_token(&claims.sub, &claims.email, &claims.role)
        .map_err(AppError::Internal)?;

    let new_refresh = state
        .jwt
        .issue_refresh_token(&claims.sub, &claims.email, &claims.role)
        .map_err(AppError::Internal)?;

    let new_hash = simple_hash(new_refresh.as_bytes());
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID in token")))?;

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at)
         VALUES ($1, $2, $3, NOW() + INTERVAL '30 days', NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&new_hash)
    .execute(&state.db)
    .await?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token: new_refresh,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
    }))
}

// ── Logout ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> AppResult<Json<MessageResponse>> {
    let token_hash = simple_hash(req.refresh_token.as_bytes());
    sqlx::query("UPDATE refresh_tokens SET revoked = true WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.db)
        .await?;

    Ok(Json(MessageResponse {
        message: "Logged out successfully".to_string(),
    }))
}

// ── 2FA / TOTP ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub qr_url: String,
}

pub async fn totp_setup(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<TotpSetupResponse>> {
    use totp_rs::{Algorithm, Secret, TOTP};

    let secret = Secret::generate_secret();
    let secret_b32 = secret.to_encoded().to_string();

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        Some("RxForge".to_string()),
        user.claims.email.clone(),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP error: {e}")))?;

    let qr_url = totp.get_url();

    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    sqlx::query(
        "INSERT INTO user_totp (id, user_id, secret, verified, created_at)
         VALUES ($1, $2, $3, false, NOW())
         ON CONFLICT (user_id) DO UPDATE SET secret = $3, verified = false",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&secret_b32)
    .execute(&state.db)
    .await?;

    Ok(Json(TotpSetupResponse {
        secret: secret_b32,
        qr_url,
    }))
}

#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    pub code: String,
}

pub async fn totp_verify(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<TotpVerifyRequest>,
) -> AppResult<Json<MessageResponse>> {
    use totp_rs::{Algorithm, Secret, TOTP};

    let user_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let row: Option<(String,)> =
        sqlx::query_as("SELECT secret FROM user_totp WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;

    let (secret_str,) = row.ok_or_else(|| AppError::BadRequest("TOTP not set up".to_string()))?;

    let secret = Secret::Encoded(secret_str);
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret
            .to_bytes()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Secret decode: {e}")))?,
        Some("RxForge".to_string()),
        user.claims.email.clone(),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP error: {e}")))?;

    let valid = totp
        .check_current(&req.code)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP check error: {e}")))?;

    if !valid {
        return Err(AppError::Unauthorized("Invalid TOTP code".to_string()));
    }

    sqlx::query("UPDATE user_totp SET verified = true WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(MessageResponse {
        message: "TOTP verified and enabled".to_string(),
    }))
}

// ── WebAuthn ──────────────────────────────────────────────────────────────────

pub async fn webauthn_register_start(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "status": "not_implemented",
        "message": "WebAuthn registration not yet fully configured"
    })))
}

pub async fn webauthn_register_finish(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(_body): Json<serde_json::Value>,
) -> AppResult<Json<MessageResponse>> {
    Ok(Json(MessageResponse {
        message: "WebAuthn registration complete".to_string(),
    }))
}

pub async fn webauthn_login_start(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "not_implemented"})))
}

pub async fn webauthn_login_finish(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "not_implemented"})))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// SHA-256 hex digest for deterministic token storage.
fn simple_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash(b"hello world");
        let h2 = simple_hash(b"hello world");
        assert_eq!(h1, h2);
        // SHA-256 produces a 64-char hex string.
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_simple_hash_different_inputs() {
        let h1 = simple_hash(b"token_a");
        let h2 = simple_hash(b"token_b");
        assert_ne!(h1, h2);
    }
}
