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
        echo "Attempt $attempt/$max_attempts - $service_name not ready yet..."
        sleep 2
        attempt=$((attempt + 1))
    done
    
    echo_warning "$service_name did not become ready in time"
    return 1
}

echo_step "SMS Scraper Pipeline Demonstration"
echo "This script demonstrates the complete venue scraping pipeline with metrics"
echo ""

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo_warning "docker-compose not found. Please install docker-compose to continue."
    exit 1
fi

echo_step "Step 1: Build and Start Services"
echo_info "Building the scraper image and starting all services..."
docker-compose build
docker-compose up -d

echo_step "Step 2: Wait for Services to Be Ready"
wait_for_service "http://localhost:8080/health" "SMS Scraper API"
wait_for_service "http://localhost:9898/metrics" "Metrics Endpoint"
wait_for_service "http://localhost:9090/-/ready" "Prometheus"
wait_for_service "http://localhost:9091" "PushGateway"

echo_step "Step 3: Verify Initial Metrics Setup"
echo_info "Checking that metrics are being exported..."
echo "Available metrics (first 10):"
curl -s http://localhost:9898/metrics | grep "^sms_" | head -10
echo ""

echo_step "Step 4: Execute Gateway Phase (Scraping)"
echo_info "Running gateway-once to scrape Blue Moon venue data..."
echo "This will generate SourcesMetrics and GatewayMetrics..."

# Run the scraping command using docker exec
docker exec sms_scraper /usr/local/bin/sms_scraper gateway-once --source-id blue_moon --bypass-cadence

echo_success "Gateway phase completed - data scraped and stored in CAS"

echo_step "Step 5: Execute Parser Phase"
echo_info "Running parser to process the scraped data..."
echo "This will generate ParserMetrics and IngestLogMetrics..."

# Run the parsing command
docker exec sms_scraper /usr/local/bin/sms_scraper parse --max 10

echo_success "Parser phase completed - structured data extracted"

echo_step "Step 6: Check Updated Metrics"
echo_info "Viewing metrics after pipeline execution..."
echo ""
echo "Sources metrics (request success/error):"
curl -s http://localhost:9898/metrics | grep "sms_sources_requests"
echo ""
echo "Gateway metrics (envelopes accepted/deduplicated):"
curl -s http://localhost:9898/metrics | grep "sms_gateway_envelopes"
echo ""
echo "Parser metrics (parsing success/duration):"
curl -s http://localhost:9898/metrics | grep "sms_parser_"
echo ""
echo "Ingest Log metrics (writes):"
curl -s http://localhost:9898/metrics | grep "sms_ingest_log_writes"
echo ""

echo_step "Step 7: Demonstrate Deduplication"
echo_info "Running the same scrape again to show deduplication metrics..."
docker exec sms_scraper /usr/local/bin/sms_scraper gateway-once --source-id blue_moon --bypass-cadence

echo ""
echo "Deduplication metrics:"
curl -s http://localhost:9898/metrics | grep "sms_gateway_envelopes_deduplicated"
echo ""

echo_step "Step 8: Show Prometheus Integration"
echo_info "Checking Prometheus has scraped our metrics..."
echo "Prometheus targets status:"
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {job: .labels.job, health: .health, lastScrape: .lastScrape}'
echo ""

echo_step "Step 9: Available Endpoints Summary"
echo ""
echo_info "🌐 Service Endpoints:"
echo "   • SMS Scraper API:     http://localhost:8080/graphql"
echo "   • GraphiQL UI:         http://localhost:8080/graphiql" 
echo "   • Health Check:        http://localhost:8080/health"
echo "   • Metrics Endpoint:    http://localhost:9898/metrics"
echo "   • Prometheus:          http://localhost:9090"
echo "   • PushGateway:         http://localhost:9091"
echo ""

echo_step "Step 10: Sample Queries"
echo_info "Example metric queries you can run:"
echo ""
echo "All SMS metrics:"
echo "curl http://localhost:9898/metrics | grep '^sms_'"
echo ""
echo "Request success rate over time (PromQL):"
echo "rate(sms_sources_requests_success_total[5m])"
echo ""
echo "Gateway processing duration histogram:"
echo "sms_gateway_processing_duration_seconds"
echo ""

if command -v docker-compose &> /dev/null; then
    echo_step "Optional: Start Grafana for Visualization"
    echo_info "To start Grafana dashboard (optional):"
    echo "docker-compose --profile observability up -d grafana"
    echo "Then visit: http://localhost:3000 (admin/admin)"
fi

echo ""
echo_step "Demo Complete!"
echo_success "The SMS Scraper pipeline is now running with full metrics integration"
echo_info "The pipeline demonstrates:"
echo "  ✓ Phase-based metrics collection (Sources, Gateway, Parser, IngestLog)"
echo "  ✓ Prometheus-compatible metrics export"
echo "  ✓ Real-time monitoring of scraping operations"
echo "  ✓ Deduplication tracking"
echo "  ✓ Performance metrics (durations, throughput)"
echo ""
echo_warning "To stop all services: docker-compose down"
echo_warning "To view logs: docker-compose logs -f scraper"
