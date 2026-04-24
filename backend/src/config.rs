use anyhow::Context;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub couchdb_url: String,
    pub couchdb_user: String,
    pub couchdb_password: String,
    pub jwt_private_key_path: String,
    pub jwt_public_key_path: String,
    pub server_port: u16,
    pub frontend_dir: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            couchdb_url: env::var("COUCHDB_URL").context("COUCHDB_URL not set")?,
            couchdb_user: env::var("COUCHDB_USER").context("COUCHDB_USER not set")?,
            couchdb_password: env::var("COUCHDB_PASSWORD").context("COUCHDB_PASSWORD not set")?,
            jwt_private_key_path: env::var("JWT_PRIVATE_KEY_PATH")
                .unwrap_or_else(|_| "./keys/private.pem".to_string()),
            jwt_public_key_path: env::var("JWT_PUBLIC_KEY_PATH")
                .unwrap_or_else(|_| "./keys/public.pem".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
            frontend_dir: env::var("FRONTEND_DIR").unwrap_or_else(|_| "./frontend/build".to_string()),
        })
    }
}
