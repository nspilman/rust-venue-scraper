# syntax=docker/dockerfile:1

# --- Build stage ---
FROM rust:1.83-alpine AS builder
WORKDIR /app

# System deps for musl builds
RUN apk add --no-cache musl-dev pkgconfig build-base curl ca-certificates sccache openssl-dev
ENV RUSTC_WRAPPER=/usr/bin/sccache

# Prefetch dependencies with only manifests to leverage Docker layer cache
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/root/.cache/sccache \
    cargo fetch

# Copy everything and build with cache mounts
# The cache mounts persist between builds, so only changed code gets recompiled
COPY . .

# Build lightweight GraphQL server binary only
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/root/.cache/sccache \
    cargo fetch && \
    cargo build --release --features db --bin graphql_server && \
    # Copy binary out of the cache mount to avoid losing it
    cp /app/target/release/graphql_server /tmp/graphql_server && \
    true

# --- Runtime stage ---
FROM alpine:3.19
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app \
    && chown -R app:app /app \
    && apk add --no-cache curl
    
COPY --from=builder /tmp/graphql_server /usr/local/bin/graphql_server

# Include runtime assets
RUN mkdir -p /app/registry/sources
COPY --from=builder /app/registry/sources/blue_moon.json /app/registry/sources/
COPY --from=builder /app/registry/sources/sea_monster.json /app/registry/sources/
COPY --from=builder /app/registry/sources/darrells_tavern.json /app/registry/sources/
COPY --from=builder /app/registry/sources/kexp.json /app/registry/sources/
COPY --from=builder /app/registry/sources/barboza.json /app/registry/sources/

EXPOSE 8080 9898
USER app
ENV SMS_METRICS_PORT=9898
CMD ["/usr/local/bin/graphql_server", "--port", "8080", "--use-database"]

