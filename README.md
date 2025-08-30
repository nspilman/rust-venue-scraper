# SMS Server Rust - Event Data Scraper

A modular, asynchronous Rust-based event data scraper with database persistence, clean architecture, and extensible API support. Part of the Seattle Music Scene (SMS) project.

## 📦 Features

- **🏢 Database Persistence**: Full Turso/libSQL integration with graph-based schema
- **💬 In-Memory Mode**: Fast development and testing without persistence  
- **🔄 ETL Pipeline**: Complete Extract, Transform, Load architecture
- **🔨 Smart Processing**: Venue/artist matching, deduplication, and relationship tracking
- **🚀 Async Architecture**: High-performance concurrent processing
- **📊 Comprehensive Metrics**: Phase-based Prometheus metrics for complete pipeline observability

## Architecture

The scraper exhibits a modular and extensible architecture with clean separation of concerns:

### Core Components

- **EventApi Trait**: Central abstraction that all crawler modules implement, defining the contract for API-specific logic
- **Pipeline Module**: Handles raw event ingestion, processing each raw event into structured `ProcessedEvent` data
- Processing: Handles stored raw data creating/updating domain objects (venues, artists, events) with change tracking
- **Storage Abstraction**: `Storage` trait abstracting all persistence operations with in-memory implementation for development

### Features

- **Modular Design**: Independent development and extension of new crawlers without impacting core pipeline logic
- **Asynchronous Processing**: Concurrent fetching and processing using `tokio` runtime for improved scalability
- **Comprehensive Logging**: Detailed observability via `tracing` with structured error handling using `anyhow` and `thiserror`
- **Flexible Execution**: CLI supports running ingestion, processing, or both sequentially on specified APIs

## Current API Implementations

- **Neumos**: Seattle venue event scraping
- **Barboza**: Capitol Hill venue data extraction
- **Blue Moon**: Venue-specific event scraping
- **Conor Byrne**: Ballard pub event data
- **KEXP**: Radio station event listings
- **Sea Monster**: Lounge event data extraction

## 🚀 Usage

### Quick Start - API Endpoints

Once the services are running, you can access:

**GraphQL API** (port 8080):
- GraphQL Playground: http://localhost:8080/graphql
- Raw GraphQL endpoint: `curl -X POST http://localhost:8080/graphql -H "Content-Type: application/json" -d '{"query":"{ events { id title venue { name } artists { name } } }"}'`

**Web Interface** (port 3001):
- Events listing: http://localhost:3001/events
- Search events: http://localhost:3001/events?search=jazz
- Filter by venue: http://localhost:3001/events?venue=neumos
- View artist: http://localhost:3001/artist/[artist-id]
- View venue: http://localhost:3001/venue/[venue-id]

**Health & Metrics**:
- API health check: http://localhost:8080/health
- Scraper metrics: http://localhost:9898/metrics
- Pushgateway: http://localhost:9091
- Prometheus: http://localhost:9090
- Grafana dashboard: http://localhost:3000 (admin/admin)

### Database Setup (Optional)

For persistent storage, set up a Turso database:

```bash
# Copy environment template
cp .env.example .env

# Edit .env with your Turso credentials:
# LIBSQL_URL=libsql://your-database.turso.io
# LIBSQL_AUTH_TOKEN=your_auth_token
```

### Running the Scraper

```bash
# Build the project
cargo build

# Run minimal ingestion (fetch raw data only)
cargo run --bin sms-scraper -- ingester --source-id neumos

# Run full pipeline (ingestion + processing)
cargo run --bin sms-scraper -- full-pipeline --source-id neumos

# Clear venue data for development/testing
cargo run --bin sms-scraper -- clear-db --venue-slug neumos

# Run GraphQL server
cargo run --bin sms-graphql

# Run web interface
cargo run --bin sms-web

# Test database connection
cargo run --bin test_db

# Run integration tests
cargo run --bin test_integration
```

### 📊 Metrics & Observability

The scraper includes comprehensive Prometheus-compatible metrics:

```bash
# Run complete pipeline demonstration with metrics
./demo-local.sh

# View scraper metrics in real-time
curl http://localhost:9898/metrics

# Start all services (GraphQL + Web)
docker-compose up -d

# Start services with scraper
docker-compose --profile scraper up -d

# Run one-off scraper job
docker-compose run --rm scraper-job
```

Metrics cover all pipeline phases:
- **Ingestion**: Raw data fetching, HTTP request metrics
- **Processing**: Parse → Normalize → Quality Gate → Enrich → Conflation → Catalog
- **Storage**: Database operations, connection health
- **Pipeline**: End-to-end processing times and success rates

See [METRICS.md](METRICS.md) for complete documentation.

## Configuration

Configuration is managed via multiple files:
- **`registry/sources/*.json`**: Individual venue/API configurations
- **`.env`**: Database credentials and environment variables
- **`config.toml`**: Rate limiting and processing settings
- **Environment variables**: `LIBSQL_URL`, `LIBSQL_AUTH_TOKEN`, `RUST_LOG`

## 🏆 Architecture Score: 5.0/5

**Strengths:**
- ✅ Loose coupling and clear abstractions
- ✅ Separation of concerns between ingestion and processing
- ✅ Extensible plugin-like architecture for new APIs
- ✅ Scalable asynchronous design
- ✅ **Full database persistence with graph schema**
- ✅ **Dual storage modes (in-memory + database)**
- ✅ **Complete ETL pipeline with audit logging**

**Future Enhancements:**
- ✅ ~~Persistent storage backend~~ → **Turso/libSQL implemented**
- ✅ ~~GraphQL API layer~~ → **Implemented with health checks**
- ✅ ~~Web dashboard~~ → **HTML interface with event/venue/artist browsing**
- Enhanced concurrent processing in processing stage
- External API integrations (Ticketmaster, Eventbrite, etc.)
- Advanced artist parsing with NLP
- Production deployment automation

## Archive

The historical “carpenter” implementation has been preserved for reference in archive/carpenter.rs. The live codebase no longer uses or references it.

## Development

The project uses standard Rust tooling:
- `cargo build` - Build the project
- `cargo test` - Run tests
- `cargo run` - Execute the scraper

Logs are written to `logs/` directory and excluded from version control.
