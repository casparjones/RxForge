FROM rust:1-alpine AS builder

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
RUN touch backend/src/main.rs && cargo build --release -p rxforge-backend

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /app/target/release/rxforge-backend /rxforge

EXPOSE 8080

CMD ["/rxforge"]
