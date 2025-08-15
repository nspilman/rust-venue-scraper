#!/bin/bash

# Grafana API settings
GRAFANA_URL="http://localhost:3000"
GRAFANA_USER="admin"
GRAFANA_PASS="admin"

echo "📊 Importing SMS Scraper dashboard into Grafana..."

# Read the entire import JSON (already has dashboard wrapped)
IMPORT_JSON=$(cat grafana-dashboard.json)

# Import the dashboard
RESPONSE=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -u "$GRAFANA_USER:$GRAFANA_PASS" \
  -d "$IMPORT_JSON" \
  "$GRAFANA_URL/api/dashboards/db")

# Check if successful
if echo "$RESPONSE" | grep -q "\"uid\""; then
  DASH_UID=$(echo "$RESPONSE" | jq -r '.uid')
  DASH_SLUG=$(echo "$RESPONSE" | jq -r '.slug')
  echo "✅ Dashboard imported successfully!"
  echo "   UID: $DASH_UID"
  echo "   URL: $GRAFANA_URL/d/$DASH_UID/$DASH_SLUG"
  echo ""
  echo "📊 Open your dashboard at:"
  echo "   $GRAFANA_URL/d/$DASH_UID/$DASH_SLUG"
else
  echo "❌ Failed to import dashboard"
  echo "Response: $RESPONSE"
  echo ""
  echo "Common issues:"
  echo "1. Check Grafana credentials (default: admin/admin)"
  echo "2. Make sure Grafana is running on port 3000"
  echo "3. Try importing manually through the Grafana UI"
fi
