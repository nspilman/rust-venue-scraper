# Server Management

This document explains how to run both the GraphQL API server and the web frontend server for the SMS Scraper project.

## Quick Start

### Start Both Servers
```bash
./start_servers.sh
```
This will start:
- **GraphQL API Server** on port 8080
- **Web Frontend Server** on port 3000

### Stop Both Servers
```bash
./stop_servers.sh
```
Or simply press `Ctrl+C` in the terminal where `start_servers.sh` is running.

## Server Details

### GraphQL API Server (Port 8080)
- **Main API**: http://localhost:8080/graphql
- **GraphiQL UI**: http://localhost:8080/graphiql (interactive query interface)
- **Playground**: http://localhost:8080/playground (alternative UI)
- **Health Check**: http://localhost:8080/health

### Web Frontend Server (Port 3000)
- **Web Interface**: http://localhost:3000
- Provides a user-friendly web interface to browse events
- Connects to the GraphQL API automatically

## Features

✅ **Automatic Process Management**: Both servers are managed as background processes
✅ **Clean Shutdown**: Pressing Ctrl+C cleanly stops both servers
✅ **Port Conflict Detection**: Scripts check for port conflicts before starting
✅ **Process Monitoring**: Monitors server health and restarts if needed
✅ **Database Integration**: GraphQL server uses Turso database for persistence

## Manual Server Management

If you need to run servers individually:

### GraphQL Server Only
```bash
./target/release/sms_scraper server --port 8080 --use-database
```

### Web Server Only (requires GraphQL server running)
```bash
cd web-server
./target/release/web-server
```

## Troubleshooting

### Port Already in Use
If you see "Port X is already in use", run:
```bash
./stop_servers.sh
```
This will kill any processes using the required ports.

### Manual Process Cleanup
If processes get stuck:
```bash
# Kill GraphQL server processes
lsof -ti:8080 | xargs kill -9

# Kill web server processes  
lsof -ti:3000 | xargs kill -9
```

### Rebuilding After Code Changes
```bash
# Rebuild main binary
cargo build --release

# Rebuild web server
cd web-server && cargo build --release
```

## Process Information

The `start_servers.sh` script saves process IDs in `server_pids.txt` for clean shutdown management.

## Environment Requirements

- Rust toolchain installed
- Environment variables for database (if using `--use-database`):
  - `LIBSQL_URL`: Your Turso database URL
  - `LIBSQL_AUTH_TOKEN`: Your Turso auth token
