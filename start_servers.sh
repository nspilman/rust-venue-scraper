#!/bin/bash

# SMS Scraper Server Management Script
# This script manages both the GraphQL API server and the web frontend server

cd "$(dirname "$0")"

GRAPHQL_PORT=8080
WEB_PORT=3000
PIDS_FILE="server_pids.txt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to clean up processes on exit
cleanup() {
    echo -e "\n${YELLOW}🛑 Shutting down servers...${NC}"
    
    if [ -f "$PIDS_FILE" ]; then
        while read -r pid name; do
            if kill -0 "$pid" 2>/dev/null; then
                echo -e "${BLUE}Stopping $name (PID: $pid)${NC}"
                kill "$pid"
                sleep 1
                # Force kill if still running
                if kill -0 "$pid" 2>/dev/null; then
                    kill -9 "$pid" 2>/dev/null
                fi
            fi
        done < "$PIDS_FILE"
        rm -f "$PIDS_FILE"
    fi
    
    # Also kill any processes still running on our ports
    lsof -ti:$GRAPHQL_PORT | xargs kill -9 2>/dev/null || true
    lsof -ti:$WEB_PORT | xargs kill -9 2>/dev/null || true
    
    echo -e "${GREEN}✅ All servers stopped${NC}"
    exit 0
}

# Function to check if a port is in use
check_port() {
    local port=$1
    local name=$2
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null ; then
        echo -e "${RED}❌ Port $port is already in use by another process${NC}"
        echo -e "${YELLOW}   Kill the process with: lsof -ti:$port | xargs kill -9${NC}"
        return 1
    fi
    return 0
}

# Set up signal handlers for clean shutdown
trap cleanup SIGINT SIGTERM EXIT

echo -e "${BLUE}🚀 Starting SMS Scraper Servers${NC}"
echo -e "${BLUE}================================${NC}"

# Check if ports are available
if ! check_port $GRAPHQL_PORT "GraphQL server"; then
    exit 1
fi

if ! check_port $WEB_PORT "Web server"; then
    exit 1
fi

# Clear any existing PID file
rm -f "$PIDS_FILE"

# Start GraphQL server in background
echo -e "${YELLOW}📊 Starting GraphQL API server on port $GRAPHQL_PORT...${NC}"
./target/release/sms_scraper server --port $GRAPHQL_PORT --use-database &
GRAPHQL_PID=$!
echo "$GRAPHQL_PID GraphQL_Server" >> "$PIDS_FILE"

# Wait a moment for GraphQL server to start
sleep 3

# Check if GraphQL server started successfully
if ! kill -0 $GRAPHQL_PID 2>/dev/null; then
    echo -e "${RED}❌ Failed to start GraphQL server${NC}"
    exit 1
fi

echo -e "${GREEN}✅ GraphQL server started (PID: $GRAPHQL_PID)${NC}"

# Start Web server in background
echo -e "${YELLOW}🌐 Starting Web frontend server on port $WEB_PORT...${NC}"
cd web-server
./target/release/web-server &
WEB_PID=$!
cd ..
echo "$WEB_PID Web_Server" >> "$PIDS_FILE"

# Wait a moment for web server to start
sleep 2

# Check if web server started successfully
if ! kill -0 $WEB_PID 2>/dev/null; then
    echo -e "${RED}❌ Failed to start web server${NC}"
    cleanup
    exit 1
fi

echo -e "${GREEN}✅ Web server started (PID: $WEB_PID)${NC}"

echo -e "\n${GREEN}🎉 Both servers are running!${NC}"
echo -e "${BLUE}================================${NC}"
echo -e "${GREEN}📊 GraphQL API:     ${NC}http://localhost:$GRAPHQL_PORT/graphql"
echo -e "${GREEN}🎮 GraphiQL UI:     ${NC}http://localhost:$GRAPHQL_PORT/graphiql"
echo -e "${GREEN}💚 Health Check:    ${NC}http://localhost:$GRAPHQL_PORT/health"
echo -e "${GREEN}🌐 Web Frontend:    ${NC}http://localhost:$WEB_PORT"
echo -e "${BLUE}================================${NC}"
echo -e "${YELLOW}💡 Press Ctrl+C to stop both servers${NC}"
echo -e "${YELLOW}💡 Server PIDs are saved in: $PIDS_FILE${NC}"

# Keep the script running and monitor the servers
while true; do
    # Check if GraphQL server is still running
    if ! kill -0 $GRAPHQL_PID 2>/dev/null; then
        echo -e "${RED}❌ GraphQL server stopped unexpectedly${NC}"
        cleanup
        exit 1
    fi
    
    # Check if web server is still running
    if ! kill -0 $WEB_PID 2>/dev/null; then
        echo -e "${RED}❌ Web server stopped unexpectedly${NC}"
        cleanup
        exit 1
    fi
    
    sleep 5
done
