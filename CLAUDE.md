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
- Blue Moon Tavern (web crawler)
- Sea Monster Lounge (web crawler)

### Planned Data Sources (from migration plan)
**External APIs:**
- Ticketmaster, AXS, Dice, Eventbrite, Songkick, Bandsintown, Tixr, Venuepilot

**Custom Web Crawlers:**
- Darrell's Tavern, Little Red Hen, Skylark, The Royal Room

## Key Commands

### Build and Run
```bash
# Build the project
cargo build

# Run with different modes
cargo run -- ingester --apis blue_moon,sea_monster
# (former carpenter/run commands have been removed)
# Use ingester and server commands as needed

# Check for errors and warnings
cargo check
cargo clippy
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
- Basic project structure and dependencies
- EventApi trait and strategy pattern implementation
- Pipeline for data ingestion and processing
- Two working web crawlers (Blue Moon, Sea Monster)
- Logging and error handling infrastructure
- CLI interface with subcommands
- Raw data storage abstraction
- Processing framework for data transformation

### 🚧 In Progress
- Storage layer (currently in-memory, needs PostgreSQL integration)
- Processing logic (basic framework exists)

### 📋 Planned
- Database schema and SQLx integration
- Remaining API integrations (Ticketmaster, etc.)
- Remaining web crawlers
- Open mic generation logic
- Idempotency and duplicate detection
- Production deployment configuration

## File Structure
```
src/
├── main.rs              # CLI entry point and command handling
├── apis/                # Data source implementations
│   ├── mod.rs
│   ├── blue_moon.rs     # Blue Moon Tavern crawler
│   └── sea_monster.rs   # Sea Monster Lounge crawler
├── domain/              # Domain entities (venue, artist, event, etc.)
├── config.rs            # Configuration management
├── error.rs             # Error types and handling
├── logging.rs           # Logging setup
├── pipeline.rs          # Data flow orchestration
├── storage.rs           # Data persistence abstraction
└── types.rs             # Core data structures and traits
```

## Configuration
- Main config in `config.toml`
- Environment variables via `.env` file
- API-specific settings (delays, timeouts, etc.)

## Migration Notes
This project follows the migration plan in `../sms_server/sms_server/RUST_MIGRATION_PLAN.md` to ensure feature parity with the existing Python implementation while improving performance through async concurrency.