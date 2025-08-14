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

# Function to wait for service to be ready
wait_for_service() {
    local url=$1
    local service_name=$2
    local max_attempts=30
    local attempt=1
    
    echo_info "Waiting for $service_name to be ready..."
    while [ $attempt -le $max_attempts ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            echo_success "$service_name is ready!"
            return 0
        fi
        if [ $attempt -eq 30 ]; then
            echo_warning "$service_name did not become ready in time"
            return 1
        fi
        sleep 2
        attempt=$((attempt + 1))
    done
}

# Function to cleanup on exit
cleanup() {
    echo_info "Cleaning up..."
    if [ ! -z "$SERVER_PID" ]; then
        echo_info "Stopping local scraper server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    
    echo_info "Stopping Docker services..."
    docker-compose -f docker-compose-local.yml down
}
trap cleanup EXIT

echo_step "SMS Scraper Pipeline with Full Monitoring Stack"
echo "This demo runs the scraper locally with Prometheus + Grafana in Docker"
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

echo_step "Step 1: Start Monitoring Stack (Docker)"
echo_info "Starting Prometheus, Grafana, and PushGateway..."
docker-compose -f docker-compose-local.yml up -d

# Wait for monitoring services
wait_for_service "http://localhost:9090/-/ready" "Prometheus"
wait_for_service "http://localhost:9091" "PushGateway"
wait_for_service "http://localhost:3000" "Grafana"

echo_step "Step 2: Start Local Scraper Server"
echo_info "Starting SMS Scraper server locally with metrics on port 9898..."

# Start the server in background
$BINARY server --port 8080 &
SERVER_PID=$!

# Wait for local services
wait_for_service "http://localhost:8080/health" "SMS Scraper API"
wait_for_service "http://localhost:9898/metrics" "Metrics Endpoint"

echo_step "Step 3: Verify Prometheus is Scraping Local Metrics"
echo_info "Checking Prometheus targets..."
sleep 5  # Give Prometheus time to scrape

# Check Prometheus targets
echo "Prometheus target status:"
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job=="sms_scraper_local") | {job: .labels.job, instance: .labels.instance, health: .health, lastScrape: .lastScrape}'

echo_step "Step 4: Execute Pipeline to Generate Metrics"
echo_info "Running gateway-once to scrape data and generate metrics..."
$BINARY gateway-once --source-id blue_moon --bypass-cadence

echo_info "Running parser to process data..."
$BINARY parse --max 10

echo_step "Step 5: Run Deduplication Test"
echo_info "Running same scrape again to demonstrate deduplication metrics..."
$BINARY gateway-once --source-id blue_moon --bypass-cadence

echo_step "Step 6: Verify Metrics in Prometheus"
echo_info "Checking that metrics are available in Prometheus..."

# Wait a moment for Prometheus to scrape the new metrics
sleep 10

echo "SMS metrics in Prometheus:"
curl -s "http://localhost:9090/api/v1/query?query=\{__name__=~\"sms_.*\"\}" | jq '.data.result[] | .metric.__name__' | sort | uniq

echo_step "Step 7: Service Endpoints Summary"
echo ""
echo_info "🌐 All Services Running:"
echo "   • Local SMS Scraper API:   http://localhost:8080/graphql"
echo "   • Local Metrics Endpoint:  http://localhost:9898/metrics"
echo "   • Prometheus (Docker):     http://localhost:9090"
echo "   • Grafana (Docker):        http://localhost:3000 (admin/admin)"
echo "   • PushGateway (Docker):    http://localhost:9091"
echo ""

echo_step "Step 8: Grafana Setup Instructions"
echo_info "To view metrics in Grafana:"
echo "1. Visit http://localhost:3000"
echo "2. Login with admin/admin"
echo "3. Go to Explore or create a new dashboard"
echo "4. Use Prometheus as the data source (should be auto-configured)"
echo "5. Query SMS metrics with: sms_sources_requests_success_total"
echo ""

echo_step "Step 9: Sample Prometheus Queries"
echo_info "Try these queries in Prometheus (http://localhost:9090):"
echo ""
echo "All SMS metrics:"
echo "  {__name__=~\"sms_.*\"}"
echo ""
echo "Request success rate:"
echo "  rate(sms_sources_requests_success_total[5m])"
echo ""
echo "Gateway processing duration:"
echo "  sms_gateway_processing_duration_seconds"
echo ""
echo "Deduplication rate:"
echo "  sms_gateway_envelopes_deduplicated_total / (sms_gateway_envelopes_accepted_total + sms_gateway_envelopes_deduplicated_total)"
echo ""

echo_step "Demo Complete!"
echo_success "Full monitoring stack is now running with metrics!"
echo ""
echo_info "The setup includes:"
echo "  ✓ Local SMS Scraper with metrics endpoint"
echo "  ✓ Prometheus scraping local metrics"
echo "  ✓ Grafana for visualization"
echo "  ✓ Complete pipeline metrics"
echo "  ✓ Deduplication tracking"
echo ""
echo_warning "Services will continue running until you press Ctrl+C"
echo ""

# Keep running until interrupted
wait $SERVER_PID
