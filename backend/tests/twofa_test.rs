//! Integration tests for Ticket 3: 2FA (TOTP) & WebAuthn shape.
//!
//! For passkey endpoints we only check that they are wired up and return a
//! JSON body – full WebAuthn ceremonies require a browser context and are
//! out of scope.

mod common;

use axum_test::TestServer;
use common::{docker_available, register_and_login, spawn_app};
use totp_rs::{Algorithm, Secret, TOTP};

fn current_totp(secret_b32: &str, email: &str) -> String {
    let secret = Secret::Encoded(secret_b32.to_string());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().expect("decode"),
        Some("RxForge".to_string()),
        email.to_string(),
    )
    .expect("totp");
    totp.generate_current().expect("code")
}

#[tokio::test]
async fn totp_setup_returns_secret_and_otpauth_url() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, access, _refresh) =
        register_and_login(&server, "totp@example.com", "correcthorse").await;

    let resp = server
        .post("/api/v1/auth/2fa/setup")
        .authorization_bearer(&access)
        .await;
    resp.assert_status_ok();

    let body: serde_json::Value = resp.json();
    let secret = body["secret"].as_str().expect("secret");
    let qr_url = body["qr_url"].as_str().expect("qr_url");
    assert!(!secret.is_empty());
    assert!(qr_url.starts_with("otpauth://totp/"), "expected otpauth URL, got {qr_url}");
}

#[tokio::test]
async fn totp_enable_with_correct_code_succeeds() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let email = "totp-ok@example.com";
    let (_uid, access, _refresh) = register_and_login(&server, email, "correcthorse").await;

    let setup = server
        .post("/api/v1/auth/2fa/setup")
        .authorization_bearer(&access)
        .await;
    setup.assert_status_ok();
    let body: serde_json::Value = setup.json();
    let secret = body["secret"].as_str().unwrap();
    let code = current_totp(secret, email);

    let verify = server
        .post("/api/v1/auth/2fa/verify")
        .authorization_bearer(&access)
        .json(&serde_json::json!({"code": code}))
        .await;
    verify.assert_status_ok();
}

#[tokio::test]
async fn totp_enable_with_wrong_code_fails() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, access, _refresh) =
        register_and_login(&server, "totp-bad@example.com", "correcthorse").await;

    server
        .post("/api/v1/auth/2fa/setup")
        .authorization_bearer(&access)
        .await
        .assert_status_ok();

    let resp = server
        .post("/api/v1/auth/2fa/verify")
        .authorization_bearer(&access)
        .json(&serde_json::json!({"code": "000000"}))
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_2fa_enabled_requires_code() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let email = "totp-login@example.com";
    let password = "correcthorse";
    let (_uid, access, _refresh) = register_and_login(&server, email, password).await;

    // Enable TOTP for this user.
    let setup = server
        .post("/api/v1/auth/2fa/setup")
        .authorization_bearer(&access)
        .await;
    let body: serde_json::Value = setup.json();
    let secret = body["secret"].as_str().unwrap().to_string();
    let code = current_totp(&secret, email);
    server
        .post("/api/v1/auth/2fa/verify")
        .authorization_bearer(&access)
        .json(&serde_json::json!({"code": code}))
        .await
        .assert_status_ok();

    // Login without a TOTP code must now fail.
    let bad = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({"email": email, "password": password}))
        .await;
    assert_eq!(bad.status_code(), axum::http::StatusCode::UNAUTHORIZED);

    // Login WITH the right code must succeed.
    let code2 = current_totp(&secret, email);
    let ok = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "totp_code": code2
        }))
        .await;
    ok.assert_status_ok();
}

#[tokio::test]
async fn passkey_endpoints_exist_and_return_shapes() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, access, _refresh) =
        register_and_login(&server, "pk@example.com", "correcthorse").await;

    let start = server
        .post("/api/v1/auth/webauthn/register-start")
        .authorization_bearer(&access)
        .await;
    start.assert_status_ok();
    let body: serde_json::Value = start.json();
    assert!(body.is_object(), "register-start must return a JSON object");

    let login_start = server
        .post("/api/v1/auth/webauthn/login-start")
        .json(&serde_json::json!({"email": "pk@example.com"}))
        .await;
    login_start.assert_status_ok();
    let body: serde_json::Value = login_start.json();
    assert!(body.is_object(), "login-start must return a JSON object");
}
