-- Store the consenting user's ID in oauth_codes so that the token exchange
-- can issue a JWT scoped to the consenting user (not the app owner).
ALTER TABLE oauth_codes
    ADD COLUMN IF NOT EXISTS user_id UUID REFERENCES users(id) ON DELETE CASCADE;
