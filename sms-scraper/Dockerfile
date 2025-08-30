# syntax=docker/dockerfile:1

# --- Build stage ---
FROM rust:1.83-alpine AS builder
WORKDIR /app

# System deps for musl builds
RUN apk add --no-cache musl-dev pkgconfig build-base curl ca-certificates openssl-dev openssl-libs-static

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY sms-core/ ./sms-core/
COPY sms-graphql/ ./sms-graphql/
COPY sms-scraper/ ./sms-scraper/
COPY sms-web/ ./sms-web/

# Build only the scraper
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p sms-scraper && \
    cp /app/target/release/sms-scraper /tmp/sms-scraper

# --- Runtime stage ---
FROM alpine:3.19
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app \
    && chown -R app:app /app \
    && apk add --no-cache curl

COPY --from=builder /tmp/sms-scraper /usr/local/bin/sms-scraper

# Copy runtime assets
RUN mkdir -p /app/registry/sources
COPY registry/sources/ /app/registry/sources/

EXPOSE 9898
USER app
ENV SMS_METRICS_PORT=9898

# Run ingestion for all sources, then run full pipeline for each source
CMD ["/bin/sh", "-c", " \
    /usr/local/bin/sms-scraper ingester --apis blue_moon,sea_monster,darrells_tavern,barboza,kexp,neumos,conor_byrne --bypass-cadence; \
    /usr/local/bin/sms-scraper full-pipeline --source-id blue_moon; \
    /usr/local/bin/sms-scraper full-pipeline --source-id sea_monster; \
    /usr/local/bin/sms-scraper full-pipeline --source-id darrells_tavern; \
    /usr/local/bin/sms-scraper full-pipeline --source-id barboza; \
    /usr/local/bin/sms-scraper full-pipeline --source-id kexp; \
    /usr/local/bin/sms-scraper full-pipeline --source-id neumos; \
    /usr/local/bin/sms-scraper full-pipeline --source-id conor_byrne \
"]