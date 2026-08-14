//! Integration tests for the sync audit trail (`sync_events`).
//!
//! Covers the "which device did what" trail that a push produces:
//!  * writes, deletes and conflicts are logged with the reported device
//!  * the per-device summary aggregates them
//!  * the trail is scoped to the app owner

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
            "name": "Trail App",
            "redirect_uris": ["https://example.com/cb"],
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    body["id"].as_str().unwrap().to_string()
}

/// Events are written off the request path, so give them a moment to land.
async fn wait_for_events(
    server: &TestServer,
    token: &str,
    app_id: &str,
    expected: usize,
) -> serde_json::Value {
    for _ in 0..50 {
        let resp = server
            .get(&format!("/api/v1/apps/{app_id}/sync-events"))
            .authorization_bearer(token)
            .await;
        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        // Wait on `total` as well: count and page are separate statements, so a
        // batch landing between them would otherwise be observed half-written.
        let listed = body["events"].as_array().map_or(0, |a| a.len());
        let total = body["total"].as_u64().unwrap_or(0) as usize;
        if listed >= expected && total >= expected {
            return body;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("audit trail did not reach {expected} events in time");
}

/// Mount the CouchDB doubles a push needs: db provisioning, per-doc reads and
/// the bulk write. `doc-2` exists on the server so it produces a conflict.
async fn mount_push_mocks(mock: &MockServer) {
    Mock::given(method("PUT"))
        .and(path_regex(r"^/[^/]+$"))
        .respond_with(ResponseTemplate::new(412))
        .mount(mock)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/[^/]+/doc-1$"))
        .respond_with(ResponseTemplate::new(404))
        .mount(mock)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/[^/]+/doc-del$"))
        .respond_with(ResponseTemplate::new(404))
        .mount(mock)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/[^/]+/doc-2$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "_id": "doc-2", "_rev": "1-server", "msg": "server"
        })))
        .mount(mock)
        .await;

    Mock::given(method("POST"))
        .and(path_regex(r"^/[^/]+/_bulk_docs$"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!([
            {"ok": true, "id": "doc-1", "rev": "1-aaa"},
        ])))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn push_records_write_delete_and_conflict_per_device() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    mount_push_mocks(&mock).await;

    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");
    let (_uid, token, _) = register_and_login(&server, "trail@example.com", "correcthorse").await;
    let app_id = create_app(&server, &token).await;

    let resp = server
        .post(&format!("/api/v1/sync/{app_id}/push"))
        .authorization_bearer(&token)
        .json(&serde_json::json!({
            "client": {
                "deviceId": "device-phone",
                "deviceLabel": "iPhone · Safari",
                "platform": "iPhone · Safari",
                "appVersion": "1.9.0",
                "reason": "user-edit"
            },
            "rows": [
                {"newDocumentState": {"id": "doc-1", "msg": "new", "updatedAt": 1700000000000i64}},
                {"newDocumentState": {"id": "doc-del", "_deleted": true, "updatedAt": 1700000000001i64}},
                {"assumedMasterState": null, "newDocumentState": {"id": "doc-2", "msg": "stale"}},
            ]
        }))
        .await;
    resp.assert_status_ok();

    let body = wait_for_events(&server, &token, &app_id, 3).await;
    let events = body["events"].as_array().unwrap();
    assert_eq!(body["total"], 3);

    let op_of = |doc: &str| -> String {
        events
            .iter()
            .find(|e| e["doc_id"] == doc)
            .unwrap_or_else(|| panic!("no event for {doc}"))["op"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(op_of("doc-1"), "write");
    assert_eq!(op_of("doc-del"), "delete");
    assert_eq!(op_of("doc-2"), "conflict");

    // Device identity and trigger are carried through verbatim.
    let first = &events[0];
    assert_eq!(first["device_id"], "device-phone");
    assert_eq!(first["device_label"], "iPhone · Safari");
    assert_eq!(first["app_version"], "1.9.0");
    assert_eq!(first["reason"], "user-edit");
    assert_eq!(op_of("doc-1"), "write");

    // updatedAt from the document is preserved for the write.
    let doc1 = events.iter().find(|e| e["doc_id"] == "doc-1").unwrap();
    assert_eq!(doc1["doc_updated_at"], 1700000000000i64);

    // Per-device summary.
    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-devices"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let devices: serde_json::Value = resp.json();
    let devices = devices.as_array().unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0]["device_id"], "device-phone");
    assert_eq!(devices[0]["writes"], 1);
    assert_eq!(devices[0]["deletes"], 1);
    assert_eq!(devices[0]["conflicts"], 1);
}

