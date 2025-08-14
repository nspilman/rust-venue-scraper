#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo_step() {
    echo -e "${BLUE}==== $1 ====${NC}"
}

echo_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

echo_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

echo_info() {
    echo -e "ℹ️  $1"
}

echo_step "SMS Scraper Pipeline Demonstration (Local)"
echo "This script demonstrates the complete venue scraping pipeline with metrics using local build"
echo ""

# Check if the binary exists
if [ ! -f "./target/release/sms_scraper" ]; then
    echo_warning "Binary not found. Building release version..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo_warning "Build failed. Trying without release mode..."
        cargo build
        BINARY="./target/debug/sms_scraper"
    else
        BINARY="./target/release/sms_scraper"
        echo_success "Release binary built successfully!"
    fi
else
    BINARY="./target/release/sms_scraper"
    echo_success "Using existing release binary!"
fi

echo_step "Step 1: Start Metrics Server"
echo_info "Starting the SMS Scraper server in the background with metrics endpoint on port 9898..."

# Start the server in background and capture its PID
$BINARY server --port 8080 &
SERVER_PID=$!

# Function to cleanup on exit
cleanup() {
    if [ ! -z "$SERVER_PID" ]; then
        echo_info "Stopping server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Wait for server to start
echo_info "Waiting for server to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo_success "Server is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo_warning "Server did not start in time"
        exit 1
    fi
    sleep 1
done

# Also wait for metrics endpoint
for i in {1..10}; do
    if curl -s http://localhost:9898/metrics > /dev/null 2>&1; then
        echo_success "Metrics endpoint is ready!"
        break
    fi
    sleep 1
done

echo_step "Step 2: Verify Initial Metrics Setup"
echo_info "Checking that metrics are being exported..."
echo "Available SMS metrics (showing first 10):"
curl -s http://localhost:9898/metrics | grep "^sms_" | head -10
echo ""

echo_step "Step 3: Execute Gateway Phase (Scraping)"
echo_info "Running gateway-once to scrape Blue Moon venue data..."
echo "This will generate SourcesMetrics and GatewayMetrics..."

$BINARY gateway-once --source-id blue_moon --bypass-cadence

echo_success "Gateway phase completed - data scraped and stored in CAS"

echo_step "Step 4: Check Metrics After Scraping"
echo_info "Viewing metrics after scraping operation..."
echo ""
echo "Sources metrics (request success):"
curl -s http://localhost:9898/metrics | grep "sms_sources_requests_success"
echo ""
echo "Gateway metrics (envelopes accepted):"
curl -s http://localhost:9898/metrics | grep "sms_gateway_envelopes_accepted"
echo ""

echo_step "Step 5: Execute Parser Phase"
echo_info "Running parser to process the scraped data..."
echo "This will generate ParserMetrics and IngestLogMetrics..."

$BINARY parse --max 10

echo_success "Parser phase completed - structured data extracted"

echo_step "Step 6: Check All Pipeline Metrics"
echo_info "Viewing metrics after full pipeline execution..."
echo ""
echo "Sources metrics:"
curl -s http://localhost:9898/metrics | grep "sms_sources_" | head -5
echo ""
echo "Gateway metrics:"
curl -s http://localhost:9898/metrics | grep "sms_gateway_" | head -5
echo ""
echo "Parser metrics:"
curl -s http://localhost:9898/metrics | grep "sms_parser_" | head -5
echo ""
echo "Ingest Log metrics:"
curl -s http://localhost:9898/metrics | grep "sms_ingest_log_" | head -5
echo ""

echo_step "Step 7: Demonstrate Deduplication"
echo_info "Running the same scrape again to show deduplication metrics..."
$BINARY gateway-once --source-id blue_moon --bypass-cadence

echo ""
echo "Deduplication metrics (should show deduplicated envelopes):"
curl -s http://localhost:9898/metrics | grep "sms_gateway_envelopes_deduplicated"
echo ""

echo_step "Step 8: Full Metrics Summary"
echo_info "Complete metrics summary:"
echo ""
echo "Total SMS metrics:"
TOTAL_METRICS=$(curl -s http://localhost:9898/metrics | grep "^sms_" | wc -l)
echo "Found $TOTAL_METRICS SMS metrics"
echo ""
echo "Metrics by phase:"
echo "• Sources: $(curl -s http://localhost:9898/metrics | grep "^sms_sources_" | wc -l)"
echo "• Gateway: $(curl -s http://localhost:9898/metrics | grep "^sms_gateway_" | wc -l)"
echo "• Parser: $(curl -s http://localhost:9898/metrics | grep "^sms_parser_" | wc -l)"
echo "• Ingest Log: $(curl -s http://localhost:9898/metrics | grep "^sms_ingest_log_" | wc -l)"
echo ""

echo_step "Step 9: Available Endpoints"
echo_info "Service endpoints (running locally):"
echo "   • SMS Scraper API:     http://localhost:8080/graphql"
echo "   • GraphiQL UI:         http://localhost:8080/graphiql"
echo "   • Health Check:        http://localhost:8080/health"  
echo "   • Metrics Endpoint:    http://localhost:9898/metrics"
echo ""

echo_step "Step 10: Sample Metrics Queries"
echo_info "You can run these commands to explore the metrics:"
echo ""
echo "View all SMS metrics:"
echo "curl http://localhost:9898/metrics | grep '^sms_'"
echo ""
echo "View only counter metrics:"
echo "curl http://localhost:9898/metrics | grep 'sms_.*_total'"
echo ""
echo "View only histogram metrics:"
echo "curl http://localhost:9898/metrics | grep 'sms_.*_seconds'"
echo ""
echo "Count metrics by phase:"
echo "curl -s http://localhost:9898/metrics | grep '^sms_' | cut -d_ -f2 | sort | uniq -c"
echo ""

echo_step "Demo Complete!"
echo_success "The SMS Scraper pipeline is now running with full metrics integration"
echo_info "The demo showed:"
echo "  ✓ Phase-based metrics collection (Sources, Gateway, Parser, IngestLog)"
echo "  ✓ Prometheus-compatible metrics export on http://localhost:9898/metrics"
echo "  ✓ Real-time monitoring of scraping operations"
echo "  ✓ Deduplication tracking"
echo "  ✓ Performance metrics (durations, throughput, sizes)"
echo ""
echo_info "The server will continue running. Press Ctrl+C to stop."
echo ""

# Keep the server running until user interrupts
wait $SERVER_PID
