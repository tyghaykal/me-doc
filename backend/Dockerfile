# --- Builder ---
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies separately from source changes.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY . .
RUN touch src/main.rs && cargo build --release

# --- Runtime ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/me-doc-backend /app/me-doc-backend
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080
CMD ["/app/me-doc-backend"]
