#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== SMS Scraper Metrics End-to-End Test ===${NC}"
echo

# Check if services are running
echo -e "${YELLOW}Checking services...${NC}"

# Check Pushgateway
if curl -s http://localhost:9091/metrics > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Pushgateway is running on :9091${NC}"
else
    echo -e "${RED}✗ Pushgateway is not running on :9091${NC}"
    echo "  Please start with: docker-compose up pushgateway"
    exit 1
fi

# Check Prometheus
if curl -s http://localhost:9090/api/v1/query?query=up > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Prometheus is running on :9090${NC}"
else
    echo -e "${RED}✗ Prometheus is not running on :9090${NC}"
    echo "  Please start with: docker-compose up prometheus"
    exit 1
fi

echo

# Run the gateway-once command (which pushes metrics)
echo -e "${YELLOW}Running gateway ingestion with metrics push...${NC}"
RUST_LOG=info cargo run --bin sms_scraper -- gateway-once \
    --source-id blue_moon \
    --bypass-cadence 2>&1 | tee /tmp/gateway_output.log

# Check if it succeeded
if grep -q "✅ Accepted envelope" /tmp/gateway_output.log; then
    echo -e "${GREEN}✓ Gateway ingestion succeeded${NC}"
    
    # Extract metrics from output
    ENVELOPE_ID=$(grep "✅ Accepted envelope" /tmp/gateway_output.log | sed -n 's/.*envelope \([^ ]*\).*/\1/p')
    BYTES=$(grep "✅ Accepted envelope" /tmp/gateway_output.log | sed -n 's/.* (\([0-9]*\) bytes.*/\1/p')
    echo "  Envelope ID: $ENVELOPE_ID"
    echo "  Bytes: $BYTES"
else
    echo -e "${RED}✗ Gateway ingestion failed${NC}"
    exit 1
fi

echo
echo -e "${YELLOW}Waiting for metrics to be scraped (5 seconds)...${NC}"
sleep 5

echo
echo -e "${YELLOW}Checking Pushgateway metrics...${NC}"

