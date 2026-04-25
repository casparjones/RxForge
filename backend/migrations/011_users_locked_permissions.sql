-- Per-user locked flag and extra permissions array
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS locked      BOOLEAN  NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS permissions TEXT[]   NOT NULL DEFAULT '{}';
