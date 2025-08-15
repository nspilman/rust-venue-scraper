#!/bin/bash

# Build Dashboard from Metrics
# This script generates a Grafana dashboard from our metrics definitions

set -euo pipefail

echo "📊 Building unified pipeline dashboard from metrics definitions..."

# Build and run the dashboard builder
cargo run --bin build-dashboard > ops/grafana/provisioning/dashboards/sms-unified-generated.json

echo "✅ Dashboard generated successfully!"
echo "📁 Output: ops/grafana/provisioning/dashboards/sms-unified-generated.json"
echo ""
echo "🔧 To use this dashboard:"
echo "1. Restart your Grafana container to pick up the new dashboard"
echo "2. Navigate to Dashboards > SMS Pipeline - Generated Dashboard"
echo ""
echo "💡 This dashboard is generated from your metrics source of truth,"
echo "   so it will always stay in sync with your actual metrics!"
