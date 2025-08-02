#!/bin/bash

# SMS Scraper Server Stop Script
# This script safely stops both the GraphQL API server and the web frontend server

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

echo -e "${BLUE}🛑 Stopping SMS Scraper Servers${NC}"
echo -e "${BLUE}================================${NC}"

# Stop servers using PID file
if [ -f "$PIDS_FILE" ]; then
    echo -e "${YELLOW}📋 Found server PID file, stopping registered processes...${NC}"
    
    while read -r pid name; do
        if [ -n "$pid" ] && [ -n "$name" ]; then
            if kill -0 "$pid" 2>/dev/null; then
                echo -e "${BLUE}🔄 Stopping $name (PID: $pid)${NC}"
                kill "$pid"
                sleep 1
                
                # Check if process is still running, force kill if necessary
                if kill -0 "$pid" 2>/dev/null; then
                    echo -e "${YELLOW}⚠️  Process $pid still running, force killing...${NC}"
                    kill -9 "$pid" 2>/dev/null
                    sleep 1
                fi
                
                if ! kill -0 "$pid" 2>/dev/null; then
                    echo -e "${GREEN}✅ $name stopped successfully${NC}"
                else
                    echo -e "${RED}❌ Failed to stop $name${NC}"
                fi
            else
                echo -e "${YELLOW}⚠️  $name (PID: $pid) was not running${NC}"
            fi
        fi
    done < "$PIDS_FILE"
    
    rm -f "$PIDS_FILE"
    echo -e "${GREEN}🗑️  Removed PID file${NC}"
else
    echo -e "${YELLOW}⚠️  No PID file found${NC}"
fi

# Kill any remaining processes on our ports
echo -e "${YELLOW}🔍 Checking for any remaining processes on ports...${NC}"

# Check GraphQL port
GRAPHQL_PIDS=$(lsof -ti:$GRAPHQL_PORT 2>/dev/null)
if [ -n "$GRAPHQL_PIDS" ]; then
    echo -e "${BLUE}🔄 Killing processes on GraphQL port $GRAPHQL_PORT${NC}"
    echo "$GRAPHQL_PIDS" | xargs kill -9 2>/dev/null
    echo -e "${GREEN}✅ GraphQL port $GRAPHQL_PORT cleared${NC}"
fi

# Check Web port
WEB_PIDS=$(lsof -ti:$WEB_PORT 2>/dev/null)
if [ -n "$WEB_PIDS" ]; then
    echo -e "${BLUE}🔄 Killing processes on Web port $WEB_PORT${NC}"
    echo "$WEB_PIDS" | xargs kill -9 2>/dev/null
    echo -e "${GREEN}✅ Web port $WEB_PORT cleared${NC}"
fi

# Final verification
echo -e "${YELLOW}🔍 Final verification...${NC}"

REMAINING_GRAPHQL=$(lsof -ti:$GRAPHQL_PORT 2>/dev/null)
REMAINING_WEB=$(lsof -ti:$WEB_PORT 2>/dev/null)

if [ -z "$REMAINING_GRAPHQL" ] && [ -z "$REMAINING_WEB" ]; then
    echo -e "${GREEN}✅ All servers stopped successfully!${NC}"
    echo -e "${GREEN}📊 GraphQL port $GRAPHQL_PORT: Available${NC}"
    echo -e "${GREEN}🌐 Web port $WEB_PORT: Available${NC}"
else
    echo -e "${RED}❌ Some processes may still be running:${NC}"
    [ -n "$REMAINING_GRAPHQL" ] && echo -e "${RED}   - GraphQL port $GRAPHQL_PORT: $REMAINING_GRAPHQL${NC}"
    [ -n "$REMAINING_WEB" ] && echo -e "${RED}   - Web port $WEB_PORT: $REMAINING_WEB${NC}"
fi

echo -e "${BLUE}================================${NC}"
