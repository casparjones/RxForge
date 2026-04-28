# ── Stage 1: Build frontend ───────────────────────────────────────────────────
FROM node:22-alpine AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json ./
COPY package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ── Stage 2: Build backend ────────────────────────────────────────────────────
FROM rust:1-alpine AS backend-builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/Cargo.toml
RUN mkdir -p backend/src && echo 'fn main() {}' > backend/src/main.rs
RUN cargo build --release -p rxforge-backend 2>/dev/null || true
RUN rm -rf backend/src

# Build actual source
COPY backend/src backend/src
COPY backend/migrations backend/migrations
# include_str! embeds test-app/index.html at compile time
COPY test-app/index.html test-app/index.html
RUN touch backend/src/main.rs && cargo build --release -p rxforge-backend

# ── Stage 3: Runtime ──────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12

COPY --from=backend-builder /app/target/release/rxforge-backend /rxforge
COPY --from=frontend-builder /app/frontend/build /app/frontend/build

EXPOSE 8080

ENV FRONTEND_DIR=/app/frontend/build
ENV JWT_PRIVATE_KEY_PATH=/app/keys/private.pem
ENV JWT_PUBLIC_KEY_PATH=/app/keys/public.pem

CMD ["/rxforge"]
