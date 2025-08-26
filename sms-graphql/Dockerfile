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

# Build only the GraphQL server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p sms-graphql && \
    cp /app/target/release/sms-graphql /tmp/sms-graphql

# --- Runtime stage ---
FROM alpine:3.19
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app \
    && chown -R app:app /app \
    && apk add --no-cache curl

COPY --from=builder /tmp/sms-graphql /usr/local/bin/sms-graphql

EXPOSE 8080
USER app

CMD ["/usr/local/bin/sms-graphql", "--port", "8080"]