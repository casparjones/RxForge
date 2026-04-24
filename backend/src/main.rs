use anyhow::Context;
use axum::middleware as axum_middleware;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rxforge_backend::{analytics, config::Config, couchdb::CouchDbClient, jwt::JwtManager, routes, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rxforge_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().context("Failed to load configuration")?;

    // Database pool
    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("Failed to run database migrations")?;

    tracing::info!("Database migrations applied");

    // JWT key manager (generates keys if missing)
    let jwt = JwtManager::load_or_generate(&config.jwt_private_key_path, &config.jwt_public_key_path)
        .context("Failed to initialize JWT manager")?;

    // CouchDB client
    let couchdb = CouchDbClient::new(&config.couchdb_url, &config.couchdb_user, &config.couchdb_password);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        jwt,
        couchdb,
    };

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Router
    let app = routes::api_router()
        .layer(axum_middleware::from_fn_with_state(
            db.clone(),
            |state: axum::extract::State<sqlx::PgPool>,
             req: axum::extract::Request,
             next: axum::middleware::Next| async move {
                analytics::track_request(state.0, req, next).await
            },
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    // Spawn background analytics aggregation
    tokio::spawn(analytics::run_daily_aggregation(db));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("RxForge backend listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind TCP listener")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
