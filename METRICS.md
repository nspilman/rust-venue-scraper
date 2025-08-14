# SMS Scraper Metrics Integration

This document describes the comprehensive metrics system integrated into the SMS Scraper pipeline.

## Overview

The scraper now includes phase-based metrics collection across all pipeline stages:

- **Sources & Registry Phase**: Request success/failure, cadence checks, registry loads
- **Gateway Phase**: Envelope processing, deduplication, CAS operations
- **Parser Phase**: Parsing operations, batch processing, record production  
- **Ingest Log Phase**: Log writes, consumer operations, file rotations

All metrics follow a consistent naming convention: `sms_{phase}_{metric_type}_{name}`

## Quick Start

### Option 1: Local Demo (Recommended)

Run the pipeline demonstration using a local build:

```bash
./demo-local.sh
```

This script will:
1. Build the scraper locally (if needed)
2. Start the metrics server in the background
3. Execute the complete scraping pipeline
4. Show metrics being generated at each phase
5. Demonstrate deduplication tracking
6. Provide comprehensive metrics analysis

### Option 2: Complete Demo with Docker Compose

Run the full pipeline demonstration with Docker:

```bash
./demo-pipeline.sh
```

This script will:
1. Build and start all services (Scraper, Prometheus, PushGateway)
2. Execute the complete scraping pipeline
3. Show metrics being generated at each phase
4. Demonstrate deduplication tracking
5. Verify Prometheus integration

### Option 2: Manual Steps

1. **Start the services**:
   ```bash
   docker-compose up -d
   ```

2. **Run a scrape operation**:
   ```bash
   docker exec sms_scraper /usr/local/bin/sms_scraper gateway-once --source-id blue_moon --bypass-cadence
   ```

3. **Parse the scraped data**:
   ```bash
   docker exec sms_scraper /usr/local/bin/sms_scraper parse --max 10
   ```

4. **View metrics**:
   ```bash
   curl http://localhost:9898/metrics
   ```

## Service Endpoints

- **SMS Scraper API**: http://localhost:8080/graphql
- **GraphiQL UI**: http://localhost:8080/graphiql
- **Health Check**: http://localhost:8080/health
- **Metrics Endpoint**: http://localhost:9898/metrics
- **Prometheus**: http://localhost:9090
- **PushGateway**: http://localhost:9091

## Available Metrics

### Sources Phase Metrics
- `sms_sources_requests_success_total`: Successful HTTP requests to data sources
- `sms_sources_requests_error_total`: Failed HTTP requests to data sources  
- `sms_sources_request_duration_seconds`: HTTP request duration histogram
- `sms_sources_payload_bytes`: Size of payloads received from sources
- `sms_sources_registry_loads_success_total`: Successful registry loads
- `sms_sources_registry_loads_error_total`: Failed registry loads
- `sms_sources_cadence_checks_total`: Cadence check operations

### Gateway Phase Metrics
- `sms_gateway_envelopes_accepted_total`: Envelopes accepted by the gateway
- `sms_gateway_envelopes_deduplicated_total`: Envelopes that were deduplicated
- `sms_gateway_cas_writes_success_total`: Successful CAS write operations
- `sms_gateway_cas_writes_error_total`: Failed CAS write operations
- `sms_gateway_processing_duration_seconds`: Envelope processing time
- `sms_gateway_cas_operation_duration_seconds`: CAS operation duration
- `sms_gateway_payload_bytes`: Payload sizes processed
- `sms_gateway_cas_write_bytes`: Bytes written to CAS
- `sms_gateway_policy_checks_total`: Policy enforcement checks

### Parser Phase Metrics
- `sms_parser_envelopes_processed_total`: Envelopes processed by parser
- `sms_parser_records_produced_total`: Records produced from parsing
- `sms_parser_errors_total`: Parsing errors encountered
- `sms_parser_duration_seconds`: Parsing operation duration
- `sms_parser_records_per_envelope`: Records produced per envelope
- `sms_parser_batches_processed_total`: Parser batch runs
- `sms_parser_batch_size_envelopes`: Envelopes per batch
- `sms_parser_batch_records_written`: Records written per batch

### Ingest Log Phase Metrics  
- `sms_ingest_log_writes_success_total`: Successful log writes
- `sms_ingest_log_writes_error_total`: Failed log writes
- `sms_ingest_log_write_bytes`: Bytes written to log
- `sms_ingest_log_consumer_reads_total`: Consumer read operations
- `sms_ingest_log_consumer_read_batch_size`: Consumer batch sizes
- `sms_ingest_log_current_file_bytes`: Current log file size
- `sms_ingest_log_consumer_lag_bytes`: Consumer lag in bytes

## Example Queries

### Prometheus Queries (PromQL)

**Request Success Rate**:
```promql
rate(sms_sources_requests_success_total[5m])
```

**Average Processing Duration**:
```promql
rate(sms_gateway_processing_duration_seconds_sum[5m]) / rate(sms_gateway_processing_duration_seconds_count[5m])
```

**Deduplication Rate**:
```promql
sms_gateway_envelopes_deduplicated_total / (sms_gateway_envelopes_accepted_total + sms_gateway_envelopes_deduplicated_total)
```

**Parser Throughput (records/sec)**:
```promql
rate(sms_parser_records_produced_total[5m])
```

### Direct HTTP Queries

**View all SMS metrics**:
```bash
curl http://localhost:9898/metrics | grep "^sms_"
```

**Check request metrics**:
```bash
curl http://localhost:9898/metrics | grep "sms_sources_requests"
```

**View processing durations**:
```bash
curl http://localhost:9898/metrics | grep "duration_seconds"
```

## Grafana Dashboards (Optional)

To start Grafana with pre-configured dashboards:

```bash
docker-compose --profile observability up -d grafana
```

Access Grafana at http://localhost:3000 (admin/admin)

## Metrics Architecture

The metrics system is built with:

- **Phase-based organization**: Each pipeline stage has its own metrics namespace
- **Prometheus compatibility**: All metrics export in Prometheus format
- **Consistent naming**: `sms_{phase}_{metric_type}_{name}` convention
- **Comprehensive coverage**: Success/error rates, durations, throughput, sizes
- **Real-time export**: Metrics available immediately at /metrics endpoint

## Development

To add new metrics:

1. Add the metric to the appropriate phase module in `src/metrics/`
2. Call the metric recording method at the relevant code location
3. Add documentation to the `metrics_documentation()` method
4. Update the metric registration in `register_metrics()`

## Troubleshooting

**Metrics not appearing**:
- Check that the metrics server is running on port 9898
- Verify the metric is being recorded by checking the code path
- Ensure the metric is registered in the phase's `register_metrics()` method

**Prometheus not scraping**:
- Check Prometheus targets at http://localhost:9090/targets
- Verify the scraper container is accessible from Prometheus
- Check the prometheus.yml configuration

**High memory usage**:
- Metrics with high cardinality labels can consume memory
- Consider reducing label dimensions or implementing metric sampling
