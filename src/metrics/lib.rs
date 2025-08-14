//! Unified metrics library for the SMS scraper system
//! 
//! This module provides a complete metrics solution including:
//! - Metric recording and collection
//! - Prometheus exporter setup
//! - Pushgateway integration for short-lived jobs
//! - Phase-specific metric helpers

use std::sync::OnceLock;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing::{info, warn};

// Re-export commonly used items
pub use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram};

// Re-export phase-specific metric modules
pub use crate::metrics::{
    gateway::GatewayMetrics,
    ingest_log::IngestLogMetrics,
    parser::ParserMetrics,
    sources::SourcesMetrics,
};

/// Global handle for Prometheus metrics exporter
static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Core metrics configuration
pub struct MetricsConfig {
    /// Address to bind the HTTP metrics server (e.g., "127.0.0.1:9898")
    pub http_addr: Option<String>,
    /// URL of the Pushgateway for pushing metrics
    pub pushgateway_url: Option<String>,
    /// Job name for Pushgateway metrics
    pub job_name: String,
    /// Whether to enable debug logging for metrics
    pub debug: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            http_addr: std::env::var("PROMETHEUS_ADDR")
                .ok()
                .or_else(|| Some("127.0.0.1:9898".to_string())),
            pushgateway_url: std::env::var("SMS_PUSHGATEWAY_URL").ok(),
            job_name: "sms_scraper".to_string(),
            debug: false,
        }
    }
}

/// Metrics system manager
pub struct MetricsSystem {
    config: MetricsConfig,
}

impl MetricsSystem {
    /// Create a new metrics system with default configuration
    pub fn new() -> Self {
        Self::with_config(MetricsConfig::default())
    }

    /// Create a new metrics system with custom configuration
    pub fn with_config(config: MetricsConfig) -> Self {
        Self { config }
    }

    /// Initialize the metrics system
    /// 
    /// This sets up the Prometheus recorder and optionally starts an HTTP server
    /// for metrics scraping. Must be called before recording any metrics.
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Build Prometheus exporter
        let mut builder = PrometheusBuilder::new();
        
        // Configure HTTP server if address is provided
        if let Some(addr) = &self.config.http_addr {
            let sock_addr = addr.parse()
                .map_err(|e| format!("Invalid metrics address '{}': {}", addr, e))?;
            
            builder = builder.with_http_listener(sock_addr);
            info!("Prometheus HTTP exporter will start at http://{}/metrics", addr);
        }

        // Install the recorder and get handle
        let handle = builder.install_recorder()
            .map_err(|e| format!("Failed to install Prometheus recorder: {}", e))?;
        
        // Store handle for later use
        if METRICS_HANDLE.set(handle).is_err() {
            return Err("Metrics system already initialized".into());
        }

        if self.config.debug {
            info!("Metrics system initialized successfully");
        }

        // Register all standard metrics
        self.register_all_metrics()?;

