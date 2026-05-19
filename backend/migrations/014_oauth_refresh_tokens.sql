-- Add refresh token support and user_id tracking to oauth_tokens.
-- refresh_token: opaque random string used to obtain a new access token.
-- user_id: the consenting user, needed to re-issue app-scoped JWTs on refresh.
ALTER TABLE oauth_tokens
    ADD COLUMN IF NOT EXISTS refresh_token TEXT UNIQUE,
    ADD COLUMN IF NOT EXISTS user_id UUID REFERENCES users(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_oauth_tokens_refresh_token ON oauth_tokens (refresh_token);
