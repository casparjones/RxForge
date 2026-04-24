-- Users, roles, permissions tables

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin', 'superadmin')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

CREATE TABLE IF NOT EXISTS roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS permissions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role_id       UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE IF NOT EXISTS user_roles (
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id   UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);

-- Seed built-in roles
INSERT INTO roles (id, name, description) VALUES
    (gen_random_uuid(), 'superadmin', 'Full system access'),
    (gen_random_uuid(), 'admin',      'Administrative access'),
    (gen_random_uuid(), 'user',       'Regular user')
ON CONFLICT (name) DO NOTHING;

-- Seed built-in permissions
INSERT INTO permissions (id, name, description) VALUES
    (gen_random_uuid(), 'apps:create',       'Create applications'),
    (gen_random_uuid(), 'apps:read',         'Read applications'),
    (gen_random_uuid(), 'apps:delete',       'Delete applications'),
    (gen_random_uuid(), 'users:manage',      'Manage users'),
    (gen_random_uuid(), 'analytics:read',    'Read own app analytics'),
    (gen_random_uuid(), 'analytics:global',  'Read global analytics'),
    (gen_random_uuid(), 'admin:access',      'Access admin dashboard')
ON CONFLICT (name) DO NOTHING;

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    revoked     BOOLEAN NOT NULL DEFAULT false,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON refresh_tokens (user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON refresh_tokens (token_hash);
