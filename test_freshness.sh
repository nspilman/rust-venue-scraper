#!/bin/bash

echo "=== Testing freshness metrics ==="

# Force fetch new data
echo "Fetching fresh data (bypassing cadence)..."
docker-compose exec -T scraper /usr/local/bin/sms_scraper gateway-once --source-id blue_moon --bypass-cadence

echo "=== Starting metric monitoring ==="
echo "Will run pipeline and monitor for timestamp metrics..."

# Start monitoring in background
{
    for i in {1..25}; do
        echo "--- Check $i ($(date)) ---"
        echo "Pushgateway timestamp metrics:"
        curl -s http://localhost:9091/metrics | grep "sms_pipeline_last_run_timestamp_seconds" || echo "No timestamp metrics found in Pushgateway"
        
        echo "Prometheus timestamp metrics:"
        curl -s "http://localhost:9090/api/v1/query?query=sms_pipeline_last_run_timestamp_seconds" | jq -r '.data.result[] | "\(.metric.instance): \(.value[1])"' || echo "No timestamp metrics found in Prometheus"
        
        echo ""
        sleep 5
    done
} &

MONITOR_PID=$!

# Wait a moment then run the pipeline
sleep 2
echo "=== Running pipeline ==="
docker-compose exec -T scraper /usr/local/bin/sms_scraper run

# Keep monitoring for a bit after pipeline completes
sleep 80
kill $MONITOR_PID 2>/dev/null

echo "=== Final check ==="
echo "Final Prometheus timestamp metrics:"
curl -s "http://localhost:9090/api/v1/query?query=sms_pipeline_last_run_timestamp_seconds" | jq '.data.result'
