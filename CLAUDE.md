# SMS Server Rust - Context for Claude

## Project Overview
This is a Rust-based event data scraper for the Seattle Music Scene (SMS) project, migrating from a Python Django implementation. The system follows an ETL (Extract, Transform, Load) architecture with two main components:

1. **Ingester**: Fetches raw, unprocessed data from external sources
2. Processing: Cleans, transforms, and loads raw data into structured database models

## Current Architecture

### Core Components
- **Pipeline**: Orchestrates the data flow from raw ingestion to processed output
- **EventApi Trait**: Common interface for all data sources (APIs and web crawlers)
- **Storage**: Abstraction layer for data persistence (currently in-memory, will connect to PostgreSQL)
- Processing: Data processing and cleaning component
- **Types**: Core data structures and models

### Implemented Data Sources
**Web Crawlers:**
- Blue Moon Tavern (Wix calendar API)
- Sea Monster Lounge (venue-specific crawler)
- Darrell's Tavern (venue-specific crawler)
- Barboza (venue-specific crawler)
- KEXP Events (event listing crawler)
- Neumos (venue-specific crawler)
- Conor Byrne (venue-specific scraper with GraphQL parser)

**Source Registry:**
- Configuration-driven approach using JSON source definitions in `registry/sources/`
- Each source has endpoints, rate limits, auth config, and parsing plans
- Centralized management of all data source configurations

### Planned Data Sources (from migration plan)
**External APIs:**
- Ticketmaster, AXS, Dice, Eventbrite, Songkick, Bandsintown, Tixr, Venuepilot

**Custom Web Crawlers:**
- Little Red Hen, Skylark, The Royal Room

## Key Commands

### Build and Run
```bash
# Build the project
cargo build

# Run with different modes
cargo run -- ingester --apis blue_moon,sea_monster,darrells_tavern,barboza,kexp,neumos,conor_byrne
# Run ingester with database persistence
cargo run -- ingester --apis blue_moon --use-database

# Run GraphQL server (in-memory or database mode)
cargo run -- server --port 8080
cargo run -- server --port 8080 --use-database

# Check for errors and warnings
cargo check
cargo clippy

# Makefile shortcuts
make ingest              # Run ingester for all crawlers (in-memory)
make ingest-db          # Run ingester with database
make run-graphql        # Run GraphQL server (in-memory)
make run-graphql-db     # Run GraphQL server (database)
make start              # Start both GraphQL and web servers
make test               # Run tests
make clippy             # Run lints with warnings as errors
```

### Docker Operations
```bash
# Build and run all services (production-like)
docker-compose up -d

# Build and run local development setup (monitoring only)
docker-compose -f docker-compose-local.yml up -d

# Run one-off ingester job
docker-compose run --rm ingester

# Stop all services
docker-compose down

# Clean up Docker resources
docker system prune -f
docker image prune -f

# View logs
docker-compose logs -f scraper
docker-compose logs -f web

# Rebuild and restart after code changes
docker-compose build && docker-compose up -d
```

### Testing
```bash
# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Development
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run lints
cargo clippy -- -D warnings
```

## Current Status

