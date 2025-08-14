//! Ingest Log Phase Metrics
//!
//! Metrics for tracking the ingest log's health and performance, including
//! writes, reads, consumer lag, and rotation operations.

use crate::metrics::{phase_metric, MetricDoc, MetricType, PhaseMetrics};

/// Metrics collection for the Ingest Log phase
pub struct IngestLogMetrics;


impl IngestLogMetrics {
    /// Record a successful write to the ingest log
    pub fn record_write_success(envelope_bytes: usize) {
        ::metrics::counter!(phase_metric!(counter, "ingest_log", "writes_success")).increment(1);
        ::metrics::histogram!(phase_metric!(histogram, "ingest_log", "write_bytes"))
            .record(envelope_bytes as f64);
    }

    /// Record a failed write to the ingest log
    pub fn record_write_error(_error_type: &str) {
        ::metrics::counter!(phase_metric!(counter, "ingest_log", "writes_error")).increment(1);
    }

    /// Record consumer read operations
    pub fn record_consumer_read(_consumer_id: &str, envelopes_read: usize) {
        ::metrics::counter!(phase_metric!(counter, "ingest_log", "consumer_reads")).increment(1);
        ::metrics::histogram!(phase_metric!(
            histogram,
            "ingest_log",
            "consumer_read_batch_size"
        ))
        .record(envelopes_read as f64);
    }

    /// Record log file size metrics
    pub fn record_current_log_size(size_bytes: u64) {
        ::metrics::gauge!(phase_metric!(gauge, "ingest_log", "current_file_bytes"))
            .set(size_bytes as f64);
    }

}

impl PhaseMetrics for IngestLogMetrics {
    fn register_metrics() {
        use metrics::{counter, gauge, histogram};

        // Pre-register all metrics (bind to placeholders to satisfy must_use)
        let _ = counter!(phase_metric!(counter, "ingest_log", "writes_success"));
        let _ = counter!(phase_metric!(counter, "ingest_log", "writes_error"));
        let _ = counter!(phase_metric!(counter, "ingest_log", "consumer_reads"));
        let _ = counter!(phase_metric!(counter, "ingest_log", "consumer_acks"));
        let _ = counter!(phase_metric!(counter, "ingest_log", "rotations"));
        let _ = counter!(phase_metric!(counter, "ingest_log", "symlink_updates"));

        let _ = histogram!(phase_metric!(histogram, "ingest_log", "write_bytes"));
        let _ = histogram!(phase_metric!(
            histogram,
            "ingest_log",
            "consumer_read_batch_size"
        ));

        let _ = gauge!(phase_metric!(gauge, "ingest_log", "consumer_lag_bytes"));
        let _ = gauge!(phase_metric!(gauge, "ingest_log", "current_file_bytes"));
        let _ = gauge!(phase_metric!(gauge, "ingest_log", "active_consumers"));
    }

    fn phase_name() -> &'static str {
        "ingest_log"
    }

    fn metrics_documentation() -> Vec<MetricDoc> {
        vec![
            MetricDoc {
                name: phase_metric!(counter, "ingest_log", "writes_success"),
                metric_type: MetricType::Counter,
                help: "Total number of successful writes to the ingest log",
                labels: vec![],
            },
            MetricDoc {
                name: phase_metric!(counter, "ingest_log", "writes_error"),
                metric_type: MetricType::Counter,
                help: "Total number of failed writes to the ingest log",
                labels: vec!["error_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "ingest_log", "consumer_reads"),
                metric_type: MetricType::Counter,
                help: "Total number of read operations by consumers",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(counter, "ingest_log", "consumer_acks"),
                metric_type: MetricType::Counter,
                help: "Total number of acknowledgments by consumers",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(counter, "ingest_log", "rotations"),
                metric_type: MetricType::Counter,
                help: "Total number of log file rotations",
                labels: vec![],
            },
            MetricDoc {
                name: phase_metric!(counter, "ingest_log", "symlink_updates"),
                metric_type: MetricType::Counter,
                help: "Total number of symlink updates",
                labels: vec!["result"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "ingest_log", "write_bytes"),
                metric_type: MetricType::Histogram,
                help: "Size of data written to the ingest log in bytes",
                labels: vec![],
            },
            MetricDoc {
                name: phase_metric!(histogram, "ingest_log", "consumer_read_batch_size"),
                metric_type: MetricType::Histogram,
                help: "Number of envelopes read in each consumer batch",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(gauge, "ingest_log", "consumer_lag_bytes"),
                metric_type: MetricType::Gauge,
                help: "Current lag in bytes for each consumer",
                labels: vec!["consumer_id"],
            },
            MetricDoc {
                name: phase_metric!(gauge, "ingest_log", "current_file_bytes"),
                metric_type: MetricType::Gauge,
                help: "Current size of the active log file in bytes",
                labels: vec![],
            },
            MetricDoc {
                name: phase_metric!(gauge, "ingest_log", "active_consumers"),
                metric_type: MetricType::Gauge,
                help: "Number of active consumers",
                labels: vec![],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_log_metrics_registration() {
        IngestLogMetrics::register_metrics();
    }

    #[test]
    fn test_metrics_documentation() {
        let docs = IngestLogMetrics::metrics_documentation();
        assert_eq!(docs.len(), 11);

        for doc in docs {
            assert!(doc.name.starts_with("sms_ingest_log_"));
        }
    }
}
