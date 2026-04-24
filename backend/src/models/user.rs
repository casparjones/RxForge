use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateUser {
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

impl User {
    pub async fn create(pool: &PgPool, input: CreateUser) -> AppResult<Self> {
        let id = Uuid::new_v4();
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW())
             RETURNING *",
        )
        .bind(id)
        .bind(&input.email)
        .bind(&input.password_hash)
        .bind(&input.role)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> AppResult<Option<Self>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<Self>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    pub async fn update_role(pool: &PgPool, id: Uuid, role: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
            .bind(role)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        match self.role.as_str() {
            "superadmin" => true,
            "admin" => !matches!(permission, "superadmin:*"),
            "user" => matches!(permission, "apps:create" | "apps:read" | "apps:delete"),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bcrypt::{hash, verify, DEFAULT_COST};

    fn make_user(role: &str) -> User {
        User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            role: role.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_superadmin_has_all_permissions() {
        let user = make_user("superadmin");
        assert!(user.has_permission("apps:create"));
        assert!(user.has_permission("users:manage"));
        assert!(user.has_permission("analytics:read"));
    }

    #[test]
    fn test_admin_permissions() {
        let user = make_user("admin");
        assert!(user.has_permission("apps:create"));
        assert!(user.has_permission("users:manage"));
        // superadmin-only permissions not granted
        assert!(!user.has_permission("superadmin:*"));
    }

    #[test]
    fn test_user_permissions() {
        let user = make_user("user");
        assert!(user.has_permission("apps:create"));
        assert!(user.has_permission("apps:read"));
        assert!(!user.has_permission("users:manage"));
        assert!(!user.has_permission("analytics:read"));
    }

    // ── bcrypt password hashing (Task B) ─────────────────────────────────────

    #[test]
    fn bcrypt_hash_verifies_correct_password() {
        let password = "s3cret-passw0rd!";
        let h = hash(password, DEFAULT_COST).expect("hash should succeed");
        assert!(h.starts_with("$2"), "bcrypt hash must start with $2");
        assert!(verify(password, &h).expect("verify should succeed"));
    }

    #[test]
    fn bcrypt_rejects_wrong_password() {
        let h = hash("correct-horse-battery-staple", DEFAULT_COST).unwrap();
        assert!(!verify("tr0ub4dor&3", &h).unwrap());
    }

    #[test]
    fn bcrypt_each_hash_has_unique_salt() {
        let pw = "same-password";
        let h1 = hash(pw, DEFAULT_COST).unwrap();
        let h2 = hash(pw, DEFAULT_COST).unwrap();
        assert_ne!(h1, h2, "bcrypt must produce a unique salt per hash");
        assert!(verify(pw, &h1).unwrap());
        assert!(verify(pw, &h2).unwrap());
    }
}
