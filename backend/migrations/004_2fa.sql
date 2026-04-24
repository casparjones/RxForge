-- 2FA: TOTP and WebAuthn/Passkey tables

CREATE TABLE IF NOT EXISTS user_totp (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    secret      TEXT NOT NULL,
    verified    BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_passkeys (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id       TEXT NOT NULL UNIQUE,
    public_key          BYTEA NOT NULL,
    sign_count          BIGINT NOT NULL DEFAULT 0,
    aaguid              TEXT,
    display_name        TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_user_passkeys_user ON user_passkeys (user_id);
CREATE INDEX IF NOT EXISTS idx_user_passkeys_credential ON user_passkeys (credential_id);
