-- Analytics tables

CREATE TABLE IF NOT EXISTS analytics_events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    path          TEXT NOT NULL,
    method        TEXT NOT NULL DEFAULT 'GET',
    status_code   INTEGER NOT NULL DEFAULT 200,
    duration_ms   DOUBLE PRECISION NOT NULL DEFAULT 0,
    app_id        UUID REFERENCES apps(id) ON DELETE SET NULL,
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    event_type    TEXT NOT NULL DEFAULT 'request',
    docs_count    BIGINT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_analytics_events_app ON analytics_events (app_id);
CREATE INDEX IF NOT EXISTS idx_analytics_events_user ON analytics_events (user_id);
CREATE INDEX IF NOT EXISTS idx_analytics_events_created ON analytics_events (created_at);
CREATE INDEX IF NOT EXISTS idx_analytics_events_path ON analytics_events (path);

CREATE TABLE IF NOT EXISTS analytics_daily_aggregates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    date            DATE NOT NULL UNIQUE,
    total_requests  BIGINT NOT NULL DEFAULT 0,
    unique_users    BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_analytics_agg_date ON analytics_daily_aggregates (date);
