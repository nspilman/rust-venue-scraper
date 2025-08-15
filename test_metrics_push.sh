#!/bin/bash
# Test script to verify metrics are being pushed to the push gateway

set -e

echo "=== Metrics Push Gateway Test ==="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if push gateway is running
echo "1. Checking if push gateway is accessible..."
if curl -s -o /dev/null -w "%{http_code}" http://localhost:9091/metrics | grep -q "200"; then
    echo -e "${GREEN}✓ Push gateway is running${NC}"
else
    echo -e "${RED}✗ Push gateway is not accessible at http://localhost:9091${NC}"
    echo "  Please start it with: docker compose up -d pushgateway"
    exit 1
fi

# Clear existing metrics for our test instance
echo
echo "2. Clearing existing metrics for test instance..."
curl -X DELETE http://localhost:9091/metrics/job/sms_scraper/instance/test_metrics 2>/dev/null || true

# Build the project
echo
echo "3. Building the project..."
cargo build --release 2>&1 | tail -5

# Run a simple command that should push metrics
echo
echo "4. Running GatewayOnce command to generate and push metrics..."
SMS_PUSHGATEWAY_URL=http://localhost:9091 ./target/release/sms_scraper gateway-once \
    --source-id blue_moon \
    --bypass-cadence 2>&1 | grep -E "(Rendered|pushed|metrics|Success)"

# Wait a moment for metrics to propagate
sleep 2

# Check if metrics were pushed
echo
echo "5. Checking push gateway for our metrics..."
METRICS=$(curl -s http://localhost:9091/metrics)

echo "Looking for key metrics:"
echo

check_metric() {
    local metric_name=$1
    if echo "$METRICS" | grep -q "$metric_name"; then
        echo -e "${GREEN}✓ Found: $metric_name${NC}"
        echo "$METRICS" | grep "$metric_name" | head -1 | sed 's/^/    /'
        return 0
    else
        echo -e "${RED}✗ Missing: $metric_name${NC}"
        return 1
    fi
}

# Check for various metric types
FOUND_COUNT=0
TOTAL_COUNT=0

# Core metrics we expect to see
EXPECTED_METRICS=(
    "sms_heartbeat_total"
    "sms_sources_requests_success_total"
    "sms_sources_request_duration_seconds"
    "sms_gateway_envelopes_accepted_total"
    "sms_gateway_processing_duration_seconds"
    "sms_push_timestamp_ms"
)

for metric in "${EXPECTED_METRICS[@]}"; do
    ((TOTAL_COUNT++))
    if check_metric "$metric"; then
        ((FOUND_COUNT++))
    fi
done

echo
echo "=== Test Results ==="
if [ $FOUND_COUNT -eq $TOTAL_COUNT ]; then
    echo -e "${GREEN}SUCCESS: All expected metrics ($FOUND_COUNT/$TOTAL_COUNT) were pushed to the gateway!${NC}"
else
    echo -e "${YELLOW}PARTIAL: Found $FOUND_COUNT/$TOTAL_COUNT expected metrics${NC}"
    echo
    echo "To debug, check the full metrics output:"
    echo "  curl http://localhost:9091/metrics | grep sms_"
fi

echo
echo "6. Checking metric details for test instance..."
echo "Metrics specific to our test instance:"
curl -s http://localhost:9091/metrics | grep 'instance="test_metrics"' | head -5

echo
echo "=== End of Test ===" 
