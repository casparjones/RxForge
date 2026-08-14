//! Sync audit trail — "which device did what".
//!
//! Every accepted push row is recorded in `sync_events` together with the
//! device identity the client reported. The identity is informational only:
//! authorization still comes from the token, a client can claim any device id.
//! What it buys is forensics — after a device wipes data (e.g. because its
//! IndexedDB was evicted and it pushed deletes for everything), the trail shows
//! which device it was and when.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{app_id}/sync-events", get(list_events))
        .route("/{app_id}/sync-devices", get(list_devices))
}

// ── Client identity ───────────────────────────────────────────────────────────

/// Device information a client sends along with a push.
/// Accepts camelCase (what browser clients send) and snake_case.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct ClientInfo {
    #[serde(alias = "deviceId", default)]
    pub device_id: Option<String>,
    #[serde(alias = "deviceLabel", default)]
    pub device_label: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(alias = "appVersion", default)]
    pub app_version: Option<String>,
    /// Why this push happened: `user-edit`, `recovery`, `initial`, …
    #[serde(default)]
    pub reason: Option<String>,
}

impl ClientInfo {
    /// Fill blanks from `X-RxForge-Device-*` headers and the User-Agent, so
    /// clients that cannot change their request body still show up in the trail.
    pub fn merged_with_headers(mut self, headers: &HeaderMap) -> Self {
        fn header(headers: &HeaderMap, name: &str) -> Option<String> {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }

        if self.device_id.as_deref().unwrap_or("").is_empty() {
            self.device_id = header(headers, "X-RxForge-Device-Id");
        }
        if self.device_label.as_deref().unwrap_or("").is_empty() {
            self.device_label = header(headers, "X-RxForge-Device-Label");
        }
        if self.platform.as_deref().unwrap_or("").is_empty() {
            self.platform = header(headers, "X-RxForge-Platform");
        }
        self
    }

    /// Truncate each field so a malicious client cannot bloat the log table.
    fn clamped(&self) -> (String, String, String, String, String) {
        fn cut(v: &Option<String>, max: usize) -> String {
            v.as_deref().unwrap_or("").chars().take(max).collect()
        }
        (
            cut(&self.device_id, 128),
            cut(&self.device_label, 128),
            cut(&self.platform, 64),
            cut(&self.app_version, 32),
            cut(&self.reason, 32),
        )
    }
}

/// One logged operation.
pub struct RecordedOp {
    pub doc_id: String,
    /// `write`, `delete` or `conflict`
    pub op: &'static str,
    pub doc_updated_at: Option<i64>,
}

/// Insert a batch of operations. Errors are logged, never propagated — a broken
/// audit trail must not fail an otherwise successful sync.
pub async fn record(
    db: &PgPool,
    retention_days: i32,
    app_id: Uuid,
    owner_id: Uuid,
    client: &ClientInfo,
    user_agent: &str,
    ops: &[RecordedOp],
) {
    if ops.is_empty() {
        return;
    }

    let (device_id, device_label, platform, app_version, reason) = client.clamped();
    let ua: String = user_agent.chars().take(256).collect();

    let doc_ids: Vec<String> = ops.iter().map(|o| o.doc_id.clone()).collect();
    let op_kinds: Vec<String> = ops.iter().map(|o| o.op.to_string()).collect();
    let updated_ats: Vec<Option<i64>> = ops.iter().map(|o| o.doc_updated_at).collect();

    let res = sqlx::query(
        "INSERT INTO sync_events
             (app_id, owner_id, device_id, device_label, platform, app_version,
              user_agent, reason, doc_id, op, doc_updated_at)
         SELECT $1, $2, $3, $4, $5, $6, $7, $8, d.doc_id, d.op, d.updated_at
         FROM UNNEST($9::text[], $10::text[], $11::bigint[])
              AS d(doc_id, op, updated_at)",
    )
    .bind(app_id)
    .bind(owner_id)
    .bind(&device_id)
    .bind(&device_label)
    .bind(&platform)
    .bind(&app_version)
    .bind(&ua)
    .bind(&reason)
    .bind(&doc_ids)
    .bind(&op_kinds)
    .bind(&updated_ats)
    .execute(db)
    .await;

    if let Err(e) = res {
        tracing::warn!("failed to record sync events for app {app_id}: {e}");
        return;
    }

    // Opportunistic retention: cheap thanks to idx_sync_events_app_created.
    if retention_days > 0 {
        let _ = sqlx::query(
            "DELETE FROM sync_events
             WHERE app_id = $1
               AND created_at < NOW() - make_interval(days => $2)",
        )
        .bind(app_id)
        .bind(retention_days)
        .execute(db)
        .await;
    }
}

