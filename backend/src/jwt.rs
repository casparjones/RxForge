use anyhow::{Context, Result};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::{fs, path::Path};
use uuid::Uuid;

use crate::middleware::auth::{AppClaims, Claims};

#[derive(Clone)]
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    /// Load RSA key pair from PEM files. If the files don't exist, generate them.
    pub fn load_or_generate(private_key_path: &str, public_key_path: &str) -> Result<Self> {
        let private_path = Path::new(private_key_path);
        let public_path = Path::new(public_key_path);

        if !private_path.exists() || !public_path.exists() {
            tracing::info!("JWT keys not found, generating RSA key pair...");
            Self::generate_keys(private_key_path, public_key_path)?;
        }

        let private_pem = fs::read(private_path)
            .with_context(|| format!("Failed to read private key: {private_key_path}"))?;
        let public_pem = fs::read(public_path)
            .with_context(|| format!("Failed to read public key: {public_key_path}"))?;

        let encoding_key =
            EncodingKey::from_rsa_pem(&private_pem).context("Failed to load RSA private key")?;
        let decoding_key =
            DecodingKey::from_rsa_pem(&public_pem).context("Failed to load RSA public key")?;

        Ok(Self {
            encoding_key,
            decoding_key,
        })
    }

    fn generate_keys(private_key_path: &str, public_key_path: &str) -> Result<()> {
        use std::process::Command;

        let private_path = Path::new(private_key_path);
        if let Some(parent) = private_path.parent() {
            fs::create_dir_all(parent).context("Failed to create keys directory")?;
        }

        // Generate RSA private key (PKCS8)
        let output = Command::new("openssl")
            .args([
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-out",
                private_key_path,
            ])
            .output()
            .context("Failed to run openssl genpkey")?;

        if !output.status.success() {
            anyhow::bail!(
                "openssl genpkey failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Extract public key
        let output = Command::new("openssl")
            .args([
                "rsa",
                "-pubout",
                "-in",
                private_key_path,
                "-out",
                public_key_path,
            ])
            .output()
            .context("Failed to run openssl rsa")?;

        if !output.status.success() {
            anyhow::bail!(
                "openssl rsa failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        tracing::info!("RSA key pair generated successfully");
        Ok(())
    }

    /// Issue a new access token (1 hour TTL).
    pub fn issue_access_token(&self, user_id: &str, email: &str, role: &str) -> Result<String> {
        let now = Utc::now().timestamp();
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            iat: now,
            exp: now + 3600, // 1 hour
            jti: Uuid::new_v4().to_string(),
        };

        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .context("Failed to encode JWT")
    }

    /// Issue a refresh token (30 days TTL).
    pub fn issue_refresh_token(&self, user_id: &str, email: &str, role: &str) -> Result<String> {
        let now = Utc::now().timestamp();
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            iat: now,
            exp: now + 30 * 24 * 3600, // 30 days
            jti: Uuid::new_v4().to_string(),
        };

        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .context("Failed to encode refresh JWT")
    }

    /// Issue a 15-minute app-scoped JWT for a token exchange.
    pub fn issue_app_jwt(&self, token_id: &str, app_id: &str, user_id: &str) -> Result<String> {
        let now = Utc::now().timestamp();
        let claims = AppClaims {
            sub: token_id.to_string(),
            app_id: app_id.to_string(),
            user_id: user_id.to_string(),
            scope: "sync:write".to_string(),
            iat: now,
            exp: now + 3600, // 1 hour
            jti: Uuid::new_v4().to_string(),
        };
        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .context("Failed to encode app JWT")
    }

    /// Verify and decode an app-scoped JWT (from token exchange).
    pub fn verify_app_jwt(&self, token: &str) -> Result<AppClaims> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        let token_data = decode::<AppClaims>(token, &self.decoding_key, &validation)
            .context("App JWT verification failed")?;
        Ok(token_data.claims)
    }

    /// Verify and decode a token.
    pub fn verify(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .context("JWT verification failed")?;

        Ok(token_data.claims)
    }

    /// Get the public key PEM for CouchDB JWT configuration.
    pub fn public_key_pem(&self) -> Option<String> {
        None // Handled by reading public key file directly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_jwt_generate_and_verify() {
        let dir = TempDir::new().unwrap();
        let private = dir.path().join("private.pem");
        let public = dir.path().join("public.pem");

        let manager =
            JwtManager::load_or_generate(private.to_str().unwrap(), public.to_str().unwrap())
                .unwrap();

        let token = manager
            .issue_access_token("user-123", "test@example.com", "user")
            .unwrap();

        let claims = manager.verify(&token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, "user");
    }

    #[test]
    fn test_jwt_invalid_token() {
        let dir = TempDir::new().unwrap();
        let private = dir.path().join("private.pem");
        let public = dir.path().join("public.pem");

        let manager =
            JwtManager::load_or_generate(private.to_str().unwrap(), public.to_str().unwrap())
                .unwrap();

        let result = manager.verify("invalid.token.here");
        assert!(result.is_err());
    }
}