# Check if metrics exist in Pushgateway
if curl -s http://localhost:9091/metrics | grep -q "sms_ingest_timestamp_ms"; then
    echo -e "${GREEN}✓ Metrics found in Pushgateway${NC}"
    
    # Get specific metrics
    TIMESTAMP=$(curl -s http://localhost:9091/metrics | grep "^sms_ingest_timestamp_ms" | awk '{print $2}')
    INGEST_BYTES=$(curl -s http://localhost:9091/metrics | grep "^sms_ingest_bytes" | awk '{print $2}')
    DURATION=$(curl -s http://localhost:9091/metrics | grep "^sms_ingest_duration_seconds" | awk '{print $2}')
    SUCCESS=$(curl -s http://localhost:9091/metrics | grep "^sms_ingest_success" | awk '{print $2}')
    
    echo "  Last ingest timestamp: $TIMESTAMP"
    echo "  Bytes ingested: $INGEST_BYTES"
    echo "  Duration: $DURATION seconds"
    echo "  Success: $SUCCESS"
else
    echo -e "${RED}✗ No metrics found in Pushgateway${NC}"
    echo "  Checking what metrics are available:"
    curl -s http://localhost:9091/metrics | grep -E "^[a-z]" | head -10
fi

echo
echo -e "${YELLOW}Checking Prometheus for scraped metrics...${NC}"

# Query Prometheus for our metrics
PROM_RESULT=$(curl -s "http://localhost:9090/api/v1/query?query=sms_ingest_success" | jq -r '.status')
if [ "$PROM_RESULT" = "success" ]; then
    VALUE=$(curl -s "http://localhost:9090/api/v1/query?query=sms_ingest_success" | jq -r '.data.result[0].value[1]' 2>/dev/null || echo "null")
    if [ "$VALUE" != "null" ] && [ "$VALUE" != "" ]; then
        echo -e "${GREEN}✓ Metrics successfully scraped by Prometheus${NC}"
        echo "  sms_ingest_success = $VALUE"
        
        # Check other metrics
        echo
        echo "  Other metrics in Prometheus:"
        for metric in sms_ingest_bytes sms_ingest_duration_seconds sms_ingest_timestamp_ms; do
            VAL=$(curl -s "http://localhost:9090/api/v1/query?query=$metric" | jq -r '.data.result[0].value[1]' 2>/dev/null || echo "N/A")
            echo "    $metric = $VAL"
        done
    else
        echo -e "${YELLOW}⚠ Prometheus can query but no data returned${NC}"
        echo "  This might mean Prometheus hasn't scraped yet, or the job/instance labels don't match"
    fi
else
    echo -e "${RED}✗ Failed to query Prometheus${NC}"
fi

echo
echo -e "${YELLOW}Running standard ingester (no push)...${NC}"
cargo run --bin sms_scraper -- ingester --bypass-cadence 2>&1 | tee /tmp/ingester_output.log

# Check results
if grep -q "Pipeline Results" /tmp/ingester_output.log; then
    echo -e "${GREEN}✓ Ingester pipeline completed${NC}"
    
    # Extract stats
    TOTAL=$(grep "Total events:" /tmp/ingester_output.log | tail -1 | awk '{print $3}')
    PROCESSED=$(grep "Processed:" /tmp/ingester_output.log | tail -1 | awk '{print $2}')
    echo "  Total events: $TOTAL"
    echo "  Processed: $PROCESSED"
else
    echo -e "${RED}✗ Ingester pipeline failed${NC}"
fi

echo
echo -e "${YELLOW}Checking application metrics endpoint...${NC}"

# Try to connect to the metrics endpoint (won't work for one-shot commands)
if curl -s http://localhost:9898/metrics 2>/dev/null | head -1 | grep -q "#"; then
    echo -e "${GREEN}✓ Application metrics endpoint is available${NC}"
    METRIC_COUNT=$(curl -s http://localhost:9898/metrics | grep -c "^sms_" || echo "0")
    echo "  Found $METRIC_COUNT SMS metrics"
else
    echo -e "${YELLOW}⚠ Application metrics endpoint not available${NC}"
    echo "  (This is expected for one-shot commands)"
fi

echo
echo -e "${BLUE}=== Summary ===${NC}"
echo

# Final summary
TESTS_PASSED=0
TESTS_TOTAL=5

# Test 1: Services running
if curl -s http://localhost:9091/metrics > /dev/null 2>&1 && \
   curl -s http://localhost:9090/api/v1/query?query=up > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Infrastructure services running${NC}"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ Infrastructure services not running${NC}"
fi

# Test 2: Gateway ingestion works
if [ -f /tmp/gateway_output.log ] && grep -q "✅ Accepted envelope" /tmp/gateway_output.log; then
    echo -e "${GREEN}✓ Gateway ingestion works${NC}"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ Gateway ingestion failed${NC}"
fi

# Test 3: Metrics pushed to Pushgateway
if curl -s http://localhost:9091/metrics | grep -q "sms_ingest_timestamp_ms"; then
    echo -e "${GREEN}✓ Metrics pushed to Pushgateway${NC}"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ Metrics not in Pushgateway${NC}"
fi

# Test 4: Prometheus can scrape
PROM_VALUE=$(curl -s "http://localhost:9090/api/v1/query?query=sms_ingest_success" | jq -r '.data.result[0].value[1]' 2>/dev/null || echo "null")
if [ "$PROM_VALUE" != "null" ] && [ "$PROM_VALUE" != "" ]; then
    echo -e "${GREEN}✓ Prometheus successfully scraping metrics${NC}"
    ((TESTS_PASSED++))
else
    echo -e "${YELLOW}⚠ Prometheus not yet scraping our metrics${NC}"
fi

# Test 5: Regular ingester runs
if [ -f /tmp/ingester_output.log ] && grep -q "Pipeline Results" /tmp/ingester_output.log; then
    echo -e "${GREEN}✓ Standard ingester pipeline works${NC}"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ Standard ingester pipeline failed${NC}"
fi

echo
echo -e "Tests passed: ${TESTS_PASSED}/${TESTS_TOTAL}"

if [ $TESTS_PASSED -eq $TESTS_TOTAL ]; then
    echo -e "${GREEN}All tests passed! ✨${NC}"
    exit 0
elif [ $TESTS_PASSED -ge 3 ]; then
    echo -e "${YELLOW}Most tests passed, but some issues remain${NC}"
    exit 1
else
    echo -e "${RED}Multiple tests failed${NC}"
    exit 2
fi
