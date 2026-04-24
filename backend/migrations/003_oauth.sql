-- OAuth 2.0 tables

CREATE TABLE IF NOT EXISTS oauth_clients (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id           TEXT NOT NULL UNIQUE,
    client_secret_hash  TEXT NOT NULL,
    redirect_uris       JSONB NOT NULL DEFAULT '[]',
    scope               TEXT NOT NULL DEFAULT '',
    active              BOOLEAN NOT NULL DEFAULT true,
    owner_id            UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oauth_clients_client_id ON oauth_clients (client_id);

CREATE TABLE IF NOT EXISTS oauth_codes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id    UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    code         TEXT NOT NULL UNIQUE,
    redirect_uri TEXT NOT NULL,
    scope        TEXT NOT NULL DEFAULT '',
    used         BOOLEAN NOT NULL DEFAULT false,
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oauth_codes_code ON oauth_codes (code);

CREATE TABLE IF NOT EXISTS oauth_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id   UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    token       TEXT NOT NULL UNIQUE,
    scope       TEXT NOT NULL DEFAULT '',
    revoked     BOOLEAN NOT NULL DEFAULT false,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oauth_tokens_token ON oauth_tokens (token);
