//! Metrics registry for coordinating phase-specific metrics
//!
//! This module handles the registration of all metrics from different phases,
//! validates naming consistency, and detects conflicts early.

use crate::metrics::{MetricDoc, PhaseMetrics};
use std::collections::HashMap;
use tracing::{info, warn};

/// Register all metrics from all phases
///
/// This function calls the registration methods for each phase and validates
/// that there are no naming conflicts between phases.
pub fn register_all_metrics() {
    let mut all_metrics = HashMap::new();

    // Register metrics for each phase
    register_phase_metrics::<super::sources::SourcesMetrics>(&mut all_metrics);
    register_phase_metrics::<super::gateway::GatewayMetrics>(&mut all_metrics);
    register_phase_metrics::<super::ingest_log::IngestLogMetrics>(&mut all_metrics);
    register_phase_metrics::<super::parser::ParserMetrics>(&mut all_metrics);

    info!(
        "Registered {} total metrics across all phases",
        all_metrics.len()
    );

    // Log metric summary for debugging
    if std::env::var("SMS_METRICS_DEBUG").is_ok() {
        log_metrics_summary(&all_metrics);
    }
}

/// Register metrics for a specific phase and detect conflicts
fn register_phase_metrics<T: PhaseMetrics>(all_metrics: &mut HashMap<String, MetricDoc>) {
    T::register_metrics();
    let phase_docs = T::metrics_documentation();
    let phase_name = T::phase_name();

    info!(
        "Registering {} metrics for phase '{}'",
        phase_docs.len(),
        phase_name
    );

    for doc in phase_docs {
        if let Some(existing) = all_metrics.get(doc.name) {
            warn!(
                "Metric name conflict detected: '{}' is defined in both '{}' phase and current phase '{}'",
                doc.name,
                existing.help, // Using help as a proxy for identifying the original phase
                phase_name
            );
        } else {
            all_metrics.insert(doc.name.to_string(), doc);
        }
    }
}

/// Log a summary of all registered metrics for debugging
fn log_metrics_summary(all_metrics: &HashMap<String, MetricDoc>) {
    info!("=== Metrics Registry Summary ===");

    let mut by_phase: HashMap<&str, Vec<&MetricDoc>> = HashMap::new();

    // Group metrics by phase (extract from metric name prefix)
    for doc in all_metrics.values() {
        let phase = extract_phase_from_metric_name(doc.name);
        by_phase.entry(phase).or_default().push(doc);
    }

    for (phase, metrics) in by_phase {
        info!("Phase '{}': {} metrics", phase, metrics.len());
        for metric in metrics {
            info!(
                "  - {} ({}): {}",
                metric.name,
                format!("{:?}", metric.metric_type),
                metric.help
            );
        }
    }

    info!("=== End Metrics Summary ===");
}

/// Extract phase name from metric name (e.g., "sms_gateway_envelopes_total" -> "gateway")
fn extract_phase_from_metric_name(metric_name: &str) -> &str {
    if let Some(stripped) = metric_name.strip_prefix("sms_") {
        if let Some(next_underscore) = stripped.find('_') {
            return &stripped[..next_underscore];
        }
    }
    "unknown"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_phase_from_metric_name() {
        assert_eq!(
            extract_phase_from_metric_name("sms_gateway_envelopes_total"),
            "gateway"
        );
        assert_eq!(
            extract_phase_from_metric_name("sms_parser_duration_seconds"),
            "parser"
        );
        assert_eq!(
            extract_phase_from_metric_name("sms_sources_requests_total"),
            "sources"
        );
        assert_eq!(
            extract_phase_from_metric_name("invalid_metric_name"),
            "unknown"
        );
    }
}
