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
# Build project dependencies with cache preservation
RUN cargo chef cook --release --recipe-path recipe.json
# Build application binary
COPY . .
RUN cargo build --release --bin cloudflare-dashboard-backend

# Stage 4: Runtime (Minimal Debian Bookworm Slim suitable for Coolify)
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies (TLS/SSL root certificates for reqwest and SQLite libraries)
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy compiled binary and migrations from builder stage
COPY --from=builder /app/target/release/cloudflare-dashboard-backend /app/cloudflare-dashboard-backend
COPY --from=builder /app/migrations /app/migrations

# Create data directory for persistent SQLite storage
RUN mkdir -p /app/data

ENV PORT=8080
ENV DATABASE_URL="sqlite://data/cloudflare.db?mode=rwc"

EXPOSE 8080

CMD ["/app/cloudflare-dashboard-backend"]
