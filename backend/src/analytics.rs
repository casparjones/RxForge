use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

/// Axum middleware that logs every request into the analytics_events table.
pub async fn track_request(
    pool: sqlx::Pool<sqlx::Postgres>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    let duration_ms = start.elapsed().as_millis() as f64;
    let status_code = response.status().as_u16() as i32;

    // Fire-and-forget analytics insert
    tokio::spawn(async move {
        let result = sqlx::query(
            "INSERT INTO analytics_events (id, path, method, status_code, duration_ms, app_id, user_id, event_type, docs_count, created_at)
             VALUES ($1, $2, $3, $4, $5, NULL, NULL, 'request', 0, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(&path)
        .bind(&method)
        .bind(status_code)
        .bind(duration_ms)
        .execute(&pool)
        .await;

        if let Err(e) = result {
            tracing::warn!("Failed to log analytics event: {e}");
        }
    });

    response
}

/// Background task: run daily aggregation of analytics data.
/// Call with tokio::spawn.
pub async fn run_daily_aggregation(pool: PgPool) {
    use tokio::time::{interval, Duration};

    let mut ticker = interval(Duration::from_secs(24 * 3600));
    loop {
        ticker.tick().await;
        tracing::info!("Running daily analytics aggregation...");

        let result = sqlx::query(
            "INSERT INTO analytics_daily_aggregates (id, date, total_requests, unique_users, created_at)
             SELECT
                 gen_random_uuid(),
                 CURRENT_DATE - INTERVAL '1 day',
                 COUNT(*),
                 COUNT(DISTINCT user_id),
                 NOW()
             FROM analytics_events
             WHERE DATE(created_at) = CURRENT_DATE - INTERVAL '1 day'
             ON CONFLICT (date) DO UPDATE SET
                 total_requests = EXCLUDED.total_requests,
                 unique_users = EXCLUDED.unique_users",
        )
        .execute(&pool)
        .await;

        match result {
            Ok(_) => tracing::info!("Daily analytics aggregation complete"),
            Err(e) => tracing::error!("Daily aggregation failed: {e}"),
        }
    }
}