#[tokio::test]
async fn device_identity_falls_back_to_headers() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    mount_push_mocks(&mock).await;

    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");
    let (_uid, token, _) = register_and_login(&server, "hdr@example.com", "correcthorse").await;
    let app_id = create_app(&server, &token).await;

    // No `client` block at all – a client that only sets headers must still
    // show up in the trail rather than as an anonymous entry.
    let resp = server
        .post(&format!("/api/v1/sync/{app_id}/push"))
        .authorization_bearer(&token)
        .add_header("X-RxForge-Device-Id", "device-from-header")
        .add_header("X-RxForge-Device-Label", "Laptop")
        .json(&serde_json::json!({
            "rows": [{"newDocumentState": {"id": "doc-1", "msg": "new"}}]
        }))
        .await;
    resp.assert_status_ok();

    let body = wait_for_events(&server, &token, &app_id, 1).await;
    assert_eq!(body["events"][0]["device_id"], "device-from-header");
    assert_eq!(body["events"][0]["device_label"], "Laptop");
}

#[tokio::test]
async fn trail_is_scoped_to_the_app_owner() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    mount_push_mocks(&mock).await;

    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");

    let (_uid, owner_token, _) =
        register_and_login(&server, "owner@example.com", "correcthorse").await;
    let app_id = create_app(&server, &owner_token).await;

    let (_uid2, stranger_token, _) =
        register_and_login(&server, "stranger@example.com", "correcthorse").await;

    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-events"))
        .authorization_bearer(&stranger_token)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::NOT_FOUND);

    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-devices"))
        .authorization_bearer(&stranger_token)
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::NOT_FOUND);

    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-events"))
        .await;
    assert_eq!(resp.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn events_can_be_filtered_by_device_and_operation() {
    if !docker_available() {
        eprintln!("Docker unavailable – skipping");
        return;
    }
    let mock = MockServer::start().await;
    mount_push_mocks(&mock).await;

    let app = spawn_app_with_couchdb_url(&mock.uri()).await;
    let server = TestServer::new(app.router.clone()).expect("test server");
    let (_uid, token, _) = register_and_login(&server, "filter@example.com", "correcthorse").await;
    let app_id = create_app(&server, &token).await;

    for device in ["device-a", "device-b"] {
        let resp = server
            .post(&format!("/api/v1/sync/{app_id}/push"))
            .authorization_bearer(&token)
            .json(&serde_json::json!({
                "client": { "deviceId": device },
                "rows": [
                    {"newDocumentState": {"id": "doc-1", "msg": "new"}},
                    {"newDocumentState": {"id": "doc-del", "_deleted": true}},
                ]
            }))
            .await;
        resp.assert_status_ok();
    }

    wait_for_events(&server, &token, &app_id, 4).await;

    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-events?device_id=device-a"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["total"], 2);

    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-events?op=delete"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["total"], 2);
    for e in body["events"].as_array().unwrap() {
        assert_eq!(e["op"], "delete");
    }

    let resp = server
        .get(&format!("/api/v1/apps/{app_id}/sync-events?doc_id=doc-1&op=write"))
        .authorization_bearer(&token)
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["total"], 2);
}
