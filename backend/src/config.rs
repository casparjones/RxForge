use anyhow::Context;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub couchdb_url: Option<String>,
    pub couchdb_user: Option<String>,
    pub couchdb_password: Option<String>,
    pub mongodb_url: Option<String>,
    pub mongodb_db: String,
    pub jwt_private_key_path: String,
    pub jwt_public_key_path: String,
    pub server_port: u16,
    pub frontend_dir: String,
    /// If set, registration requires this code.
    pub register_invite_code: Option<String>,
    /// If set, a user logging in with this email is auto-promoted to superadmin.
    pub admin_user_email: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            couchdb_url: env::var("COUCHDB_URL").ok(),
            couchdb_user: env::var("COUCHDB_USER").ok(),
            couchdb_password: env::var("COUCHDB_PASSWORD").ok(),
            mongodb_url: env::var("MONGODB_URL").ok(),
            mongodb_db: env::var("MONGODB_DB").unwrap_or_else(|_| "rxforge".to_string()),
            jwt_private_key_path: env::var("JWT_PRIVATE_KEY_PATH")
                .unwrap_or_else(|_| "./keys/private.pem".to_string()),
            jwt_public_key_path: env::var("JWT_PUBLIC_KEY_PATH")
                .unwrap_or_else(|_| "./keys/public.pem".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
            frontend_dir: env::var("FRONTEND_DIR").unwrap_or_else(|_| "./frontend/build".to_string()),
            register_invite_code: env::var("REGISTER_INVITE_CODE").ok(),
            admin_user_email: env::var("ADMIN_USER_EMAIL").ok(),
        })
    }
}
