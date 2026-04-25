-- Add auth_type discriminator to apps (oauth = existing flow, token = public API key)
ALTER TABLE apps
    ADD COLUMN auth_type TEXT NOT NULL DEFAULT 'oauth'
        CHECK (auth_type IN ('oauth', 'token'));

-- Per-app API tokens for token-based apps
CREATE TABLE app_tokens (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    app_id          UUID         NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    name            TEXT         NOT NULL DEFAULT 'Default',
    token_hash      TEXT         NOT NULL UNIQUE,   -- SHA-256(plaintext token)
    token_prefix    TEXT         NOT NULL,           -- first 16 chars for UI display
    allowed_origins TEXT[]       NOT NULL DEFAULT '{}', -- empty = unrestricted
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ
);

CREATE INDEX idx_app_tokens_app_id ON app_tokens(app_id);
-- Partial index: only active tokens need fast hash lookups
CREATE INDEX idx_app_tokens_hash_active ON app_tokens(token_hash)
    WHERE revoked_at IS NULL;
