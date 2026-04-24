use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::{require_permission, AuthUser},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/global", get(global_analytics))
        .route("/apps/{id}", get(app_analytics))
}

#[derive(Debug, Serialize)]
pub struct GlobalAnalyticsResponse {
    pub total_requests: i64,
    pub total_users: i64,
    pub total_apps: i64,
    pub requests_today: i64,
    pub top_endpoints: Vec<EndpointStat>,
}

#[derive(Debug, Serialize)]
pub struct EndpointStat {
    pub path: String,
    pub count: i64,
    pub avg_duration_ms: f64,
}

pub async fn global_analytics(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<GlobalAnalyticsResponse>> {
    require_permission(&user, "analytics:global")?;

    let (total_requests,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM analytics_events")
            .fetch_one(&state.db)
            .await?;

    let (total_users,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    let (total_apps,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM apps")
        .fetch_one(&state.db)
        .await?;

    let (requests_today,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM analytics_events WHERE created_at >= CURRENT_DATE",
    )
    .fetch_one(&state.db)
    .await?;

    #[derive(sqlx::FromRow)]
    struct EndpointRow {
        path: String,
        count: i64,
        avg_duration: f64,
    }

    let top_endpoints: Vec<EndpointRow> = sqlx::query_as(
        "SELECT path, COUNT(*) as count, AVG(duration_ms) as avg_duration
         FROM analytics_events
         GROUP BY path
         ORDER BY count DESC
         LIMIT 10",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(GlobalAnalyticsResponse {
        total_requests,
        total_users,
        total_apps,
        requests_today,
        top_endpoints: top_endpoints
            .into_iter()
            .map(|r| EndpointStat {
                path: r.path,
                count: r.count,
                avg_duration_ms: r.avg_duration,
            })
            .collect(),
    }))
}

#[derive(Debug, Serialize)]
pub struct AppAnalyticsResponse {
    pub app_id: String,
    pub total_sync_requests: i64,
    pub total_push_docs: i64,
    pub total_pull_docs: i64,
    pub active_users_today: i64,
    pub requests_by_day: Vec<DayStat>,
}

#[derive(Debug, Serialize)]
pub struct DayStat {
    pub date: String,
    pub count: i64,
}

pub async fn app_analytics(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<AppAnalyticsResponse>> {
    let owner_id = Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))?;

    let is_admin = user.has_role("admin");
    if !is_admin {
        let app: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM apps WHERE id = $1 AND owner_id = $2")
                .bind(app_id)
                .bind(owner_id)
                .fetch_optional(&state.db)
                .await?;

        if app.is_none() {
            return Err(AppError::NotFound("App not found".to_string()));
        }
    }

    let (total_sync_requests,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM analytics_events WHERE app_id = $1")
            .bind(app_id)
            .fetch_one(&state.db)
            .await?;

    let (total_push_docs,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(docs_count), 0)::BIGINT FROM analytics_events WHERE app_id = $1 AND event_type = 'push'",
    )
    .bind(app_id)
    .fetch_one(&state.db)
    .await?;

    let (total_pull_docs,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(docs_count), 0)::BIGINT FROM analytics_events WHERE app_id = $1 AND event_type = 'pull'",
    )
    .bind(app_id)
    .fetch_one(&state.db)
    .await?;

    let (active_users_today,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT user_id) FROM analytics_events WHERE app_id = $1 AND created_at >= CURRENT_DATE",
    )
    .bind(app_id)
    .fetch_one(&state.db)
    .await?;

    #[derive(sqlx::FromRow)]
    struct DayRow {
        day: Option<chrono::NaiveDate>,
        count: i64,
    }

    let day_rows: Vec<DayRow> = sqlx::query_as(
        "SELECT DATE(created_at) as day, COUNT(*) as count
         FROM analytics_events WHERE app_id = $1
         GROUP BY day ORDER BY day DESC LIMIT 30",
    )
    .bind(app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(AppAnalyticsResponse {
        app_id: app_id.to_string(),
        total_sync_requests,
        total_push_docs,
        total_pull_docs,
        active_users_today,
        requests_by_day: day_rows
            .into_iter()
            .map(|r| DayStat {
                date: r.day.map(|d| d.to_string()).unwrap_or_default(),
                count: r.count,
            })
            .collect(),
    }))
}
