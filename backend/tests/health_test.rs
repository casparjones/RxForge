use axum::{routing::get, Router};
use axum_test::TestServer;

use rxforge_backend::routes::health_check;

/// Task A acceptance test — starts the axum app and checks `/health` returns 200.
#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app: Router = Router::new().route("/health", get(health_check));

    let server = TestServer::new(app).expect("failed to start test server");

    let response = server.get("/health").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "rxforge-backend");
}
