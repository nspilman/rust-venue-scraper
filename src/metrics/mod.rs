//! Centralized metrics infrastructure for the SMS pipeline
//!
//! This module provides a type-safe, phase-organized approach to metrics collection.
//! Each pipeline phase defines its own metrics in a dedicated submodule, ensuring
//! clear ownership and preventing naming conflicts.

pub mod gateway;
pub mod ingest_log;
pub mod lib;
pub mod parser;
pub mod registry;
pub mod sources;

// Re-export the library components
pub use lib::{MetricsConfig, MetricsSystem};

// Re-export the metrics structs for easier importing
pub use gateway::GatewayMetrics;
pub use ingest_log::IngestLogMetrics;
pub use parser::ParserMetrics;
pub use sources::SourcesMetrics;

use std::sync::{Once, OnceLock};
use tracing::{info, warn};

static INIT: Once = Once::new();
static HANDLE: OnceLock<metrics_exporter_prometheus::PrometheusHandle> = OnceLock::new();

/// Initialize the global metrics infrastructure
///
/// Idempotent. This:
/// - Installs a Prometheus recorder with optional HTTP exporter (only if PROMETHEUS_ADDR or SMS_METRICS_ADDR is set)
/// - Registers all phase-specific metrics
/// - Stores a handle for in-process rendering so short-lived jobs can push directly to Pushgateway without scraping
pub fn init_metrics() {
    INIT.call_once(|| {
        let mut builder = metrics_exporter_prometheus::PrometheusBuilder::new();

        // Always start HTTP listener for metrics (required for render to work)
        let addr_str = std::env::var("PROMETHEUS_ADDR")
            .ok()
            .or_else(|| std::env::var("SMS_METRICS_ADDR").ok())
            .unwrap_or_else(|| "127.0.0.1:9898".to_string());
        
        if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
            builder = builder.with_http_listener(addr);
            info!("Prometheus HTTP exporter started at http://{}/metrics", addr);
            // Also set the scrape URL for pushgateway fallback
            std::env::set_var("SMS_METRICS_SCRAPE_URL", format!("http://{}/metrics", addr));
        } else {
            warn!("Invalid metrics addr '{}', using default 127.0.0.1:9898", addr_str);
            builder = builder.with_http_listener("127.0.0.1:9898".parse::<std::net::SocketAddr>().unwrap());
            std::env::set_var("SMS_METRICS_SCRAPE_URL", "http://127.0.0.1:9898/metrics");
        }

        // Always install the recorder (even without HTTP listener) for in-process rendering
        match builder.install_recorder() {
            Ok(handle) => {
                info!("METRICS: install_recorder() succeeded, got handle");
                let set_result = HANDLE.set(handle);
                if set_result.is_ok() {
                    info!("METRICS: Handle stored successfully in OnceLock");
                } else {
                    warn!("METRICS: Failed to store handle in OnceLock (already set?)");
                }
                info!("Prometheus recorder installed (handle available for in-process render)");

                // Register all phase metrics to validate naming and detect conflicts early
                registry::register_all_metrics();
                info!("All pipeline metrics registered successfully");
            }
            Err(e) => {
                warn!("Failed to install Prometheus recorder: {}", e);
            }
        }
    });
}

/// Trait for phase-specific metrics collections
///
/// Each pipeline phase implements this trait to provide:
/// - Metric registration at startup
/// - Consistent naming conventions
/// - Documentation of what each metric measures
pub trait PhaseMetrics {
    /// Register all metrics for this phase
    ///
    /// This is called during application startup to ensure all metrics
    /// are properly initialized and to detect naming conflicts early.
    fn register_metrics();

    /// Get the phase name for prefixing metrics
    fn phase_name() -> &'static str;

    /// Get documentation for all metrics in this phase
    ///
    /// Used for generating documentation and validation
    fn metrics_documentation() -> Vec<MetricDoc>;
}

/// Documentation for a single metric
#[derive(Debug, Clone)]
pub struct MetricDoc {
    pub name: &'static str,
    pub metric_type: MetricType,
    pub help: &'static str,
    #[allow(dead_code)]
    pub labels: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Histogram,
    Gauge,
}

