# ===== Builder stage =====
FROM rust:1.92-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    build-essential \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build the release binary
RUN cargo build --release

# ===== Runtime stage =====
FROM debian:bookworm-slim

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-dev \
    libssl-dev \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/matrix-webhook-thing /usr/local/bin/app

EXPOSE 1337
CMD ["app"]

