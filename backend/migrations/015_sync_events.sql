-- Sync audit trail: which device wrote or deleted which document.
--
-- Motivation: clients that infer deletions from "document missing locally" can
-- wipe a whole dataset when their local storage is evicted (e.g. Safari clears
-- IndexedDB after ~7 days of inactivity while localStorage survives). Without a
-- log there is no way to tell afterwards which device caused the damage.
-- Every accepted push row is recorded here with its originating device.

CREATE TABLE sync_events (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    app_id         UUID        NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    owner_id       UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Client-reported identity (never trusted for authorization, only for the trail)
    device_id      TEXT        NOT NULL DEFAULT '',
    device_label   TEXT        NOT NULL DEFAULT '',
    platform       TEXT        NOT NULL DEFAULT '',
    app_version    TEXT        NOT NULL DEFAULT '',
    user_agent     TEXT        NOT NULL DEFAULT '',
    -- Why the client pushed: e.g. 'user-edit', 'recovery', 'initial'
    reason         TEXT        NOT NULL DEFAULT '',
    doc_id         TEXT        NOT NULL,
    -- write   = document created/updated
    -- delete  = document soft-deleted (_deleted: true)
    -- conflict = push rejected, server state returned to the client
    op             TEXT        NOT NULL CHECK (op IN ('write', 'delete', 'conflict')),
    -- The document's own updatedAt (epoch ms) when the client provided one
    doc_updated_at BIGINT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_events_app_created ON sync_events (app_id, created_at DESC);
CREATE INDEX idx_sync_events_device ON sync_events (app_id, device_id);
CREATE INDEX idx_sync_events_doc ON sync_events (app_id, doc_id);
CREATE INDEX idx_sync_events_op ON sync_events (app_id, op);
