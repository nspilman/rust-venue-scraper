# Handy shortcuts for local development

# Default target
.DEFAULT_GOAL := help

# Colors
YELLOW=\033[1;33m
GREEN=\033[0;32m
NC=\033[0m

help: ## Show this help
	@printf "$(YELLOW)Available targets$(NC)\n"
	@awk 'BEGIN {FS = ":.*##"}; /^[a-zA-Z0-9_-]+:.*?##/ { printf "  $(GREEN)%-22s$(NC) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# Build
build: ## Build debug binaries
	cargo build

build-release: ## Build release binaries (required for start_servers.sh)
	cargo build --release && (cd web-server && cargo build --release)

# Servers
run-graphql: ## Run GraphQL server (in-memory) on port 8080
	cargo run -- server --port 8080

run-graphql-db: ## Run GraphQL server (database mode) on port 8080 (requires LIBSQL_* env)
	cargo run -- server --port 8080 --use-database

run-web: ## Run web UI (port 3000)
	cd web-server && cargo run

start: build-release ## Start both GraphQL and web servers via script (ports 8080/3000)
	./start_servers.sh

stop: ## Stop both servers via script
	./stop_servers.sh || true

# Pipelines
ingest: ## Run ingester (in-memory) for all crawlers
	cargo run -- ingester --apis blue_moon,sea_monster,darrells_tavern

ingest-db: ## Run ingester (database mode)
	cargo run -- ingester --apis blue_moon,sea_monster,darrells_tavern --use-database


clear-db: ## Clear all data from the database (DANGEROUS)
	cargo run -- clear-db

# Tools
validate-envelope: ## Validate an envelope JSON file: make validate-envelope FILE=path/to/file.json
	@if [ -z "$(FILE)" ]; then echo "Usage: make validate-envelope FILE=path/to/file.json [SCHEMA=schemas/envelope.v1.json]"; exit 1; fi
	cargo run --bin validate-envelope -- $(FILE) $(if $(SCHEMA),--schema $(SCHEMA),)

# Tests
test: ## Run tests
	cargo test

clippy: ## Run clippy lints (deny warnings)
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

# Dashboard Generation
dashboard: ## Generate static dashboard JSON
	cargo run --bin build-dashboard

dashboard-dynamic: ## Generate dynamic dashboard JSON from MetricName enum
	cargo run --bin build-dashboard dynamic

dashboard-provision: dashboard-dynamic ## Generate and provision dynamic dashboard to Grafana
	@echo "📊 Provisioning dynamic dashboard to Grafana..."
	cp grafana-dashboard-dynamic.json ops/grafana/provisioning/dashboards/
	@echo "✅ Dashboard provisioned! Grafana will auto-reload in ~30 seconds"
	@echo "🌐 Access at: http://localhost:3000"

dashboard-update: ## Update dashboard when adding new metrics
	./scripts/update-dashboard.sh

