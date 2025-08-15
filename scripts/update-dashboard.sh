#!/bin/bash
set -e

echo "🤖 Automatically generating dynamic Grafana dashboard..."

# Generate the dynamic dashboard
cargo run --bin build-dashboard dynamic > /dev/null 2>&1

# Copy to Grafana provisioning directory
cp grafana-dashboard-dynamic.json ops/grafana/provisioning/dashboards/

echo "✅ Dynamic dashboard updated in Grafana provisioning!"
echo "📄 File: ops/grafana/provisioning/dashboards/grafana-dashboard-dynamic.json"
echo ""
echo "🔄 To apply changes:"
echo "   - If Grafana is running: It will auto-reload in ~30 seconds"
echo "   - If Grafana is stopped: Changes will be applied on next startup"
echo ""
echo "🌐 Access dashboard at: http://localhost:3000"
