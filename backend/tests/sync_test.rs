//! Integration tests for Ticket 3 (Round 3): RxDB sync endpoints.
//!
//! Uses wiremock to stand in for CouchDB. Exercises:
//!  * /pull without auth → 401
//!  * /pull with auth proxies `_changes` and shapes the response
//!  * /push with auth proxies `_bulk_docs` and surfaces conflicts
//!  * /stream returns `Content-Type: text/event-stream`

mod common;

use axum_test::TestServer;
use common::{docker_available, register_and_login, spawn_app_with_couchdb_url};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn create_app(server: &TestServer, token: &str) -> String {
    let resp = server
        .post("/api/v1/apps")
        .authorization_bearer(token)
        .json(&serde_json::json!({
            "name": "Sync App",
            "redirect_uris": ["https://example.com/cb"],
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn pull_without_auth_returns_401() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let app_id = uuid::Uuid::new_v4();
    let resp = server.get(&format!("/api/v1/sync/{app_id}/pull")).await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pull_forwards_to_couchdb_changes() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) = register_and_login(&server, "puller@example.com", "correcthorse").await;
    let app_id = create_app(&server, &token).await;

    // CouchDB _changes mock – returns two docs and a last_seq checkpoint.
    Mock::given(method("GET"))
        .and(path_regex(r"^/app_.+_user_.+/_changes$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "last_seq": "42-abc",
            "results": [
                {"id": "a", "seq": "1", "doc": {"_id": "a", "msg": "hello"}},
                {"id": "b", "seq": "2", "doc": {"_id": "b", "msg": "world"}},
            ],
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let resp = server
        .get(&format!("/api/v1/sync/{app_id}/pull"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let docs = body["documents"].as_array().expect("documents array");
    assert_eq!(docs.len(), 2);
    assert_eq!(body["checkpoint"], "42-abc");
}

#[tokio::test]
async fn push_forwards_bulk_docs_and_returns_conflicts() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) = register_and_login(&server, "pusher@example.com", "correcthorse").await;
    let app_id = create_app(&server, &token).await;

    // One OK, one conflict.
    Mock::given(method("POST"))
        .and(path_regex(r"^/app_.+_user_.+/_bulk_docs$"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([
            {"ok": true, "id": "doc-1", "rev": "1-aaa"},
            {"id": "doc-2", "error": "conflict", "reason": "Document update conflict."},
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    let resp = server
        .post(&format!("/api/v1/sync/{app_id}/push"))
        .authorization_bearer(&token)
        .json(&serde_json::json!({
            "documents": [
                {"_id": "doc-1", "msg": "new"},
                {"_id": "doc-2", "msg": "stale"},
            ]
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["written"], 1);
    let conflicts = body["conflicts"].as_array().expect("conflicts array");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0], "doc-2");
}

#[tokio::test]
async fn stream_returns_event_stream_content_type() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;

    // Provide *some* response so the stream handler connects cleanly;
    // we only care about the response headers that our sync layer emits.
    Mock::given(method("GET"))
        .and(path_regex(r"^/app_.+_user_.+/_changes$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\n"))
        .mount(&mock)
        .await;

    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, token, _) =
        register_and_login(&server, "streamer@example.com", "correcthorse").await;
    let app_id = create_app(&server, &token).await;

    let resp = server
        .get(&format!("/api/v1/sync/{app_id}/stream"))
        .authorization_bearer(&token)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .expect("content-type")
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("text/event-stream"),
        "expected SSE content-type, got {ct}"
    );
}
