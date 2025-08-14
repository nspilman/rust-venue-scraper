# SMS Scraper Metrics Reference

This document describes the comprehensive phase-based metrics system for monitoring the SMS scraper pipeline.

## Overview

The metrics system is organized by pipeline phases as described in the Platonic Ideal document:

1. **Sources & Registry** - Data source interactions and registry operations
2. **Gateway** - Envelope processing and CAS operations  
3. **Ingest Log** - Log operations and consumer management
4. **Parser** - Envelope parsing and record production

All metrics follow the naming convention: `sms_{phase}_{metric_name}_{type}`

## Metrics Endpoint

- **URL**: `http://localhost:9898/metrics` (configurable via `SMS_METRICS_PORT`)
- **Format**: Prometheus exposition format
- **Refresh**: Real-time updates as operations occur

## Phase-Specific Metrics

### 📊 Sources & Registry Phase

**Request Metrics:**
- `sms_sources_requests_success_total` - Total successful source requests
- `sms_sources_requests_error_total` - Total failed source requests  
- `sms_sources_request_duration_seconds` - Request duration histogram
- `sms_sources_payload_bytes` - Payload size histogram

**Registry Operations:**
- `sms_sources_registry_loads_success_total` - Successful registry loads
- `sms_sources_registry_loads_error_total` - Failed registry loads

**Cadence Management:**
- `sms_sources_cadence_checks_total` - Total cadence checks performed

### 🚪 Gateway Phase

**Envelope Processing:**
- `sms_gateway_envelopes_accepted_total` - Envelopes accepted by gateway
- `sms_gateway_envelopes_deduplicated_total` - Deduplicated envelopes
- `sms_gateway_processing_duration_seconds` - Processing duration histogram
- `sms_gateway_payload_bytes` - Payload size histogram

**CAS Operations:**
- `sms_gateway_cas_writes_success_total` - Successful CAS writes
- `sms_gateway_cas_writes_error_total` - Failed CAS writes
- `sms_gateway_cas_write_bytes` - CAS write size histogram
- `sms_gateway_cas_operation_duration_seconds` - CAS operation duration

**Policy Enforcement:**
- `sms_gateway_policy_checks_total` - Policy checks performed

### 📝 Ingest Log Phase

**Log Operations:**
- `sms_ingest_log_writes_success_total` - Successful log writes
- `sms_ingest_log_writes_error_total` - Failed log writes
- `sms_ingest_log_write_bytes` - Log write size histogram

**Consumer Management:**
- `sms_ingest_log_consumer_reads_total` - Consumer read operations
- `sms_ingest_log_consumer_acks_total` - Consumer acknowledgments
- `sms_ingest_log_consumer_read_batch_size` - Read batch size histogram
- `sms_ingest_log_consumer_lag_bytes` - Consumer lag in bytes (gauge)

**Log Management:**
- `sms_ingest_log_rotations_total` - Log file rotations
- `sms_ingest_log_current_file_bytes` - Current log file size (gauge)
- `sms_ingest_log_active_consumers` - Number of active consumers (gauge)
- `sms_ingest_log_symlink_updates_total` - Symlink updates

### ⚙️ Parser Phase

**Processing Metrics:**
- `sms_parser_envelopes_processed_total` - Envelopes processed
- `sms_parser_records_produced_total` - Records produced
- `sms_parser_duration_seconds` - Parse duration histogram
- `sms_parser_records_per_envelope` - Records per envelope histogram

**Error Tracking:**
- `sms_parser_errors_total` - Parse errors
- `sms_parser_empty_envelopes_total` - Envelopes producing zero records

**Batch Operations:**
- `sms_parser_batches_processed_total` - Processing batches completed
- `sms_parser_batch_size_envelopes` - Batch size histogram
- `sms_parser_batch_records_written` - Records written per batch

**Payload Resolution:**
- `sms_parser_payload_resolutions_success_total` - Successful payload resolutions
- `sms_parser_payload_resolutions_error_total` - Failed payload resolutions
- `sms_parser_payload_bytes_resolved` - Resolved payload size histogram
- `sms_parser_payload_resolution_duration_seconds` - Resolution duration

## Grafana Dashboards

### SMS Scraper Overview (`sms-overview`)
- High-level pipeline health and data freshness
- Compatible with existing monitoring setup
- Shows legacy metrics for backwards compatibility

### SMS Scraper - Detailed Phase Metrics (`sms-detailed`)
- **NEW**: Comprehensive phase-by-phase breakdown
- Real-time rates and percentiles
- Error tracking and performance analysis
- Organized by pipeline phases with clear visual separation

### Dashboard Access
- **Local**: `http://localhost:3000` (when running with docker-compose)
- **Credentials**: admin/admin (default Grafana setup)

## Key Queries for Alerting

### High Error Rate
```promql
# Sources phase error rate > 5%
rate(sms_sources_requests_error_total[5m]) / rate(sms_sources_requests_success_total[5m]) > 0.05

# Gateway CAS write failures
rate(sms_gateway_cas_writes_error_total[5m]) > 0

# Parser error rate
rate(sms_parser_errors_total[5m]) > 0.1
```

### Performance Issues
```promql
# Slow source requests (>30s at 95th percentile)
histogram_quantile(0.95, rate(sms_sources_request_duration_seconds_bucket[5m])) > 30

# High parser processing time (>5s at 95th percentile) 
histogram_quantile(0.95, rate(sms_parser_duration_seconds_bucket[5m])) > 5

# Large consumer lag (>100MB)
sms_ingest_log_consumer_lag_bytes > 100000000
```

### Data Freshness
```promql
# No pipeline runs in 6 hours
time() - max(sms_pipeline_last_run_timestamp_seconds) > 21600
```

## Migration from Legacy Metrics

The new metrics system runs alongside existing metrics for backwards compatibility:

**Legacy → New Mapping:**
- `sms_ingest_runs_total` → Use `sms_sources_requests_success_total`
- `sms_events_processed_total` → Use `sms_parser_records_produced_total` 
- `sms_pipeline_duration_seconds` → Still available (cross-cutting metric)

## Integration with Existing Stack

**Prometheus Configuration:**
- Already configured to scrape `host.docker.internal:9898`
- No changes needed to existing `prometheus.yml`

**Grafana Setup:**
- Dashboards auto-provision from `ops/grafana/provisioning/dashboards/`
- New detailed dashboard available immediately after restart

**Docker Compose:**
- No changes needed to existing `docker-compose.yml`
- Metrics port 9898 already exposed

## Next Steps

1. **Deploy**: Restart your application to begin emitting new metrics
2. **Monitor**: Check the new "SMS Scraper - Detailed Phase Metrics" dashboard
3. **Alert**: Set up alerts using the provided query examples
4. **Iterate**: Add more specific metrics as monitoring needs evolve

The phase-based organization makes it easy to:
- **Pinpoint issues** to specific pipeline stages
- **Scale monitoring** as new phases are added
- **Maintain consistency** across all metric naming
- **Troubleshoot performance** with detailed timing data
