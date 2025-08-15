# Quality Gate Metrics Integration

This document describes the integration of Quality Gate metrics into the SMS Pipeline dashboard system.

## Overview

The Quality Gate is a data quality validation step in the SMS scraper pipeline that assesses normalized records for quality issues before they proceed to final storage. This integration adds comprehensive metrics and dashboard visualization for monitoring the quality gate's performance.

## Quality Gate Metrics Added

The following metrics have been added to track quality gate operations:

### Counter Metrics
- `sms_quality_gate_records_accepted_total` - Total records that passed quality checks
- `sms_quality_gate_records_accepted_with_warnings_total` - Records accepted despite warnings
- `sms_quality_gate_records_quarantined_total` - Records quarantined due to quality issues
- `sms_quality_gate_issues_detected_total` - Total quality issues detected (with labels for issue type and severity)
- `sms_quality_gate_batches_processed_total` - Total batches processed through quality gate

### Histogram Metrics
- `sms_quality_gate_quality_score` - Distribution of quality scores assigned to records
- `sms_quality_gate_batch_size` - Size distribution of quality gate processing batches

## Dashboard Integration

### Static Dashboard
Quality Gate metrics are included in the static dashboard catalog (`DashboardBuilder::from_catalog()`) under the "quality_gate" phase. This ensures they appear in manually maintained dashboard definitions.

### Dynamic Dashboard  
Quality Gate metrics are automatically discovered through the `MetricName::all_metrics()` iterator, making them part of the dynamic dashboard generation that auto-discovers all metrics from the enum.

### Dashboard Panels
The quality gate section includes:
- **Acceptance Rate**: Total records accepted (stat panel)
- **Warnings**: Records accepted with warnings (stat panel) 
- **Quarantine Rate**: Records quarantined (stat panel)
- **Quality Score Distribution**: Histogram showing score percentiles (p50, p95, p99)
- **Issues Detected**: Total quality issues found (stat panel)
- **Batch Processing**: Batches processed count (stat panel)
- **Batch Size**: Histogram of batch sizes (p50, p95, p99)

## Implementation Details

### Metrics Module Structure
```rust
// In src/observability/metrics.rs
pub mod quality_gate {
    pub fn record_accepted() { ... }
    pub fn record_accepted_with_warnings() { ... }
    pub fn record_quarantined() { ... }
    pub fn quality_score_recorded(score: f64) { ... }
    pub fn issue_detected(issue_type: &str, severity: &str) { ... }
    pub fn batch_processed(total_records: usize, accepted_count: usize, quarantined_count: usize) { ... }
}
```

### MetricName Enum
All quality gate metrics are defined as variants in the `MetricName` enum:
- `QualityGateRecordsAccepted`
- `QualityGateRecordsAcceptedWithWarnings`
- `QualityGateRecordsQuarantined`
- `QualityGateQualityScore`
- `QualityGateIssuesDetected`
- `QualityGateBatchesProcessed`
- `QualityGateBatchSize`

### Metric Type Inference
The metric type inference system automatically categorizes quality gate metrics:
- Counter: metrics containing "accepted", "quarantined", "detected", "processed"
- Histogram: metrics containing "score", "size"

### Pushgateway Support
All quality gate counter metrics include automatic pushgateway support for integration with Prometheus in push mode, suitable for short-lived jobs.

## Usage

### Dashboard Generation
```bash
# Generate static dashboard with quality gate metrics
make dashboard

# Generate dynamic dashboard (auto-discovers quality gate metrics)
make dashboard-dynamic

# Update provisioned dashboard in Grafana
make dashboard-provision
```

### Viewing Metrics
1. Access Grafana at http://localhost:3000
2. Navigate to the "SMS Pipeline - Unified Dashboard" 
3. Scroll to the "QUALITY_GATE Metrics" section
4. View real-time quality gate performance metrics

### Integration with Quality Gate Code
When implementing the quality gate processing logic, call the appropriate metrics functions:

```rust
use crate::observability::metrics::quality_gate;

// When a record passes quality checks
quality_gate::record_accepted();

// When a record has warnings but is accepted
quality_gate::record_accepted_with_warnings();

// When a record is quarantined
quality_gate::record_quarantined();

// Record quality score distribution
quality_gate::quality_score_recorded(0.85);

// Track detected issues
quality_gate::issue_detected("missing_venue_address", "warning");

// Track batch processing
quality_gate::batch_processed(100, 95, 5); // total, accepted, quarantined
```

## Dashboard Features

### Panel Types
- **Stat Panels**: Show cumulative totals for key metrics (accepted, quarantined, issues)
- **Graph Panels**: Show histogram distributions for quality scores and batch sizes using Prometheus histogram quantiles

### Time Range
- Default: Last 1 hour with 10-second refresh
- Configurable refresh intervals: 5s, 10s, 30s, 1m, 5m, 15m, 30m, 1h, 2h, 1d

### Organization
Quality gate metrics are grouped in their own dashboard section with a clear row separator, making it easy to focus on data quality monitoring.

## Benefits

1. **Quality Monitoring**: Real-time visibility into data quality patterns
2. **Issue Detection**: Quick identification of quality problems in the pipeline
3. **Performance Tracking**: Monitor acceptance vs. quarantine rates over time
4. **Operational Insights**: Understand quality score distributions and batch processing patterns
5. **Alerting Ready**: Metrics are available for Prometheus alerting rules
6. **Historical Analysis**: Track quality trends over time through Grafana's time series capabilities

## Files Modified

- `src/observability/metrics.rs` - Added quality gate metrics module and enum variants
- `src/observability/metrics/dashboard.rs` - Added quality gate metrics to static catalog
- `ops/grafana/provisioning/dashboards/sms-unified-pipeline.json` - Updated with quality gate panels
- `docs/quality-gate-dashboard-integration.md` - This documentation

The quality gate metrics are now fully integrated into the SMS scraper monitoring and dashboard system, providing comprehensive visibility into data quality operations.
