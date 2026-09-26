# Three stages: build the frontend, build the backend, then copy only the
# results into a small runtime image.

# ---- 1. Svelte frontend -> static files in /app/dist ----
FROM node:24-slim AS frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# ---- 2. Rust backend -> one binary ----
FROM rust:1-slim-trixie AS backend
WORKDIR /app
# Build the dependencies first, with a placeholder main.rs. Docker caches this
# layer, so later code changes don't recompile every crate.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src
COPY backend/ ./
# `touch` makes cargo notice the real main.rs is newer than the placeholder build.
RUN touch src/main.rs && cargo build --release

# ---- 3. Runtime ----
FROM debian:trixie-slim
# sqlite3 CLI: used by scripts/backup.sh through `docker compose exec`.
# ca-certificates: lets the app verify HTTPS when calling the AI gateway.
RUN apt-get update \
    && apt-get install -y --no-install-recommends sqlite3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /app/target/release/ielts /app/ielts
COPY --from=frontend /app/dist /app/static

ENV APP_BIND=0.0.0.0:1111 \
    DATABASE_PATH=/data/ielts.db \
    STATIC_DIR=/app/static \
    APP_TZ=Asia/Bangkok \
    RUST_LOG=info
EXPOSE 1111
CMD ["/app/ielts"]
