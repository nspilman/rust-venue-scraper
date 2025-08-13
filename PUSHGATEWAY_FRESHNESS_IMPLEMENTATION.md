# Pushgateway Data Freshness Implementation

## Overview

We've successfully implemented a combined approach to solve the "stale data masquerading as fresh data" problem in your Pushgateway metrics. This ensures your Grafana dashboards clearly distinguish between fresh pipeline data and outdated information.

## What Was Implemented

### 1. Timestamp Tracking
- Added `sms_pipeline_last_run_timestamp_seconds` metric to all Pushgateway pushes
- Contains Unix timestamp of when the pipeline completed
- Enables "data age" calculations in Grafana dashboards

### 2. Delete-on-Success Behavior  
- After successfully pushing metrics to Pushgateway, immediately delete them
- Prevents stale data from lingering between pipeline runs
- Creates intentional gaps in data that reflect reality

### 3. Updated Pipeline Flow
The new pipeline flow works like this:
1. Pipeline runs and completes successfully
2. Pushes all metrics (including timestamp) to Pushgateway
3. Immediately deletes the metrics from Pushgateway
4. Prometheus scrapes the fresh data during the brief window
5. Subsequent scrapes show no data until the next pipeline run

## Code Changes Made

### `src/pipeline.rs`
- Enhanced `push_pushgateway_metrics()` function with:
  - Timestamp generation using `chrono::Utc::now().timestamp()`
  - Additional metric: `sms_pipeline_last_run_timestamp_seconds`
  - HTTP DELETE request after successful push
  - Improved error handling and logging

### `docs/observability.md`
- Added comprehensive documentation of the new approach
- Included recommended Grafana panel configurations
- Updated Prometheus query examples
- Added troubleshooting guidance for the new behavior

## How to Use the New Features

### 1. For Immediate Testing
```bash
# Build and run a pipeline to see the new metrics
cargo build --bin sms_scraper

# Set environment variable (if not using Docker)
export SMS_PUSHGATEWAY_URL=http://localhost:9091

# Run a pipeline (example with blue_moon API)
./target/debug/sms_scraper ingester --apis blue_moon --bypass-cadence
```

### 2. Updated Grafana Queries

Instead of simple aggregations, use `last_over_time()` to capture values even during intentional gaps:

**Old queries:**
```promql
sum by (instance) (sms_events_processed_total)
```

**New queries:**
```promql
last_over_time(sms_events_processed_total[1h])
```

**New freshness panels:**
```promql
# Data age in seconds
time() - max by (instance) (last_over_time(sms_pipeline_last_run_timestamp_seconds[1h]))

# Last run timestamp
max by (instance) (last_over_time(sms_pipeline_last_run_timestamp_seconds[1h]))
```

### 3. Recommended Dashboard Layout

Create these panels in your Grafana dashboard:

1. **Last Run Age** (Stat panel)
   - Shows time since each API last ran
   - Use color thresholds: Green < 1h, Yellow 1-6h, Red > 6h

2. **Last Run Timestamp** (Stat panel)  
   - Shows actual completion time for each API
   - Format as human-readable timestamp

3. **Updated Data Panels**
   - All existing "last run" panels should use `last_over_time(metric[1h])`
   - Adjust time range (`[1h]`, `[6h]`, `[24h]`) based on your pipeline frequency

## Expected Behavior Changes

### What You'll See
- **During/immediately after pipeline runs**: All panels populate with fresh data
- **Between pipeline runs**: Data panels show "No data" instead of stale values
- **Freshness panels**: Always show current data age and last run time
- **Log output**: New messages like "Successfully deleted stale metrics from Pushgateway"

### What This Solves
- **Before**: Grafana showed pipeline results from 6 hours ago as if they were current
- **After**: Grafana clearly shows when data is old via explicit age tracking
- **Before**: No way to know if displayed values were fresh or stale  
- **After**: "Last Run Age" panels make data freshness immediately visible

## Testing the Implementation

### 1. Verify Metrics Are Being Pushed
Check your scraper logs for these messages:
```
[INFO] Pushed metrics to Pushgateway for api=blue_moon
[INFO] Successfully deleted stale metrics from Pushgateway for api=blue_moon
```

### 2. Check Pushgateway Directly
Visit `http://localhost:9091/metrics` and verify:
- Metrics appear briefly after pipeline runs
- Include the new timestamp metric: `sms_pipeline_last_run_timestamp_seconds`
- Metrics disappear shortly after (due to delete behavior)

### 3. Verify Prometheus Storage
Query Prometheus at `http://localhost:9090` for:
```promql
# Should show historical timestamp data
sms_pipeline_last_run_timestamp_seconds

# Should work even when current Pushgateway shows no data
last_over_time(sms_events_processed_total[1h])
```

## Migration Notes

### Existing Dashboards
Update your current dashboard panels to use the new query patterns. The old queries will start showing gaps (which is correct behavior).

### Monitoring Considerations
- Set up alerts based on data age thresholds (e.g., alert if API hasn't run in > 8 hours)
- Consider the intentional data gaps when setting up SLOs
- Use longer time ranges (`[24h]`) for infrequently running APIs

## Troubleshooting

### "No Data" in Panels
This is expected! Update your queries to use `last_over_time()` as documented.

### Large "Last Run Age" Values  
Indicates the API hasn't run recently. Check pipeline schedules and error logs.

### Missing Timestamp Metrics
Verify pipeline completed successfully (failed runs don't push metrics).

## Benefits Achieved

1. **Eliminates confusion** about data freshness
2. **Makes stale data obvious** through explicit age tracking  
3. **Reduces false confidence** in outdated metrics
4. **Provides clear debugging path** when pipelines aren't running as expected
5. **Follows Pushgateway best practices** for batch job monitoring

This implementation gives you the transparency you need to trust your observability data and quickly identify when your venue scraping pipelines aren't performing as expected.
