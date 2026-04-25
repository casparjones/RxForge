use axum::{extract::Request, middleware::Next, response::Response};
use sqlx::PgPool;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct AnalyticsEvent {
    pub id: Uuid,
    pub path: String,
    pub method: String,
    pub status_code: i32,
    pub duration_ms: f64,
}

pub type AnalyticsSender = mpsc::Sender<AnalyticsEvent>;

/// Static-asset extensions that are not worth tracking in analytics.
fn is_static_asset(path: &str) -> bool {
    let ext_start = path.rfind('.').unwrap_or(0);
    matches!(
        &path[ext_start..],
        ".js" | ".css" | ".png" | ".jpg" | ".jpeg" | ".svg" | ".ico"
            | ".woff" | ".woff2" | ".ttf" | ".eot" | ".map" | ".webp"
            | ".avif" | ".gif" | ".json" | ".txt" | ".xml"
    )
}

/// Axum middleware: captures timing, sends to the in-memory channel (non-blocking).
pub async fn track_request(
    sender: AnalyticsSender,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    if !is_static_asset(&path) {
        let event = AnalyticsEvent {
            id: Uuid::new_v4(),
            path,
            method,
            status_code: response.status().as_u16() as i32,
            duration_ms: start.elapsed().as_millis() as f64,
        };
        // try_send: if the channel is full, drop the event rather than blocking
        let _ = sender.try_send(event);
    }

    response
}

/// Background task: drains the channel and batch-inserts into the DB every 5 s or every 200 events.
/// Call with tokio::spawn at startup.
pub fn start_analytics_writer(pool: PgPool) -> AnalyticsSender {
    let (tx, mut rx) = mpsc::channel::<AnalyticsEvent>(10_000);

    tokio::spawn(async move {
        use tokio::time::{interval, Duration};

        let mut ticker = interval(Duration::from_secs(5));
        let mut batch: Vec<AnalyticsEvent> = Vec::with_capacity(200);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if !batch.is_empty() {
                        flush(&pool, &mut batch).await;
                    }
                }
                maybe = rx.recv() => {
                    match maybe {
                        Some(ev) => {
                            batch.push(ev);
                            if batch.len() >= 200 {
                                flush(&pool, &mut batch).await;
                            }
                        }
                        None => {
                            // Channel closed — flush remaining and exit
                            if !batch.is_empty() {
                                flush(&pool, &mut batch).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    tx
}

async fn flush(pool: &PgPool, batch: &mut Vec<AnalyticsEvent>) {
    let ids:      Vec<Uuid>   = batch.iter().map(|e| e.id).collect();
    let paths:    Vec<&str>   = batch.iter().map(|e| e.path.as_str()).collect();
    let methods:  Vec<&str>   = batch.iter().map(|e| e.method.as_str()).collect();
    let statuses: Vec<i32>    = batch.iter().map(|e| e.status_code).collect();
    let durations: Vec<f64>   = batch.iter().map(|e| e.duration_ms).collect();

    let result = sqlx::query(
        "INSERT INTO analytics_events
             (id, path, method, status_code, duration_ms, app_id, user_id, event_type, docs_count, created_at)
         SELECT
             UNNEST($1::uuid[]),
             UNNEST($2::text[]),
             UNNEST($3::text[]),
             UNNEST($4::int[]),
             UNNEST($5::float8[]),
             NULL, NULL, 'request', 0, NOW()",
    )
    .bind(&ids)
    .bind(&paths)
    .bind(&methods)
    .bind(&statuses)
    .bind(&durations)
    .execute(&*pool)
    .await;

    if let Err(e) = result {
        tracing::warn!("Analytics batch flush failed ({} events): {e}", batch.len());
    }

    batch.clear();
}

/// Background task: run daily aggregation of analytics data.
/// Call with tokio::spawn at startup.
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
             WHERE created_at >= CURRENT_DATE - INTERVAL '1 day'
               AND created_at <  CURRENT_DATE
             ON CONFLICT (date) DO UPDATE SET
                 total_requests = EXCLUDED.total_requests,
                 unique_users   = EXCLUDED.unique_users",
        )
        .execute(&pool)
        .await;

        match result {
            Ok(_) => tracing::info!("Daily analytics aggregation complete"),
            Err(e) => tracing::error!("Daily aggregation failed: {e}"),
        }
    }
}
