//! Sources  Registry Phase Metrics
//!
//! Metrics for tracking the health and performance of data source interactions,
//! including registry loading, source requests, and payload fetching.

use crate::metrics::{phase_metric, MetricDoc, MetricType, PhaseMetrics};

/// Metrics collection for the Sources & Registry phase
pub struct SourcesMetrics;


impl SourcesMetrics {
    /// Record a successful source request  
    pub fn record_request_success(_source_id: &str, duration_secs: f64, payload_bytes: usize) {
        // For now, record without labels to avoid lifetime issues
        ::metrics::counter!(phase_metric!(counter, "sources", "requests_success")).increment(1);
        ::metrics::histogram!(phase_metric!(
            histogram,
            "sources",
            "request_duration_seconds"
        ))
        .record(duration_secs);
        ::metrics::histogram!(phase_metric!(histogram, "sources", "payload_bytes"))
            .record(payload_bytes as f64);
    }

    /// Record a failed source request
    pub fn record_request_error(_source_id: &str, _error_type: &str) {
        ::metrics::counter!(phase_metric!(counter, "sources", "requests_error")).increment(1);
    }

    /// Record registry operation metrics
    pub fn record_registry_load_success(_source_id: &str) {
        ::metrics::counter!(phase_metric!(counter, "sources", "registry_loads_success"))
            .increment(1);
    }

    pub fn record_registry_load_error(_source_id: &str, _error_type: &str) {
        ::metrics::counter!(phase_metric!(counter, "sources", "registry_loads_error")).increment(1);
    }

    /// Record cadence check metrics
    pub fn record_cadence_check(_source_id: &str, _result: CadenceResult) {
        ::metrics::counter!(phase_metric!(counter, "sources", "cadence_checks")).increment(1);
    }

    /// Record a timing measurement for source requests
    pub fn record_request_duration(_source_id: &str, duration_secs: f64) {
        ::metrics::histogram!(phase_metric!(
            histogram,
            "sources",
            "request_duration_seconds"
        ))
        .record(duration_secs);
    }
}

/// Result of a cadence check
pub enum CadenceResult {
    Allowed,
    Skipped,
    Bypassed,
}

impl PhaseMetrics for SourcesMetrics {
    fn register_metrics() {
        use metrics::{counter, histogram};

        // Pre-register all metrics to ensure they appear in /metrics endpoint
        // even before first use (bind to placeholders to satisfy must_use)
        let _ = counter!(phase_metric!(counter, "sources", "requests_success"));
        let _ = counter!(phase_metric!(counter, "sources", "requests_error"));
        let _ = counter!(phase_metric!(counter, "sources", "registry_loads_success"));
        let _ = counter!(phase_metric!(counter, "sources", "registry_loads_error"));
        let _ = counter!(phase_metric!(counter, "sources", "cadence_checks"));

        let _ = histogram!(phase_metric!(
            histogram,
            "sources",
            "request_duration_seconds"
        ));
        let _ = histogram!(phase_metric!(histogram, "sources", "payload_bytes"));
    }

    fn phase_name() -> &'static str {
        "sources"
    }

    fn metrics_documentation() -> Vec<MetricDoc> {
        vec![
            MetricDoc {
                name: phase_metric!(counter, "sources", "requests_success"),
                metric_type: MetricType::Counter,
                help: "Total number of successful requests to data sources",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(counter, "sources", "requests_error"),
                metric_type: MetricType::Counter,
                help: "Total number of failed requests to data sources",
                labels: vec!["source_id", "error_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "sources", "registry_loads_success"),
                metric_type: MetricType::Counter,
                help: "Total number of successful registry loads",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(counter, "sources", "registry_loads_error"),
                metric_type: MetricType::Counter,
                help: "Total number of failed registry loads",
                labels: vec!["source_id", "error_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "sources", "cadence_checks"),
                metric_type: MetricType::Counter,
                help: "Total number of cadence checks performed",
                labels: vec!["source_id", "result"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "sources", "request_duration_seconds"),
                metric_type: MetricType::Histogram,
                help: "Duration of requests to data sources in seconds",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "sources", "payload_bytes"),
                metric_type: MetricType::Histogram,
                help: "Size of payloads received from data sources in bytes",
                labels: vec!["source_id"],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sources_metrics_registration() {
        SourcesMetrics::register_metrics();
        // If we get here without panicking, registration succeeded
    }

    #[test]
    fn test_metrics_documentation() {
        let docs = SourcesMetrics::metrics_documentation();
        assert!(!docs.is_empty());
        assert_eq!(docs.len(), 7); // We defined 7 metrics

        // Verify all metric names follow our naming convention
        for doc in docs {
            assert!(doc.name.starts_with("sms_sources_"));
        }
    }
}
