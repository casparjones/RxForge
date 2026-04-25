-- Track last login time per user
ALTER TABLE users ADD COLUMN last_login_at TIMESTAMPTZ;

-- OAuth consent: records that a user has approved access for an app
CREATE TABLE oauth_consents (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id)         ON DELETE CASCADE,
    client_id  UUID        NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, client_id)
);

CREATE INDEX idx_oauth_consents_user ON oauth_consents(user_id);
CREATE INDEX idx_oauth_consents_client ON oauth_consents(client_id);
