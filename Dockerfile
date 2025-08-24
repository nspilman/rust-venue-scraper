# syntax=docker/dockerfile:1

# --- Build stage ---
FROM rust:1.83-alpine AS builder
WORKDIR /app

# System deps for musl builds
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig build-base curl ca-certificates

# Copy everything and build with cache mounts
# The cache mounts persist between builds, so only changed code gets recompiled
COPY . .

# Build both projects with BuildKit cache mounts for maximum caching
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --features db && \
    cargo build --release --manifest-path web-server/Cargo.toml --target-dir /app/target && \
    # Copy binaries out of the cache mount to avoid losing them
    cp /app/target/release/sms_scraper /tmp/sms_scraper && \
    cp /app/target/release/web-server /tmp/web-server

# --- Runtime stage ---
FROM alpine:3.19
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app \
    && chown -R app:app /app \
    && apk add --no-cache curl
    
COPY --from=builder /tmp/sms_scraper /usr/local/bin/sms_scraper
COPY --from=builder /tmp/web-server /usr/local/bin/web-server

# Include runtime assets
RUN mkdir -p /app/registry/sources
COPY --from=builder /app/registry/sources/*.json /app/registry/sources/

EXPOSE 8080 9898 3000
USER app
ENV SMS_METRICS_PORT=9898
CMD ["/usr/local/bin/sms_scraper", "server", "--port", "8080", "--use-database"]
