//! Parser Phase Metrics
//!
//! Metrics for tracking the parser's performance, including envelope processing,
//! record production, parsing errors, and parser-specific operations.

use crate::metrics::{phase_metric, MetricDoc, MetricType, PhaseMetrics};

/// Metrics collection for the Parser phase
pub struct ParserMetrics;


impl ParserMetrics {
    /// Record a successful parsing operation
    pub fn record_parse_success(
        _source_id: &str,
        _parse_plan: &str,
        records_produced: usize,
        duration_secs: f64,
    ) {
        // For now, record without labels to avoid lifetime issues
        ::metrics::counter!(phase_metric!(counter, "parser", "envelopes_processed")).increment(1);
        ::metrics::counter!(phase_metric!(counter, "parser", "records_produced"))
            .increment(records_produced as u64);
        ::metrics::histogram!(phase_metric!(histogram, "parser", "duration_seconds"))
            .record(duration_secs);
        ::metrics::histogram!(phase_metric!(histogram, "parser", "records_per_envelope"))
            .record(records_produced as f64);
    }

    /// Record a parsing error
    pub fn record_parse_error(_source_id: &str, _parse_plan: &str, _error_type: &str) {
        ::metrics::counter!(phase_metric!(counter, "parser", "errors")).increment(1);
    }


    /// Simple record counter for batch runs
    pub fn record_batch_run(
        _consumer_id: &str,
        envelopes_processed: usize,
        records_written: usize,
    ) {
        ::metrics::counter!(phase_metric!(counter, "parser", "batches_processed")).increment(1);
        ::metrics::histogram!(phase_metric!(histogram, "parser", "batch_size_envelopes"))
            .record(envelopes_processed as f64);
        ::metrics::histogram!(phase_metric!(histogram, "parser", "batch_records_written"))
            .record(records_written as f64);
    }

    /// Record parse duration
    pub fn record_parse_duration(_source_id: &str, _parse_plan: &str, duration_secs: f64) {
        ::metrics::histogram!(phase_metric!(histogram, "parser", "duration_seconds"))
            .record(duration_secs);
    }

    /// Record successful batch run (for main.rs compatibility)
    pub fn record_batch_run_success(records: usize, duration_secs: f64) {
        ::metrics::counter!(phase_metric!(counter, "parser", "batches_processed")).increment(1);
        ::metrics::histogram!(phase_metric!(histogram, "parser", "batch_records_written"))
            .record(records as f64);
        if duration_secs > 0.0 {
            ::metrics::histogram!(phase_metric!(histogram, "parser", "duration_seconds"))
                .record(duration_secs);
        }
    }

    /// Record envelopes skipped
    pub fn record_envelopes_skipped(count: usize) {
        ::metrics::counter!(phase_metric!(counter, "parser", "envelopes_filtered"))
            .increment(count as u64);
    }

    /// Record payload bytes resolved
    pub fn record_payload_bytes_resolved(bytes: usize) {
        ::metrics::histogram!(phase_metric!(histogram, "parser", "payload_bytes_resolved"))
            .record(bytes as f64);
    }
}

impl PhaseMetrics for ParserMetrics {
    fn register_metrics() {
        use metrics::{counter, histogram};

        // Pre-register all metrics (bind to placeholders to satisfy must_use)
        let _ = counter!(phase_metric!(counter, "parser", "envelopes_processed"));
        let _ = counter!(phase_metric!(counter, "parser", "records_produced"));
        let _ = counter!(phase_metric!(counter, "parser", "errors"));
        let _ = counter!(phase_metric!(counter, "parser", "empty_envelopes"));
        let _ = counter!(phase_metric!(
            counter,
            "parser",
            "payload_resolutions_success"
        ));
        let _ = counter!(phase_metric!(
            counter,
            "parser",
            "payload_resolutions_error"
        ));
        let _ = counter!(phase_metric!(counter, "parser", "envelopes_filtered"));
        let _ = counter!(phase_metric!(counter, "parser", "dedupe_resolutions"));
        let _ = counter!(phase_metric!(counter, "parser", "batches_processed"));

        let _ = histogram!(phase_metric!(histogram, "parser", "duration_seconds"));
        let _ = histogram!(phase_metric!(histogram, "parser", "records_per_envelope"));
        let _ = histogram!(phase_metric!(histogram, "parser", "payload_bytes_resolved"));
        let _ = histogram!(phase_metric!(histogram, "parser", "batch_size_envelopes"));
        let _ = histogram!(phase_metric!(histogram, "parser", "batch_records_written"));
        let _ = histogram!(phase_metric!(
            histogram,
            "parser",
            "payload_resolution_duration_seconds"
        ));
    }

    fn phase_name() -> &'static str {
        "parser"
    }

    fn metrics_documentation() -> Vec<MetricDoc> {
        vec![
            MetricDoc {
                name: phase_metric!(counter, "parser", "envelopes_processed"),
                metric_type: MetricType::Counter,
                help: "Total number of envelopes processed by the parser",
                labels: vec!["source_id", "parse_plan"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "records_produced"),
                metric_type: MetricType::Counter,
                help: "Total number of records produced by the parser",
                labels: vec!["source_id", "parse_plan"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "errors"),
                metric_type: MetricType::Counter,
                help: "Total number of parsing errors",
                labels: vec!["source_id", "parse_plan", "error_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "empty_envelopes"),
                metric_type: MetricType::Counter,
                help: "Total number of envelopes that produced zero records",
                labels: vec!["source_id", "parse_plan"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "payload_resolutions_success"),
                metric_type: MetricType::Counter,
                help: "Total number of successful payload resolutions from CAS",
                labels: vec!["storage_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "payload_resolutions_error"),
                metric_type: MetricType::Counter,
                help: "Total number of failed payload resolutions from CAS",
                labels: vec!["storage_type", "error_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "envelopes_filtered"),
                metric_type: MetricType::Counter,
                help: "Total number of envelopes filtered out during processing",
                labels: vec!["source_id", "filter_reason"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "dedupe_resolutions"),
                metric_type: MetricType::Counter,
                help: "Total number of dedupe resolution attempts",
                labels: vec!["result"],
            },
            MetricDoc {
                name: phase_metric!(counter, "parser", "batches_processed"),
                metric_type: MetricType::Counter,
                help: "Total number of parsing batches processed",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "parser", "duration_seconds"),
                metric_type: MetricType::Histogram,
                help: "Duration of parsing operations in seconds",
                labels: vec!["source_id", "parse_plan"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "parser", "records_per_envelope"),
                metric_type: MetricType::Histogram,
                help: "Number of records produced per envelope",
                labels: vec!["source_id", "parse_plan"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "parser", "payload_bytes_resolved"),
                metric_type: MetricType::Histogram,
                help: "Size of payloads resolved from CAS in bytes",
                labels: vec!["storage_type"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "parser", "batch_size_envelopes"),
                metric_type: MetricType::Histogram,
                help: "Number of envelopes in each processing batch",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "parser", "batch_records_written"),
                metric_type: MetricType::Histogram,
                help: "Number of records written in each processing batch",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "parser", "payload_resolution_duration_seconds"),
                metric_type: MetricType::Histogram,
                help: "Duration of payload resolution operations in seconds",
                labels: vec!["storage_type"],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_metrics_registration() {
        ParserMetrics::register_metrics();
    }

    #[test]
    fn test_metrics_documentation() {
        let docs = ParserMetrics::metrics_documentation();
        assert_eq!(docs.len(), 15);

        for doc in docs {
            assert!(doc.name.starts_with("sms_parser_"));
        }
    }
}
