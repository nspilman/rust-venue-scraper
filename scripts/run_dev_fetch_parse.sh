#!/usr/bin/env bash
set -euo pipefail

# Dev helper: Fetch every time (bypass cadence) and then parse envelopes for a single venue
# Usage:
#   scripts/run_dev_fetch_parse.sh <venue_id> [data_root] [consumer] [output_file]
# Examples:
#   scripts/run_dev_fetch_parse.sh blue_moon
#   scripts/run_dev_fetch_parse.sh sea_monster data parser_sea_monster parsed_sea_monster.ndjson

VENUE_ID=${1:-blue_moon}
DATA_ROOT=${2:-data}
CONSUMER=${3:-parser_${VENUE_ID}}
OUTPUT_FILE=${4:-parsed_${VENUE_ID}.ndjson}

# Ensure bypass cadence for development testing
export SMS_BYPASS_CADENCE=1

echo "[1/2] Fetching via GatewayOnce (bypass cadence) for venue=${VENUE_ID} ..."
cargo run --bin sms_scraper -- gateway-once \
  --source-id "${VENUE_ID}" \
  --data-root "${DATA_ROOT}" \
  --bypass-cadence

echo "[2/2] Parsing envelopes to ${OUTPUT_FILE} (consumer=${CONSUMER}) ..."
cargo run --bin sms_scraper -- parse \
  --consumer "${CONSUMER}" \
  --max 100 \
  --data-root "${DATA_ROOT}" \
  --output "${OUTPUT_FILE}" \
  --source-id "${VENUE_ID}"

echo "Done. Output written to ${OUTPUT_FILE}"
