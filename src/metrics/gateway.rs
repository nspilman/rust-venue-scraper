//! Gateway Phase Metrics
//!
//! Metrics for tracking the ingestion gateway's performance, including envelope
//! processing, deduplication, CAS operations, and policy enforcement.

use crate::metrics::{phase_metric, MetricDoc, MetricType, PhaseMetrics};

/// Metrics collection for the Ingestion Gateway phase
pub struct GatewayMetrics;


impl GatewayMetrics {
    /// Record an accepted envelope
    pub fn record_envelope_accepted(
        _source_id: &str,
        payload_bytes: usize,
        processing_duration_secs: f64,
    ) {
        // For now, record without labels to avoid lifetime issues
        let counter_name = phase_metric!(counter, "gateway", "envelopes_accepted");
        tracing::info!("METRICS: About to increment counter: {}", counter_name);
        ::metrics::counter!(phase_metric!(counter, "gateway", "envelopes_accepted")).increment(1);
        tracing::info!("METRICS: Incremented counter: {}", counter_name);
        
        let hist_name = phase_metric!(histogram, "gateway", "payload_bytes");
        tracing::info!("METRICS: Recording histogram {} with value {}", hist_name, payload_bytes);
        ::metrics::histogram!(phase_metric!(histogram, "gateway", "payload_bytes"))
            .record(payload_bytes as f64);
        
        let duration_name = phase_metric!(histogram, "gateway", "processing_duration_seconds");
        tracing::info!("METRICS: Recording histogram {} with value {}", duration_name, processing_duration_secs);
        ::metrics::histogram!(phase_metric!(
            histogram,
            "gateway",
            "processing_duration_seconds"
        ))
        .record(processing_duration_secs);

        // Record as potential records ingested (1 envelope = 1+ potential records)
        let records_name = phase_metric!(counter, "gateway", "records_ingested");
        tracing::info!("METRICS: About to increment counter: {}", records_name);
        ::metrics::counter!(phase_metric!(counter, "gateway", "records_ingested")).increment(1);
        tracing::debug!(
            "Incremented sms_gateway_records_ingested_total (via record_envelope_accepted)"
        );
    }

    /// Record a deduplicated envelope
    pub fn record_envelope_deduplicated(_source_id: &str) {
        ::metrics::counter!(phase_metric!(counter, "gateway", "envelopes_deduplicated"))
            .increment(1);
    }

    /// Record a successful CAS write
    pub fn record_cas_write_success(_storage_type: &str, payload_bytes: usize) {
        ::metrics::counter!(phase_metric!(counter, "gateway", "cas_writes_success")).increment(1);
        ::metrics::histogram!(phase_metric!(histogram, "gateway", "cas_write_bytes"))
            .record(payload_bytes as f64);
    }

    /// Record a failed CAS write
    pub fn record_cas_write_error(_storage_type: &str, _error_type: &str) {
        ::metrics::counter!(phase_metric!(counter, "gateway", "cas_writes_error")).increment(1);
    }


    /// Record envelope processing duration
    pub fn record_processing_duration(_source_id: &str, duration_secs: f64) {
        ::metrics::histogram!(phase_metric!(
            histogram,
            "gateway",
            "processing_duration_seconds"
        ))
        .record(duration_secs);
    }

    /// Record CAS operation duration
    pub fn record_cas_operation_duration(_storage_type: &str, duration_secs: f64) {
        ::metrics::histogram!(phase_metric!(
            histogram,
            "gateway",
            "cas_operation_duration_seconds"
        ))
        .record(duration_secs);
    }
}


impl PhaseMetrics for GatewayMetrics {
    fn register_metrics() {
        use metrics::{describe_counter, describe_histogram};

        // Properly register metrics with descriptions
        describe_counter!(
            phase_metric!(counter, "gateway", "envelopes_accepted"),
            "Total number of envelopes accepted by the gateway"
        );
        describe_counter!(
            phase_metric!(counter, "gateway", "envelopes_deduplicated"),
            "Total number of envelopes that were deduplicated"
        );
        describe_counter!(
            phase_metric!(counter, "gateway", "cas_writes_success"),
            "Total number of successful CAS writes"
        );
        describe_counter!(
            phase_metric!(counter, "gateway", "cas_writes_error"),
            "Total number of failed CAS writes"
        );
        describe_counter!(
            phase_metric!(counter, "gateway", "policy_checks"),
            "Total number of policy checks performed"
        );
        describe_counter!(
            phase_metric!(counter, "gateway", "records_ingested"),
            "Total number of records ingested through the gateway"
        );

        describe_histogram!(
            phase_metric!(histogram, "gateway", "payload_bytes"),
            "Size of payloads processed by the gateway in bytes"
        );
        describe_histogram!(
            phase_metric!(histogram, "gateway", "processing_duration_seconds"),
            "Duration of envelope processing in the gateway in seconds"
        );
        describe_histogram!(
            phase_metric!(histogram, "gateway", "cas_write_bytes"),
            "Size of data written to CAS in bytes"
        );
        describe_histogram!(
            phase_metric!(histogram, "gateway", "cas_operation_duration_seconds"),
            "Duration of CAS operations in seconds"
        );
    }

    fn phase_name() -> &'static str {
        "gateway"
    }

    fn metrics_documentation() -> Vec<MetricDoc> {
        vec![
            MetricDoc {
                name: phase_metric!(counter, "gateway", "envelopes_accepted"),
                metric_type: MetricType::Counter,
                help: "Total number of envelopes accepted by the gateway",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(counter, "gateway", "envelopes_deduplicated"),
                metric_type: MetricType::Counter,
                help: "Total number of envelopes that were deduplicated",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(counter, "gateway", "cas_writes_success"),
                metric_type: MetricType::Counter,
                help: "Total number of successful CAS writes",
                labels: vec!["storage_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "gateway", "cas_writes_error"),
                metric_type: MetricType::Counter,
                help: "Total number of failed CAS writes",
                labels: vec!["storage_type", "error_type"],
            },
            MetricDoc {
                name: phase_metric!(counter, "gateway", "policy_checks"),
                metric_type: MetricType::Counter,
                help: "Total number of policy checks performed",
                labels: vec!["source_id", "check_type", "result"],
            },
            MetricDoc {
                name: phase_metric!(counter, "gateway", "records_ingested"),
                metric_type: MetricType::Counter,
                help: "Total number of records ingested through the gateway (envelopes that will be parsed into records)",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "gateway", "payload_bytes"),
                metric_type: MetricType::Histogram,
                help: "Size of payloads processed by the gateway in bytes",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "gateway", "processing_duration_seconds"),
                metric_type: MetricType::Histogram,
                help: "Duration of envelope processing in the gateway in seconds",
                labels: vec!["source_id"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "gateway", "cas_write_bytes"),
                metric_type: MetricType::Histogram,
                help: "Size of data written to CAS in bytes",
                labels: vec!["storage_type"],
            },
            MetricDoc {
                name: phase_metric!(histogram, "gateway", "cas_operation_duration_seconds"),
                metric_type: MetricType::Histogram,
                help: "Duration of CAS operations in seconds",
                labels: vec!["storage_type"],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_metrics_registration() {
        GatewayMetrics::register_metrics();
    }

    #[test]
    fn test_metrics_documentation() {
        let docs = GatewayMetrics::metrics_documentation();
        assert_eq!(docs.len(), 10);

        for doc in docs {
            assert!(doc.name.starts_with("sms_gateway_"));
        }
    }
}
