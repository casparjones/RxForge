//! Integration tests for Ticket 4 (Round 3): Analytics tracking + aggregation.

mod common;

use axum_test::TestServer;
use common::{docker_available, promote_user, register_and_login, spawn_app};

#[tokio::test]
async fn middleware_inserts_analytics_event_on_request() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    // Hit the health endpoint – fire-and-forget insert happens in background.
    server.get("/health").await.assert_status_ok();

    // Poll until at least one row for /health appears (or give up).
    let mut rows: i64 = 0;
    for _ in 0..50 {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics_events WHERE path = '/health'",
        )
        .fetch_one(&app.pool)
        .await
        .unwrap();
        rows = c;
        if rows > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(rows > 0, "analytics_events row for /health not recorded");
}

#[tokio::test]
async fn global_analytics_requires_admin_role() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, user_token, _) =
        register_and_login(&server, "u@example.com", "correcthorse").await;

    let denied = server
        .get("/api/v1/analytics/global")
        .authorization_bearer(&user_token)
        .await;
    assert_eq!(denied.status_code(), axum::http::StatusCode::FORBIDDEN);

    // Promote and re-login.
    let (admin_uid, _, _) =
        register_and_login(&server, "ad@example.com", "correcthorse").await;
    promote_user(&app.pool, &admin_uid, "admin").await;
    let login = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({
            "email": "ad@example.com",
            "password": "correcthorse"
        }))
        .await;
    let body: serde_json::Value = login.json();
    let admin_token = body["access_token"].as_str().unwrap().to_string();

    let ok = server
        .get("/api/v1/analytics/global")
        .authorization_bearer(&admin_token)
        .await;
    ok.assert_status_ok();
    let payload: serde_json::Value = ok.json();
    assert!(payload["total_requests"].is_i64());
    assert!(payload["total_users"].is_i64());
    assert!(payload["total_apps"].is_i64());
}

#[tokio::test]
async fn app_analytics_owner_only_non_owner_forbidden() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_owner_uid, owner_token, _) =
        register_and_login(&server, "owner2@example.com", "correcthorse").await;
    let (_other_uid, other_token, _) =
        register_and_login(&server, "other@example.com", "correcthorse").await;

    // Create an app as owner.
    let resp = server
        .post("/api/v1/apps")
        .authorization_bearer(&owner_token)
        .json(&serde_json::json!({
            "name": "Analytics App",
            "redirect_uris": ["https://example.com/cb"]
        }))
        .await;
    resp.assert_status_ok();
    let app_id = resp.json::<serde_json::Value>()["id"].as_str().unwrap().to_string();

    // Owner: OK.
    let mine = server
        .get(&format!("/api/v1/analytics/apps/{app_id}"))
        .authorization_bearer(&owner_token)
        .await;
    mine.assert_status_ok();
    let payload: serde_json::Value = mine.json();
    assert_eq!(payload["app_id"], app_id);

    // Non-owner: must not be able to read foreign app analytics.
    let forbidden = server
        .get(&format!("/api/v1/analytics/apps/{app_id}"))
        .authorization_bearer(&other_token)
        .await;
    let status = forbidden.status_code();
    assert!(
        status == axum::http::StatusCode::FORBIDDEN
            || status == axum::http::StatusCode::NOT_FOUND,
        "non-owner must be denied (403) or hidden (404), got {status}"
    );
}