/// Macro to create phase-specific metric instances with consistent naming
///
/// This ensures all metrics follow the naming convention:
/// sms_{phase}_{metric_name}_{type}
macro_rules! phase_metric {
    (counter, $phase:literal, $name:literal) => {
        concat!("sms_", $phase, "_", $name, "_total")
    };
    (histogram, $phase:literal, $name:literal) => {
        concat!("sms_", $phase, "_", $name)
    };
    (gauge, $phase:literal, $name:literal) => {
        concat!("sms_", $phase, "_", $name)
    };
}

pub(crate) use phase_metric;

/// Bump a heartbeat counter to guarantee a non-empty snapshot for short-lived jobs
pub fn bump_run_heartbeat() {
    info!("METRICS: About to increment heartbeat counter sms_runs_heartbeat_total");
    ::metrics::counter!("sms_runs_heartbeat_total").increment(1);
    info!("METRICS: Incremented heartbeat counter");
}

/// Push a simple set of key metrics directly to Pushgateway.
///
/// Since the metrics-exporter-prometheus crate's render() doesn't work properly,
/// we'll just manually format and push the most important metrics.
///
/// Env:
/// - SMS_PUSHGATEWAY_URL: base URL to Pushgateway (e.g., http://localhost:9091)
pub async fn push_all_to_pushgateway(instance: &str) {
    // Use the default metrics for now
    push_simple_metrics_to_pushgateway(instance).await;
}

/// Push enhanced metrics with additional context
pub async fn push_ingest_metrics(
    instance: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
    envelope_id: &str,
) {
    push_detailed_metrics_to_pushgateway(instance, bytes, duration_secs, success, envelope_id).await;
}

/// Push manually formatted metrics to Pushgateway
async fn push_simple_metrics_to_pushgateway(instance: &str) {
    let base = match std::env::var("SMS_PUSHGATEWAY_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            info!("pushgateway: SMS_PUSHGATEWAY_URL not configured, skipping push");
            return;
        }
    };

    let push_url = format!(
        "{}/metrics/job/{}/instance/{}",
        base.trim_end_matches('/'),
        "sms_scraper",
        instance
    );

    // Create a simple metric in Prometheus text format
    // This is just to prove the connection works
    let timestamp = chrono::Utc::now().timestamp_millis();
    let metrics_text = format!(
        "# HELP sms_ingest_run_timestamp_ms Last run timestamp in milliseconds\n\
         # TYPE sms_ingest_run_timestamp_ms gauge\n\
         sms_ingest_run_timestamp_ms{{}} {}\n\
         # HELP sms_ingest_runs_total Total number of ingest runs\n\
         # TYPE sms_ingest_runs_total counter\n\
         sms_ingest_runs_total{{}} 1\n",
        timestamp
    );

    let client = reqwest::Client::new();
    
    info!("pushgateway: pushing {} bytes to {}", metrics_text.len(), push_url);
    
    // Push to Pushgateway
    match client
        .post(&push_url)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_text)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            info!("pushgateway: successfully pushed metrics for instance={}", instance);
        }
        Ok(r) => {
            warn!("pushgateway: push failed with status={} for instance={}", r.status().as_u16(), instance);
        }
        Err(e) => {
            warn!("pushgateway: push request failed: {} for instance={}", e, instance);
        }
    }
}

