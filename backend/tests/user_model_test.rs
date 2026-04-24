//! Integration test for the user data model (Task B).
//!
//! Spins up a throw-away Postgres via testcontainers, runs all migrations,
//! then inserts a bcrypt-hashed user and reads it back.
//!
//! Marked `#[ignore]` if the Docker daemon is unreachable – run with
//! `cargo test --manifest-path backend/Cargo.toml -- --ignored` to force.

use bcrypt::{hash, verify, DEFAULT_COST};
use rxforge_backend::models::user::{CreateUser, User};
use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{runners::AsyncRunner, ImageExt},
};

async fn docker_available() -> bool {
    // Rough probe: try to create the image spec; actual start happens below.
    std::process::Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn user_insert_and_read_roundtrip() {
    if !docker_available().await {
        eprintln!("Docker not available – skipping testcontainers test");
        return;
    }

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

    // Sanity check — core tables must exist.
    for table in &["users", "roles", "permissions", "user_roles"] {
        let (exists,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = $1)",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("table existence query");
        assert!(exists, "expected table '{table}' to exist after migrations");
    }

    let password = "correct horse battery staple";
    let password_hash = hash(password, DEFAULT_COST).expect("bcrypt hash");

    let created = User::create(
        &pool,
        CreateUser {
            email: "felix@example.com".into(),
            password_hash: password_hash.clone(),
            role: "user".into(),
        },
    )
    .await
    .expect("user insert should succeed");

    assert_eq!(created.email, "felix@example.com");
    assert_eq!(created.role, "user");

    let fetched = User::find_by_email(&pool, "felix@example.com")
        .await
        .expect("lookup should succeed")
        .expect("user should exist");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.email, created.email);
    assert!(
        verify(password, &fetched.password_hash).expect("verify"),
        "stored hash must verify against the original password",
    );

    let by_id = User::find_by_id(&pool, created.id)
        .await
        .expect("find_by_id")
        .expect("user should exist by id");
    assert_eq!(by_id.email, "felix@example.com");
}
