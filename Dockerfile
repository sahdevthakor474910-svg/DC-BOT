# ────────────────────────────────────────────────────────────────────────────
# Stage 1: Build
# ────────────────────────────────────────────────────────────────────────────
FROM rust:1.79-slim-bookworm AS builder

WORKDIR /app

# Install native build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Pre-cache dependencies by building a stub binary first
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release --locked
RUN rm -f target/release/deps/dc_bot*

# Copy real source and migrations, then build
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release --locked

# ────────────────────────────────────────────────────────────────────────────
# Stage 2: Runtime
# ────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create data directory for SQLite persistence
RUN mkdir -p /data

# Copy compiled binary
COPY --from=builder /app/target/release/dc-bot /usr/local/bin/dc-bot

# Persistent volume for the SQLite database
VOLUME ["/data"]

# Default environment (overridden by docker-compose or -e flags)
ENV DATABASE_URL=sqlite:///data/bot.db
ENV LOG_LEVEL=info

CMD ["dc-bot"]
