-- Per-app queries filter on (app_id, event_type) for push/pull doc counts.
CREATE INDEX IF NOT EXISTS idx_analytics_events_app_event
    ON analytics_events (app_id, event_type);

-- Per-app queries filter/sort on (app_id, created_at) for time-range and day-histogram.
CREATE INDEX IF NOT EXISTS idx_analytics_events_app_created
    ON analytics_events (app_id, created_at DESC);
