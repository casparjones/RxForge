# syntax=docker/dockerfile:1.7

# ── Stage 1: Build frontend ───────────────────────────────────────────────────
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package.json ./
COPY package-lock.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm ci

COPY frontend/ ./
RUN npm run build

# ── Stage 2: Build backend ────────────────────────────────────────────────────
FROM rust:1-bookworm AS backend-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source (no dummy-build trick needed with cache mounts)
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY backend/src backend/src
COPY backend/migrations backend/migrations
# include_str! embeds test-app/index.html at compile time
COPY test-app/index.html test-app/index.html

# Persistent caches for cargo registry, git index, and build artifacts.
# Binary must be copied OUT of the cache mount because /app/target is ephemeral.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release -p rxforge-backend && \
    cp /app/target/release/rxforge-backend /rxforge-backend

# ── Stage 3: Runtime ──────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12

COPY --from=backend-builder /rxforge-backend /rxforge
COPY --from=frontend-builder /app/frontend/build /app/frontend/build

EXPOSE 8080

ENV FRONTEND_DIR=/app/frontend/build
ENV JWT_PRIVATE_KEY_PATH=/app/keys/private.pem
ENV JWT_PUBLIC_KEY_PATH=/app/keys/public.pem

CMD ["/rxforge"]