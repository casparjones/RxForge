//! Integration tests for Ticket 4: App-Verwaltung endpoints.
//!
//! Covers create/list/get/delete and cross-tenant isolation (users cannot
//! read or delete another user's apps).

mod common;

use axum_test::TestServer;
use common::{docker_available, register_and_login, spawn_app};

async fn create_app(server: &TestServer, token: &str, name: &str) -> serde_json::Value {
    let resp = server
        .post("/api/v1/apps")
        .authorization_bearer(token)
        .json(&serde_json::json!({
            "name": name,
            "redirect_uris": ["https://example.com/cb"]
        }))
        .await;
    resp.assert_status_ok();
    resp.json()
}

#[tokio::test]
async fn authenticated_user_creates_app() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) = register_and_login(&server, "owner@example.com", "correcthorse").await;

    let body = create_app(&server, &token, "Test App").await;
    assert_eq!(body["name"], "Test App");
    assert!(body["id"].is_string());
    assert!(body["client_id"].as_str().unwrap().starts_with("rxf_"));
    assert!(body["client_secret"].is_string());

    // CouchDB db-prefix must have been reserved.
    let app_id = uuid::Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    let (prefix,): (String,) =
        sqlx::query_as("SELECT db_prefix FROM app_db_reservations WHERE app_id = $1")
            .bind(app_id)
            .fetch_one(&app.pool)
            .await
            .expect("reservation row");
    assert!(
        prefix.starts_with("app_"),
        "db_prefix should start with 'app_'"
    );
}

#[tokio::test]
async fn user_can_list_own_apps() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) = register_and_login(&server, "lister@example.com", "correcthorse").await;

    create_app(&server, &token, "App A").await;
    create_app(&server, &token, "App B").await;

    let resp = server
        .get("/api/v1/apps")
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let arr: Vec<serde_json::Value> = resp.json();
    assert_eq!(arr.len(), 2);
    let names: Vec<&str> = arr.iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"App A"));
    assert!(names.contains(&"App B"));
}

#[tokio::test]
async fn user_can_get_own_app_by_id() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) = register_and_login(&server, "getter@example.com", "correcthorse").await;
    let created = create_app(&server, &token, "Solo App").await;
    let id = created["id"].as_str().unwrap();

    let resp = server
        .get(&format!("/api/v1/apps/{id}"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"], id);
    assert_eq!(body["name"], "Solo App");
}

#[tokio::test]
async fn user_cannot_access_other_users_app() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_a_uid, a_token, _) = register_and_login(&server, "a@example.com", "correcthorse").await;
    let (_b_uid, b_token, _) = register_and_login(&server, "b@example.com", "correcthorse").await;

    let a_app = create_app(&server, &a_token, "A's App").await;
    let a_id = a_app["id"].as_str().unwrap();

    let resp = server
        .get(&format!("/api/v1/apps/{a_id}"))
        .authorization_bearer(&b_token)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn user_can_delete_own_app() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) = register_and_login(&server, "deleter@example.com", "correcthorse").await;
    let created = create_app(&server, &token, "Deletable").await;
    let id = created["id"].as_str().unwrap();

    let resp = server
        .delete(&format!("/api/v1/apps/{id}"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["deleted"], true);

    // Subsequent GET must return 404.
    let gone = server
        .get(&format!("/api/v1/apps/{id}"))
        .authorization_bearer(&token)
        .await;
    assert_eq!(gone.status_code(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn user_cannot_delete_other_users_app() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let app = spawn_app().await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_a_uid, a_token, _) =
        register_and_login(&server, "victim@example.com", "correcthorse").await;
    let (_b_uid, b_token, _) =
        register_and_login(&server, "thief@example.com", "correcthorse").await;

    let a_app = create_app(&server, &a_token, "Not yours").await;
    let a_id = a_app["id"].as_str().unwrap();

    let resp = server
        .delete(&format!("/api/v1/apps/{a_id}"))
        .authorization_bearer(&b_token)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::NOT_FOUND);

    // Row must still exist under the original owner.
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM apps WHERE id = $1")
        .bind(uuid::Uuid::parse_str(a_id).unwrap())
        .fetch_one(&app.pool)
        .await
        .expect("count");
    assert_eq!(count.0, 1);
}
