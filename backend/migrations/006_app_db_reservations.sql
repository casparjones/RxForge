-- Placeholder reservation rows for CouchDB db-prefix per app.
-- Actual CouchDB provisioning happens in a later ticket – for now we record
-- the intended db prefix so app creation is idempotent and auditable.

CREATE TABLE IF NOT EXISTS app_db_reservations (
    app_id      UUID PRIMARY KEY REFERENCES apps(id) ON DELETE CASCADE,
    db_prefix   TEXT NOT NULL UNIQUE,
    provisioned BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_app_db_reservations_prefix ON app_db_reservations (db_prefix);
