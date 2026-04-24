//! Shared test harness: spins up Postgres via testcontainers, applies migrations,
//! and builds a ready-to-use axum Router with AppState + real JwtManager.
//!
//! Integration tests skip with a log message if Docker is unavailable.

#![allow(dead_code)]

use axum::Router;
use rxforge_backend::{
    analytics, config::Config, couchdb::CouchDbClient, jwt::JwtManager, routes, state::AppState,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tempfile::TempDir;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt},
};

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    pub pool: PgPool,
    // Keep container + tempdir alive for the duration of the test.
    _container: ContainerAsync<Postgres>,
    _keys_dir: TempDir,
}

pub fn docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Spin up Postgres, run migrations, build an AppState with a real JWT manager
/// (on-the-fly RSA keys in a tempdir), and return a finalized `Router` ready to
/// hand to `axum_test::TestServer`.
pub async fn spawn_app() -> TestApp {
    spawn_app_with_couchdb_url("http://localhost:5984").await
}

/// Same as [`spawn_app`] but lets the caller point the `CouchDbClient` at a
/// custom URL (e.g. a `wiremock::MockServer::uri()`).
pub async fn spawn_app_with_couchdb_url(couchdb_url: &str) -> TestApp {
    let container = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await
        .expect("failed to start postgres container");

    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("failed to get mapped port");

    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to test postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations must apply cleanly");

    let keys_dir = TempDir::new().expect("tempdir");
    let private = keys_dir.path().join("private.pem");
    let public = keys_dir.path().join("public.pem");

    let jwt = JwtManager::load_or_generate(
        private.to_str().unwrap(),
        public.to_str().unwrap(),
    )
    .expect("jwt manager");

    let config = Config {
        database_url: database_url.clone(),
        couchdb_url: couchdb_url.to_string(),
        couchdb_user: "admin".to_string(),
        couchdb_password: "password".to_string(),
        jwt_private_key_path: private.to_string_lossy().to_string(),
        jwt_public_key_path: public.to_string_lossy().to_string(),
        server_port: 0,
        frontend_dir: "./dist".to_string(),
    };

    let couchdb = CouchDbClient::new(&config.couchdb_url, &config.couchdb_user, &config.couchdb_password);

    let state = AppState {
        db: pool.clone(),
        config,
        jwt,
        couchdb,
    };

    let router = routes::api_router()
        .layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            |axum::extract::State(pool): axum::extract::State<sqlx::PgPool>,
             req: axum::extract::Request,
             next: axum::middleware::Next| async move {
                analytics::track_request(pool, req, next).await
            },
        ))
        .with_state(state.clone());

    TestApp {
        router,
        state,
        pool,
        _container: container,
        _keys_dir: keys_dir,
    }
}

/// Register + login a user, returning (user_id, access_token, refresh_token).
pub async fn register_and_login(
    server: &axum_test::TestServer,
    email: &str,
    password: &str,
) -> (String, String, String) {
    let reg = server
        .post("/api/v1/auth/register")
        .json(&serde_json::json!({"email": email, "password": password}))
        .await;
    reg.assert_status_ok();
    let reg_body: serde_json::Value = reg.json();
    let user_id = reg_body["id"].as_str().unwrap().to_string();

    let login = server
        .post("/api/v1/auth/login")
        .json(&serde_json::json!({"email": email, "password": password}))
        .await;
    login.assert_status_ok();
    let body: serde_json::Value = login.json();

    (
        user_id,
        body["access_token"].as_str().unwrap().to_string(),
        body["refresh_token"].as_str().unwrap().to_string(),
    )
}

/// Promote a user to `role` directly via SQL (admin, superadmin, …).
pub async fn promote_user(pool: &PgPool, user_id: &str, role: &str) {
    let uid = uuid::Uuid::parse_str(user_id).expect("valid user uuid");
    sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
        .bind(role)
        .bind(uid)
        .execute(pool)
        .await
        .expect("promote user");
}
