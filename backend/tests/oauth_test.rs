//! Integration tests for Ticket 1 (Round 3): OAuth 2.0 Authorization Server.
//!
//! Covers client_credentials flow, authorization_code flow, and token revocation.

mod common;

use axum_test::TestServer;
use common::{docker_available, register_and_login, spawn_app};

/// Helper: create an app (via POST /api/v1/apps) and return (client_id, client_secret, app_id).
async fn create_oauth_app(
    server: &TestServer,
    token: &str,
    redirect_uri: &str,
) -> (String, String, String) {
    let resp = server
        .post("/api/v1/apps")
        .authorization_bearer(token)
        .json(&serde_json::json!({
            "name": "OAuth Test App",
            "redirect_uris": [redirect_uri],
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    (
        body["client_id"].as_str().unwrap().to_string(),
        body["client_secret"].as_str().unwrap().to_string(),
        body["id"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn client_credentials_issues_access_token() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, user_token, _) = register_and_login(&server, "cc@example.com", "correcthorse").await;
    let (client_id, client_secret, _app_id) =
        create_oauth_app(&server, &user_token, "https://example.com/cb").await;

    let resp = server
        .post("/oauth/token")
        .form(&serde_json::json!({
            "grant_type": "client_credentials",
            "client_id": client_id,
            "client_secret": client_secret,
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["access_token"].is_string());
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 3600);
}

#[tokio::test]
async fn client_credentials_wrong_secret_returns_401() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, user_token, _) =
        register_and_login(&server, "cc2@example.com", "correcthorse").await;
    let (client_id, _secret, _app_id) =
        create_oauth_app(&server, &user_token, "https://example.com/cb").await;

    let resp = server
        .post("/oauth/token")
        .form(&serde_json::json!({
            "grant_type": "client_credentials",
            "client_id": client_id,
            "client_secret": "totally-wrong-secret",
        }))
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authorization_code_flow_full_roundtrip() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, user_token, _) = register_and_login(&server, "ac@example.com", "correcthorse").await;
    let redirect_uri = "https://example.com/cb";
    let (client_id, client_secret, _app_id) =
        create_oauth_app(&server, &user_token, redirect_uri).await;

    // Step 1: /oauth/authorize → 302 with ?code=...
    let auth_resp = server
        .get("/oauth/authorize")
        .add_query_params(&[
            ("response_type", "code"),
            ("client_id", &client_id),
            ("redirect_uri", redirect_uri),
            ("state", "xyz"),
        ])
        .await;
    assert!(
        auth_resp.status_code().is_redirection(),
        "expected 3xx redirect, got {}",
        auth_resp.status_code()
    );
    let location = auth_resp
        .headers()
        .get("location")
        .expect("Location header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(location.starts_with(redirect_uri));
    assert!(location.contains("code="));
    assert!(location.contains("state=xyz"));

    // Extract code param from query string.
    let code = location
        .split_once("code=")
        .unwrap()
        .1
        .split('&')
        .next()
        .unwrap()
        .to_string();

    // Step 2: /oauth/token with the code → access_token.
    let token_resp = server
        .post("/oauth/token")
        .form(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
            "client_id": client_id,
            "client_secret": client_secret,
        }))
        .await;
    token_resp.assert_status_ok();
    let body: serde_json::Value = token_resp.json();
    assert!(body["access_token"].is_string());
    assert_eq!(body["token_type"], "Bearer");
}

#[tokio::test]
async fn revoke_marks_token_revoked() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, user_token, _) =
        register_and_login(&server, "rev@example.com", "correcthorse").await;
    let (client_id, client_secret, _app_id) =
        create_oauth_app(&server, &user_token, "https://example.com/cb").await;

    let token_resp = server
        .post("/oauth/token")
        .form(&serde_json::json!({
            "grant_type": "client_credentials",
            "client_id": client_id,
            "client_secret": client_secret,
        }))
        .await;
    token_resp.assert_status_ok();
    let access: String = token_resp.json::<serde_json::Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    let rev = server
        .post("/oauth/revoke")
        .form(&serde_json::json!({ "token": access }))
        .await;
    rev.assert_status_ok();

    let (revoked,): (bool,) = sqlx::query_as("SELECT revoked FROM oauth_tokens WHERE token = $1")
        .bind(&access)
        .fetch_one(&app.pool)
        .await
        .expect("token row must exist");
    assert!(revoked, "token must be marked revoked after /oauth/revoke");
}
