#!/bin/bash
# Test that the Ingester command pushes metrics correctly

set -e

echo "=== Testing Ingester Metrics Push ==="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if push gateway is running
echo "1. Checking push gateway..."
if ! curl -s -o /dev/null -w "%{http_code}" http://localhost:9091/metrics | grep -q "200"; then
    echo -e "${RED}✗ Push gateway not accessible${NC}"
    echo "Start it with: docker-compose up -d pushgateway"
    exit 1
fi
echo -e "${GREEN}✓ Push gateway is running${NC}"

# Clear metrics for test
echo
echo "2. Clearing test metrics..."
curl -X DELETE http://localhost:9091/metrics/job/sms_scraper/instance/ingester_test 2>/dev/null || true

# Run ingester with bypass cadence to ensure it runs
echo
echo "3. Running Ingester command..."
SMS_PUSHGATEWAY_URL=http://localhost:9091 \
    ./target/release/sms_scraper ingester \
    --apis blue_moon \
    --bypass-cadence 2>&1 | tee ingester_output.log | grep -E "(Pipeline|Pushing|metrics|Successfully)"

# Wait for metrics to propagate
sleep 2

# Check metrics
echo
echo "4. Checking for pushed metrics..."
METRICS=$(curl -s http://localhost:9091/metrics)

check_metric() {
    local metric=$1
    local description=$2
    if echo "$METRICS" | grep -q "$metric"; then
        echo -e "${GREEN}✓ $description${NC}"
        VALUE=$(echo "$METRICS" | grep "$metric" | head -1 | awk '{print $2}')
        echo "    Value: $VALUE"
        return 0
    else
        echo -e "${RED}✗ $description${NC}"
        return 1
    fi
}

echo
echo "Core Metrics:"
FOUND=0
TOTAL=0

# These metrics should be present after an ingester run
METRICS_TO_CHECK=(
    "sms_heartbeat_total:Heartbeat counter"
    "sms_sources_registry_loads_success_total:Registry loads"
    "sms_sources_requests_success_total:HTTP requests success"
    "sms_parser_parse_success_total:Parse operations"
    "sms_parser_records_extracted_total:Records extracted"
)

for metric_desc in "${METRICS_TO_CHECK[@]}"; do
    IFS=':' read -r metric description <<< "$metric_desc"
    ((TOTAL++))
    if check_metric "$metric" "$description"; then
        ((FOUND++))
    fi
done

echo
echo "=== Summary ==="
if [ $FOUND -eq $TOTAL ]; then
    echo -e "${GREEN}SUCCESS: All $TOTAL expected metrics were pushed!${NC}"
    exit 0
else
    echo -e "${YELLOW}PARTIAL: Found $FOUND/$TOTAL expected metrics${NC}"
    echo
    echo "Debug: All SMS metrics in push gateway:"
    curl -s http://localhost:9091/metrics | grep "^sms_" | cut -d' ' -f1 | sort -u | head -20
    exit 1
fi
