//! Integration tests for Ticket 1: Auth endpoints (register / login / refresh / logout).
//!
//! Spins up Postgres via testcontainers and drives the real axum router through
//! `axum_test::TestServer`. Tests are skipped with a log message if Docker is
//! unavailable – run with `cargo test -- --ignored` is not needed; they just
//! short-circuit on the Docker probe.

mod common;

use axum_test::TestServer;
use common::{docker_available, spawn_app};

#[tokio::test]
async fn register_returns_200_and_user_shape() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let resp = server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": "alice@example.com",
            "password": "correcthorse"
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["id"].is_string(), "expected string id");
    assert_eq!(body["email"], "alice@example.com");
}

#[tokio::test]
async fn register_with_existing_email_fails() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let payload = serde_json::json!({
        "email": "dup@example.com",
        "password": "correcthorse"
    });

    let first = server.post("/api/v1/auth/register").json(&payload).await;
    first.assert_status_ok();

    let second = server.post("/api/v1/auth/register").json(&payload).await;
    assert!(
        second.status_code().is_client_error(),
        "expected 4xx on duplicate email, got {}",
        second.status_code()
    );
}

#[tokio::test]
async fn login_with_correct_credentials_returns_jwt() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": "bob@example.com",
            "password": "correcthorse"
        }))
        .await
        .assert_status_ok();

    let login = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": "bob@example.com",
            "password": "correcthorse"
        }))
        .await;
    login.assert_status_ok();

    let body: serde_json::Value = login.json();
    let access = body["access_token"].as_str().expect("access_token");
    assert!(access.split('.').count() == 3, "JWT must have 3 segments");
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["refresh_token"].is_string());

    // Token must verify via the app's own JwtManager.
    let claims = app.state.jwt.verify(access).expect("verify access token");
    assert_eq!(claims.email, "bob@example.com");
    assert_eq!(claims.role, "user");
}

#[tokio::test]
async fn login_with_wrong_password_returns_401() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({
            "email": "carol@example.com",
            "password": "correcthorse"
        }))
        .await
        .assert_status_ok();

    let bad = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": "carol@example.com",
            "password": "wrongpassword"
        }))
        .await;
    assert_eq!(bad.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_returns_new_jwt() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, _access, refresh) =
        common::register_and_login(&server, "dave@example.com", "correcthorse").await;

    let resp = server
        .post("/api/v1/auth/refresh")
        .json(&serde_json::json!({"refresh_token": refresh}))
        .await;
    resp.assert_status_ok();

    let body: serde_json::Value = resp.json();
    let new_access = body["access_token"].as_str().expect("access_token");
    let claims = app
        .state
        .jwt
        .verify(new_access)
        .expect("verify refreshed token");
    assert_eq!(claims.email, "dave@example.com");
    assert!(body["refresh_token"].is_string());
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, _access, refresh) =
        common::register_and_login(&server, "erin@example.com", "correcthorse").await;

    server
        .post("/api/v1/auth/logout")
        .json(&serde_json::json!({"refresh_token": refresh.clone()}))
        .await
        .assert_status_ok();

    // Using the now-revoked refresh token must fail.
    let resp = server
        .post("/api/v1/auth/refresh")
        .json(&serde_json::json!({"refresh_token": refresh}))
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}
