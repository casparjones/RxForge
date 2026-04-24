//! Integration tests for Ticket 2 (Round 3): CouchDB auto-provisioning.
//!
//! Uses wiremock to stand in for a real CouchDB server. Verifies:
//!  * provision_user_db issues `PUT /{db_name}`
//!  * 412 Precondition Failed (db already exists) is treated as success
//!  * configure_jwt_auth issues `PUT /_node/_local/_config/jwt_auth/...`

use rxforge_backend::couchdb::CouchDbClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn provision_user_db_creates_db_via_put() {
    let server = MockServer::start().await;
    let db_name = "app_abc123_user_def456";

    Mock::given(method("PUT"))
        .and(path(format!("/{db_name}")))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"ok": true})))
        .expect(1)
        .mount(&server)
        .await;

    let client = CouchDbClient::new(&server.uri(), "admin", "password");
    let db = client
        .provision_user_db("abc123", "def456")
        .await
        .expect("provision_user_db must succeed");
    assert_eq!(db, db_name);
}

#[tokio::test]
async fn provision_user_db_idempotent_on_412() {
    let server = MockServer::start().await;
    let db_name = "app_dup_user_dup";

    Mock::given(method("PUT"))
        .and(path(format!("/{db_name}")))
        .respond_with(ResponseTemplate::new(412).set_body_json(serde_json::json!({
            "error": "file_exists",
            "reason": "The database could not be created, the file already exists."
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = CouchDbClient::new(&server.uri(), "admin", "password");
    client
        .provision_user_db("dup", "dup")
        .await
        .expect("412 exists must be treated as success");
}

#[tokio::test]
async fn configure_jwt_auth_writes_config() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/_node/_local/_config/jwt_keys/rsa:rxforge"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\"\""))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/_node/_local/_config/jwt_auth/required_claims"))
        .respond_with(ResponseTemplate::new(200).set_body_string("\"\""))
        .expect(1)
        .mount(&server)
        .await;

    let client = CouchDbClient::new(&server.uri(), "admin", "password");
    let pem = "-----BEGIN PUBLIC KEY-----\nFAKE\n-----END PUBLIC KEY-----\n";
    client
        .configure_jwt_auth(pem)
        .await
        .expect("configure_jwt_auth must succeed");
}

#[tokio::test]
async fn provision_db_returns_error_on_5xx() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/app_err_user_err"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "internal"
        })))
        .mount(&server)
        .await;

    let client = CouchDbClient::new(&server.uri(), "admin", "password");
    let result = client.provision_user_db("err", "err").await;
    assert!(result.is_err(), "5xx must surface as error");
}