/// Push detailed metrics with comprehensive ingest information
async fn push_detailed_metrics_to_pushgateway(
    instance: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
    envelope_id: &str,
) {
    let base = match std::env::var("SMS_PUSHGATEWAY_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            info!("pushgateway: SMS_PUSHGATEWAY_URL not configured, skipping push");
            return;
        }
    };

    let push_url = format!(
        "{}/metrics/job/{}/instance/{}",
        base.trim_end_matches('/'),
        "sms_scraper",
        instance
    );

    // Create comprehensive metrics in Prometheus text format
    let timestamp = chrono::Utc::now().timestamp_millis();
    let success_val = if success { 1 } else { 0 };
    let failure_val = if success { 0 } else { 1 };
    
    let metrics_text = format!(
        "# HELP sms_ingest_run_timestamp_ms Last run timestamp in milliseconds\n\
         # TYPE sms_ingest_run_timestamp_ms gauge\n\
         sms_ingest_run_timestamp_ms{{}} {}\n\
         \n\
         # HELP sms_ingest_runs_total Total number of ingest runs\n\
         # TYPE sms_ingest_runs_total counter\n\
         sms_ingest_runs_total{{}} 1\n\
         \n\
         # HELP sms_ingest_success_total Total number of successful ingests\n\
         # TYPE sms_ingest_success_total counter\n\
         sms_ingest_success_total{{}} {}\n\
         \n\
         # HELP sms_ingest_failure_total Total number of failed ingests\n\
         # TYPE sms_ingest_failure_total counter\n\
         sms_ingest_failure_total{{}} {}\n\
         \n\
         # HELP sms_ingest_bytes_total Total bytes ingested\n\
         # TYPE sms_ingest_bytes_total counter\n\
         sms_ingest_bytes_total{{}} {}\n\
         \n\
         # HELP sms_ingest_last_bytes Size of last ingest in bytes\n\
         # TYPE sms_ingest_last_bytes gauge\n\
         sms_ingest_last_bytes{{}} {}\n\
         \n\
         # HELP sms_ingest_duration_seconds Duration of ingest operation\n\
         # TYPE sms_ingest_duration_seconds histogram\n\
         sms_ingest_duration_seconds_bucket{{le=\"0.1\"}} {}\n\
         sms_ingest_duration_seconds_bucket{{le=\"0.5\"}} {}\n\
         sms_ingest_duration_seconds_bucket{{le=\"1\"}} {}\n\
         sms_ingest_duration_seconds_bucket{{le=\"5\"}} {}\n\
         sms_ingest_duration_seconds_bucket{{le=\"10\"}} {}\n\
         sms_ingest_duration_seconds_bucket{{le=\"+Inf\"}} 1\n\
         sms_ingest_duration_seconds_sum{{}} {}\n\
         sms_ingest_duration_seconds_count{{}} 1\n\
         \n\
         # HELP sms_ingest_last_envelope_id Last processed envelope ID\n\
         # TYPE sms_ingest_last_envelope_id gauge\n\
         # NOTE: This is a text metric stored as a comment for reference\n\
         # last_envelope_id=\"{}\"\n",
        timestamp,
        success_val,
        failure_val,
        bytes,
        bytes,
        if duration_secs <= 0.1 { 1 } else { 0 },
        if duration_secs <= 0.5 { 1 } else { 0 },
        if duration_secs <= 1.0 { 1 } else { 0 },
        if duration_secs <= 5.0 { 1 } else { 0 },
        if duration_secs <= 10.0 { 1 } else { 0 },
        duration_secs,
        envelope_id
    );

    let client = reqwest::Client::new();
    
    info!("pushgateway: pushing detailed metrics ({} bytes, {} secs, success={}) to {}", 
        bytes, duration_secs, success, push_url);
    
    // Push to Pushgateway
    match client
        .post(&push_url)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_text)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            info!("pushgateway: successfully pushed detailed metrics for instance={}", instance);
        }
        Ok(r) => {
            warn!("pushgateway: push failed with status={} for instance={}", r.status().as_u16(), instance);
        }
        Err(e) => {
            warn!("pushgateway: push request failed: {} for instance={}", e, instance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_naming_convention() {
        assert_eq!(
            phase_metric!(counter, "gateway", "envelopes_accepted"),
            "sms_gateway_envelopes_accepted_total"
        );
        assert_eq!(
            phase_metric!(histogram, "parser", "duration_seconds"),
            "sms_parser_duration_seconds"
        );
        assert_eq!(
            phase_metric!(gauge, "ingest_log", "consumer_lag_bytes"),
            "sms_ingest_log_consumer_lag_bytes"
        );
    }
}
