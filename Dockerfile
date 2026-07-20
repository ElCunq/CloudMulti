# syntax=docker/dockerfile:1

# Stage 1: Chef
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

# Stage 2: Planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build project dependencies using persistent BuildKit cache mounts so Cargo never recompiles existing crates!
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# Copy application source code
COPY . .

# Build application binary with persistent cache mounts for lightning-fast incremental builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin cloudflare-dashboard-backend && \
    cp /app/target/release/cloudflare-dashboard-backend /app/cloudflare-dashboard-backend

# Stage 4: Runtime (Minimal Debian Bookworm Slim suitable for Coolify)
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies (TLS/SSL root certificates for reqwest and SQLite libraries)
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy compiled binary and migrations from builder stage
COPY --from=builder /app/cloudflare-dashboard-backend /app/cloudflare-dashboard-backend
COPY --from=builder /app/migrations /app/migrations

# Create data directory and public directory for persistent storage and frontend SPA
RUN mkdir -p /app/data
COPY public /app/public

ENV PORT=8080
ENV DATABASE_URL="sqlite:///app/data/cloudflare.db?mode=rwc"

VOLUME /app/data

EXPOSE 8080
EXPOSE 80

CMD ["/app/cloudflare-dashboard-backend"]