// ── Read API ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    /// Filter by a single device id.
    pub device_id: Option<String>,
    /// Filter by document id (exact match).
    pub doc_id: Option<String>,
    /// Filter by operation: `write`, `delete` or `conflict`.
    pub op: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SyncEvent {
    pub id: Uuid,
    pub device_id: String,
    pub device_label: String,
    pub platform: String,
    pub app_version: String,
    pub reason: String,
    pub doc_id: String,
    pub op: String,
    pub doc_updated_at: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub events: Vec<SyncEvent>,
    pub total: i64,
    pub page: u64,
    pub per_page: u64,
    pub pages: u64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeviceSummary {
    pub device_id: String,
    pub device_label: String,
    pub platform: String,
    pub app_version: String,
    pub writes: i64,
    pub deletes: i64,
    pub conflicts: i64,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Verify the caller owns this app.
async fn assert_app_owner(state: &AppState, app_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM apps WHERE id = $1 AND owner_id = $2")
            .bind(app_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
    row.ok_or_else(|| AppError::NotFound("App not found".to_string()))?;
    Ok(())
}

fn caller_id(user: &AuthUser) -> AppResult<Uuid> {
    Uuid::parse_str(user.user_id())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user ID")))
}

pub async fn list_events(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ListResponse>> {
    let user_id = caller_id(&user)?;
    assert_app_owner(&state, app_id, user_id).await?;

    let per_page = query.per_page.unwrap_or(50).clamp(1, 200);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    // Empty filters are passed as NULL so a single statement covers every combination.
    let device = query.device_id.filter(|s| !s.is_empty());
    let doc = query.doc_id.filter(|s| !s.is_empty());
    let op = query
        .op
        .filter(|s| matches!(s.as_str(), "write" | "delete" | "conflict"));

    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sync_events
         WHERE app_id = $1
           AND ($2::text IS NULL OR device_id = $2)
           AND ($3::text IS NULL OR doc_id = $3)
           AND ($4::text IS NULL OR op = $4)",
    )
    .bind(app_id)
    .bind(&device)
    .bind(&doc)
    .bind(&op)
    .fetch_one(&state.db)
    .await?;

    let events: Vec<SyncEvent> = sqlx::query_as(
        "SELECT id, device_id, device_label, platform, app_version, reason,
                doc_id, op, doc_updated_at, created_at
         FROM sync_events
         WHERE app_id = $1
           AND ($2::text IS NULL OR device_id = $2)
           AND ($3::text IS NULL OR doc_id = $3)
           AND ($4::text IS NULL OR op = $4)
         ORDER BY created_at DESC, id DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(app_id)
    .bind(&device)
    .bind(&doc)
    .bind(&op)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await?;

    let pages = if total == 0 {
        1
    } else {
        (total as u64 + per_page - 1) / per_page
    };

    Ok(Json(ListResponse {
        events,
        total,
        page,
        per_page,
        pages,
    }))
}

/// One row per device that ever pushed to this app, with per-operation counts.
pub async fn list_devices(
    State(state): State<AppState>,
    user: AuthUser,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Vec<DeviceSummary>>> {
    let user_id = caller_id(&user)?;
    assert_app_owner(&state, app_id, user_id).await?;

    // A device may change its label or app version over time; report the newest.
    let devices: Vec<DeviceSummary> = sqlx::query_as(
        "SELECT device_id,
                (ARRAY_AGG(device_label ORDER BY created_at DESC))[1]  AS device_label,
                (ARRAY_AGG(platform ORDER BY created_at DESC))[1]      AS platform,
                (ARRAY_AGG(app_version ORDER BY created_at DESC))[1]   AS app_version,
                COUNT(*) FILTER (WHERE op = 'write')    AS writes,
                COUNT(*) FILTER (WHERE op = 'delete')   AS deletes,
                COUNT(*) FILTER (WHERE op = 'conflict') AS conflicts,
                MIN(created_at) AS first_seen,
                MAX(created_at) AS last_seen
         FROM sync_events
         WHERE app_id = $1
         GROUP BY device_id
         ORDER BY MAX(created_at) DESC",
    )
    .bind(app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(devices))
}
