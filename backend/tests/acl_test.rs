//! Integration tests for Ticket 2: Roles & ACL.
//!
//! Exercises the permission gate on `POST /api/v1/apps`, the admin grant
//! endpoint `PATCH /api/v1/admin/users/:id/role`, and rejection of grant
//! attempts by non-admins.

mod common;

use axum_test::TestServer;
use common::{docker_available, promote_user, register_and_login, spawn_app};

/// Forge an access token with an arbitrary role, bypassing the DB CHECK
/// constraint on `users.role`. Used to simulate a user whose effective role
/// does NOT grant `apps:create` (e.g. a future `readonly` role).
fn forge_token(app: &common::TestApp, user_id: &str, email: &str, role: &str) -> String {
    app.state
        .jwt
        .issue_access_token(user_id, email, role)
        .expect("issue token")
}

#[tokio::test]
async fn user_without_apps_create_gets_403_on_post_apps() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (uid, _access, _refresh) =
        register_and_login(&server, "readonly@example.com", "correcthorse").await;

    // Issue a token whose role isn't in the {user, admin, superadmin} allow-list
    // for `apps:create` – `require_permission` must reject with 403.
    let token = forge_token(&app, &uid, "readonly@example.com", "readonly");

    let resp = server
        .post("/api/v1/apps")
        .authorization_bearer(&token)
        .json(&serde_json::json!({
            "name": "My App",
            "redirect_uris": ["https://example.com/cb"]
        }))
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_grant_role_via_admin_endpoint() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    // Register an admin + a target user.
    let (admin_uid, _, _) = register_and_login(&server, "admin@example.com", "correcthorse").await;
    promote_user(&app.pool, &admin_uid, "admin").await;
    // Re-login to pick up the new role claim.
    let login = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "correcthorse"
        }))
        .await;
    let body: serde_json::Value = login.json();
    let admin_token = body["access_token"].as_str().unwrap().to_string();

    let (target_uid, _, _) = register_and_login(&server, "target@example.com", "correcthorse").await;

    let resp = server
        .patch(&format!("/api/v1/admin/users/{target_uid}/role"))
        .authorization_bearer(&admin_token)
        .json(&serde_json::json!({"role": "admin"}))
        .await;

    resp.assert_status_ok();

    // DB state should reflect the update.
    let role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&target_uid).unwrap())
        .fetch_one(&app.pool)
        .await
        .expect("fetch updated role");
    assert_eq!(role.0, "admin");
}

#[tokio::test]
async fn non_admin_cannot_grant_role() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_actor_uid, actor_token, _) =
        register_and_login(&server, "actor@example.com", "correcthorse").await;
    let (target_uid, _, _) = register_and_login(&server, "victim@example.com", "correcthorse").await;

    let resp = server
        .patch(&format!("/api/v1/admin/users/{target_uid}/role"))
        .authorization_bearer(&actor_token)
        .json(&serde_json::json!({"role": "admin"}))
        .await;

    assert_eq!(resp.status_code(), axum::http::StatusCode::FORBIDDEN);
}
