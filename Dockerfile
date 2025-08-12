# syntax=docker/dockerfile:1

# --- Build stage ---
FROM rust:1.83-alpine AS builder
WORKDIR /app
# System deps for musl builds (include static OpenSSL for musl)
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig build-base curl ca-certificates
# Cache deps first
COPY Cargo.toml Cargo.lock ./
# Create dummy src to cache dependencies (file, not directory)
RUN mkdir -p src && printf 'fn main(){}' > src/main.rs
# Now copy the real source
COPY . .
# Final build with stable toolchain (disable default features to avoid pulling optional DB)
RUN cargo build --release --no-default-features

# --- Runtime stage ---
FROM alpine:3.19
WORKDIR /app
RUN addgroup -S app && adduser -S app -G app \
    && chown -R app:app /app \
    && apk add --no-cache curl
COPY --from=builder /app/target/release/sms_scraper /usr/local/bin/sms_scraper
# Include runtime assets needed by the binary (registry files)
# Create minimal registry structure and copy only required source specs
RUN mkdir -p /app/registry/sources
COPY --from=builder /app/registry/sources/blue_moon.json /app/registry/sources/blue_moon.json
COPY --from=builder /app/registry/sources/sea_monster.json /app/registry/sources/sea_monster.json
COPY --from=builder /app/registry/sources/darrells_tavern.json /app/registry/sources/darrells_tavern.json
# Expose GraphQL server port and Prometheus metrics port
EXPOSE 8080 9898
USER app
# Default command runs the long-lived server so the metrics endpoint stays up.
ENV SMS_METRICS_PORT=9898
CMD ["/usr/local/bin/sms_scraper", "server", "--port", "8080"]
