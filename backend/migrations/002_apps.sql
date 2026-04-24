-- Applications table

CREATE TABLE IF NOT EXISTS apps (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                TEXT NOT NULL,
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id           TEXT NOT NULL UNIQUE,
    client_secret_hash  TEXT NOT NULL,
    redirect_uris       JSONB NOT NULL DEFAULT '[]',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_apps_owner ON apps (owner_id);
CREATE INDEX IF NOT EXISTS idx_apps_client_id ON apps (client_id);
