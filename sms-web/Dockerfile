# syntax=docker/dockerfile:1

# --- Build stage ---
FROM rust:1.83-alpine AS builder
WORKDIR /app

# System deps for musl builds
RUN apk add --no-cache musl-dev pkgconfig build-base curl ca-certificates openssl-dev

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY sms-core/ ./sms-core/
COPY sms-graphql/ ./sms-graphql/
COPY sms-scraper/ ./sms-scraper/
COPY sms-web/ ./sms-web/

# Build only the web server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p sms-web && \
    cp /app/target/release/sms-web /tmp/sms-web

# --- Runtime stage ---
FROM alpine:3.19
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app \
    && chown -R app:app /app \
    && apk add --no-cache curl

COPY --from=builder /tmp/sms-web /usr/local/bin/sms-web
COPY --from=builder /app/sms-web/templates ./templates
COPY --from=builder /app/sms-web/static ./static

EXPOSE 3001
USER app

CMD ["/usr/local/bin/sms-web"]