        Ok(())
    }

    /// Register all pipeline metrics with descriptions
    fn register_all_metrics(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Register phase-specific metrics
        SourcesMetrics::register();
        GatewayMetrics::register();
        IngestLogMetrics::register();
        ParserMetrics::register();

        // Register system-wide metrics
        describe_counter!(
            "sms_runs_heartbeat_total",
            "Heartbeat counter incremented at the start of each run"
        );

        describe_gauge!(
            "sms_ingest_run_timestamp_ms",
            "Timestamp of the last ingest run in milliseconds"
        );

        describe_counter!(
            "sms_ingest_runs_total",
            "Total number of ingest runs"
        );

        describe_counter!(
            "sms_ingest_success_total",
            "Total number of successful ingests"
        );

        describe_counter!(
            "sms_ingest_failure_total", 
            "Total number of failed ingests"
        );

        describe_counter!(
            "sms_ingest_bytes_total",
            "Total bytes ingested"
        );

        describe_gauge!(
            "sms_ingest_last_bytes",
            "Size of last ingest in bytes"
        );

        describe_histogram!(
            "sms_ingest_duration_seconds",
            "Duration of ingest operations in seconds"
        );

        if self.config.debug {
            info!("All metrics registered successfully");
        }

        Ok(())
    }

    /// Record a heartbeat metric to indicate the system is running
    pub fn record_heartbeat(&self) {
        counter!("sms_runs_heartbeat_total").increment(1);
        if self.config.debug {
            info!("Heartbeat recorded");
        }
    }

    /// Get the current metrics as Prometheus text format
    /// 
    /// Returns None if the metrics system is not initialized or HTTP server is not running
    pub fn render_metrics(&self) -> Option<String> {
        METRICS_HANDLE.get().map(|handle| handle.render())
    }

    /// Push metrics to Pushgateway (for short-lived jobs)
    pub async fn push_to_pushgateway(&self, instance: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pushgateway_url = self.config.pushgateway_url.as_ref()
            .ok_or("Pushgateway URL not configured")?;

        let metrics_text = self.render_metrics()
            .ok_or("No metrics available to push")?;

        let push_url = format!(
            "{}/metrics/job/{}/instance/{}",
            pushgateway_url.trim_end_matches('/'),
            self.config.job_name,
            instance
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics_text)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Pushgateway returned status: {}", response.status()).into());
        }

        if self.config.debug {
            info!("Successfully pushed metrics to Pushgateway for instance={}", instance);
        }

        Ok(())
    }

    /// Push detailed ingest metrics to Pushgateway
    /// 
    /// This creates a custom metrics payload with ingest-specific information
    pub async fn push_ingest_metrics(
        &self,
        source_id: &str,
        bytes: usize,
        duration_secs: f64,
        success: bool,
        envelope_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pushgateway_url = self.config.pushgateway_url.as_ref()
            .ok_or("Pushgateway URL not configured")?;

        let push_url = format!(
            "{}/metrics/job/{}/instance/{}",
            pushgateway_url.trim_end_matches('/'),
            self.config.job_name,
            source_id
        );

        // Create detailed metrics in Prometheus text format
        let timestamp = chrono::Utc::now().timestamp_millis();
        let metrics_text = self.format_ingest_metrics(
            timestamp,
            bytes,
            duration_secs,
            success,
            envelope_id,
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics_text)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Pushgateway returned status: {}", response.status()).into());
        }

        info!(
            "Successfully pushed ingest metrics for source={} ({} bytes, {:.2}s, success={})",
            source_id, bytes, duration_secs, success
        );

        Ok(())
    }

    /// Format ingest metrics as Prometheus text
    fn format_ingest_metrics(
        &self,
        timestamp: i64,
        bytes: usize,
        duration_secs: f64,
        success: bool,
        envelope_id: &str,
    ) -> String {
        let success_val = if success { 1 } else { 0 };
        let failure_val = if success { 0 } else { 1 };
        
        format!(
            "# HELP sms_ingest_run_timestamp_ms Last run timestamp in milliseconds\n\
             # TYPE sms_ingest_run_timestamp_ms gauge\n\
             sms_ingest_run_timestamp_ms {}\n\
             \n\
             # HELP sms_ingest_runs_total Total number of ingest runs\n\
             # TYPE sms_ingest_runs_total counter\n\
             sms_ingest_runs_total 1\n\
             \n\
             # HELP sms_ingest_success_total Total number of successful ingests\n\
             # TYPE sms_ingest_success_total counter\n\
             sms_ingest_success_total {}\n\
             \n\
             # HELP sms_ingest_failure_total Total number of failed ingests\n\
             # TYPE sms_ingest_failure_total counter\n\
             sms_ingest_failure_total {}\n\
             \n\
             # HELP sms_ingest_bytes_total Total bytes ingested\n\
             # TYPE sms_ingest_bytes_total counter\n\
             sms_ingest_bytes_total {}\n\
             \n\
             # HELP sms_ingest_last_bytes Size of last ingest in bytes\n\
             # TYPE sms_ingest_last_bytes gauge\n\
             sms_ingest_last_bytes {}\n\
             \n\
             # HELP sms_ingest_duration_seconds Duration of ingest operation\n\
             # TYPE sms_ingest_duration_seconds histogram\n\
             sms_ingest_duration_seconds_bucket{{le=\"0.1\"}} {}\n\
             sms_ingest_duration_seconds_bucket{{le=\"0.5\"}} {}\n\
             sms_ingest_duration_seconds_bucket{{le=\"1\"}} {}\n\
             sms_ingest_duration_seconds_bucket{{le=\"5\"}} {}\n\
             sms_ingest_duration_seconds_bucket{{le=\"10\"}} {}\n\
             sms_ingest_duration_seconds_bucket{{le=\"+Inf\"}} 1\n\
             sms_ingest_duration_seconds_sum {}\n\
             sms_ingest_duration_seconds_count 1\n\
             \n\
             # NOTE: Envelope ID stored as comment (Prometheus doesn't support text metrics)\n\
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
        )
    }
}

/// Convenience functions for common metrics operations

/// Initialize metrics with default configuration
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    MetricsSystem::new().init()
}

/// Initialize metrics with custom configuration
pub fn init_with_config(config: MetricsConfig) -> Result<(), Box<dyn std::error::Error>> {
    MetricsSystem::with_config(config).init()
}

/// Record a heartbeat metric
pub fn heartbeat() {
    counter!("sms_runs_heartbeat_total").increment(1);
}

/// Push all current metrics to Pushgateway
pub async fn push_to_pushgateway(instance: &str) -> Result<(), Box<dyn std::error::Error>> {
    MetricsSystem::new().push_to_pushgateway(instance).await
}

/// Push detailed ingest metrics
pub async fn push_ingest_metrics(
    source_id: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
    envelope_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    MetricsSystem::new().push_ingest_metrics(
        source_id,
        bytes,
        duration_secs,
        success,
        envelope_id,
    ).await
}

/// Helper macro to create phase-specific metric names
#[macro_export]
macro_rules! phase_metric {
    (counter, $phase:expr, $name:expr) => {
        concat!("sms_", $phase, "_", $name, "_total")
    };
    (histogram, $phase:expr, $name:expr) => {
        concat!("sms_", $phase, "_", $name)
    };
    (gauge, $phase:expr, $name:expr) => {
        concat!("sms_", $phase, "_", $name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert_eq!(config.job_name, "sms_scraper");
    }

    #[test]
    fn test_phase_metric_macro() {
        assert_eq!(
            phase_metric!(counter, "gateway", "requests"),
            "sms_gateway_requests_total"
        );
        assert_eq!(
            phase_metric!(histogram, "parser", "duration"),
            "sms_parser_duration"
        );
        assert_eq!(
            phase_metric!(gauge, "ingest", "queue_size"),
            "sms_ingest_queue_size"
        );
    }
}