### ✅ Completed
- **Architecture:** EventApi trait, clean architecture, ETL pipeline
- **Data Sources:** 7 working crawlers (Blue Moon, Sea Monster, Darrell's, Barboza, KEXP, Neumos, Conor Byrne)
- **Database:** Full Turso/libSQL integration with graph schema and migrations  
- **Storage:** Dual-mode (in-memory + database) with abstractions
- **Pipeline:** Complete ingestion → parsing → normalization → conflation → enrichment
- **Server:** GraphQL API with health checks and metrics endpoints
- **Web Interface:** Separate web server with HTML templates for event/venue/artist browsing
- **Observability:** Comprehensive Prometheus metrics, Grafana dashboards, logging
- **DevOps:** Docker containerization, docker-compose stack with monitoring
- **Testing:** Integration tests, schema validation, envelope testing
- **Tooling:** Multiple utility binaries, Makefile shortcuts, development scripts

### 🚧 In Progress  
- Advanced artist parsing and relationship detection
- Quality gates and data validation enhancements
- Performance optimizations for large datasets

### 📋 Planned
- External API integrations (Ticketmaster, Eventbrite, etc.)
- Remaining venue crawlers (Little Red Hen, Skylark, Royal Room)
- Open mic event generation logic
- Enhanced deduplication and conflation algorithms
- Production deployment automation

## File Structure
```
src/
├── main.rs              # CLI entry point and command handling
├── apis/                # Data source implementations
│   ├── mod.rs
│   ├── blue_moon.rs     # Blue Moon Tavern (Wix calendar API)
│   ├── sea_monster.rs   # Sea Monster Lounge crawler
│   ├── darrells_tavern.rs # Darrell's Tavern crawler
│   ├── barboza.rs       # Barboza venue crawler
│   ├── kexp.rs          # KEXP events crawler
│   ├── neumos.rs        # Neumos venue crawler
│   └── conor_byrne.rs   # Conor Byrne venue crawler
├── app/                 # Application use cases and ports
├── common/              # Shared types, errors, constants
├── domain/              # Domain entities (venue, artist, event, etc.)
├── graphql/             # GraphQL server and resolvers
├── infra/               # Infrastructure adapters
├── observability/       # Logging, metrics, and monitoring
├── pipeline/            # ETL pipeline components
│   ├── ingestion/       # Data ingestion (envelope, gateway, registry)
│   ├── processing/      # Data processing (parse, normalize, conflate)
│   └── storage/         # Storage abstractions and implementations
├── scrapers/            # Alternative scraper implementations
└── server.rs            # HTTP server setup
```

## Additional Important Files
```
registry/sources/        # JSON configuration for all data sources
migrations/              # Database schema migrations (SQLite/libSQL)
tests/                   # Integration tests and test resources
ops/                     # Operational configs (Prometheus, Grafana)
web-server/              # Separate web UI server project
```

## Docker Architecture

### Services Overview
The system runs multiple decoupled containerized services:

#### Core Application Services
- **graphql** (`sms-graphql`): Lightweight GraphQL API server (port 8080) - reads from shared database
- **web** (`sms-web`): Web frontend server (port 3001) - communicates with GraphQL via HTTP
- **scraper** (`sms-scraper`): Full scraper with processing pipeline (port 9898 for metrics) - writes to shared database
- **scraper-job**: One-off scraper job (disabled profile, run manually)

#### Monitoring Stack
- **prometheus**: Metrics collection and storage (port 9090)
- **pushgateway**: Prometheus push gateway for batch jobs (port 9091)  
- **grafana**: Metrics visualization dashboard (port 3000, admin/admin)

### Environment Variables
- `LIBSQL_URL`: Database connection URL
- `LIBSQL_AUTH_TOKEN`: Database authentication token
- `RUST_LOG`: Log level configuration
- `SMS_PUSHGATEWAY_URL`: Pushgateway endpoint for metrics

### Service Architecture
- **Decoupled Design**: GraphQL API, web frontend, and scraper run independently
- **Shared Database**: All services communicate via the shared Turso/libSQL database
- **Service Profiles**: Scraper uses profile `["scraper"]` to run only when explicitly requested
- **HTTP Communication**: Web frontend communicates with GraphQL API via HTTP

### Configuration Files
- `docker-compose.yml`: Full production-like stack with decoupled services
- `docker-compose-local.yml`: Local development (monitoring only)
- `ops/prometheus.yml`: Prometheus configuration
- `ops/grafana/provisioning/`: Grafana dashboards and datasources

### Health Checks
- GraphQL service includes health check endpoint at `/health`
- Services have proper dependency ordering and restart policies
- Web service depends on GraphQL service health check

## Database Schema

### Graph-Based Architecture
The system uses a graph database schema with two main tables:

**Nodes Table:** Stores all entities (venues, artists, events, external_ids)
- `id`: UUID primary key  
- `label`: Entity type (venue, artist, event, external_id)
- `data`: JSON payload with entity-specific fields
- Unique constraints ensure no duplicates (venue slug, artist name_slug, event composite key)

**Edges Table:** Stores relationships between entities
- `source_id` → `target_id` with `relation` type
- Examples: artist "performs_at" venue, event "happening_on" date
- Prevents duplicate relationships with unique constraints

### Database Features
- **Dual Mode:** In-memory (development) or Turso/libSQL (production)
- **Migrations:** SQL schema files in `migrations/` directory
- **JSON Storage:** Flexible schema using SQLite JSON functions
- **Triggers:** Auto-updating timestamps on modifications

## Configuration

### Configuration Files
- **`config.toml`**: API-specific settings (delays, timeouts, geo points)
- **`.env`**: Database credentials, API keys, environment variables  
- **`registry/sources/*.json`**: Individual data source configurations
- **`schemas/*.json`**: JSON schema validation files

### Environment Variables
```bash
# Database (Turso/libSQL)
LIBSQL_URL=libsql://your-database.turso.io
LIBSQL_AUTH_TOKEN=your_auth_token

# API Keys
TICKETMASTER_API_KEY=your_key
EVENTBRITE_API_TOKEN=your_token

# Runtime Configuration  
RUST_LOG=info
OUTPUT_DIRECTORY=output
LOG_DIRECTORY=logs
```

### Binary Programs
The project includes multiple utility binaries:
- `sms_scraper`: Main CLI (ingester, server modes)
- `build-dashboard`: Grafana dashboard generator
- `validate-envelope`: JSON schema validation
- `clear-database`: Database cleanup utility
- `diagnose-artist-links`: Debugging tool for artist relationships

## Migration Notes
This project follows the migration plan in `../sms_server/sms_server/RUST_MIGRATION_PLAN.md` to ensure feature parity with the existing Python implementation while improving performance through async concurrency.