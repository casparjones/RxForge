-- oauth_clients.id is the same UUID as apps.id, but had no FK.
-- Orphaned rows (apps deleted without cleaning up oauth_clients) block
-- the constraint addition, so remove them first.
DELETE FROM oauth_clients WHERE id NOT IN (SELECT id FROM apps);

-- Now link oauth_clients lifetime to its parent app.
-- Deleting an app will cascade → oauth_clients → oauth_codes, oauth_tokens,
-- oauth_consents, user_features (all have ON DELETE CASCADE to oauth_clients).
ALTER TABLE oauth_clients
    ADD CONSTRAINT fk_oauth_clients_app
    FOREIGN KEY (id) REFERENCES apps(id) ON DELETE CASCADE;
