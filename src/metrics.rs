//! Simple metrics module for the SMS scraper system
//! 
//! This module provides a straightforward API for recording metrics using
//! the standard Prometheus naming conventions.

pub mod dashboard;

use tracing::{info, warn};
use std::sync::Arc;

/// Initialize the metrics system with optional push gateway support
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    init_with_push_options(None, None)
}

/// Initialize with push gateway configuration
pub fn init_with_push_options(
    job_name: Option<&str>,
    instance: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    
    // Install the recorder and get the handle
    let handle = builder
        .install_recorder()
        .map_err(|e| format!("Failed to install Prometheus recorder: {}", e))?;
    
    // If push gateway is configured, store the handle for later pushing
    if let Ok(pushgateway_url) = std::env::var("SMS_PUSHGATEWAY_URL") {
        let job = job_name.unwrap_or("sms_scraper");
        let inst = instance.unwrap_or("default");
        
        // Store handle for push_all_metrics function
        METRICS_HANDLE.set(Arc::new(MetricsState {
            handle,
            pushgateway_url,
            job: job.to_string(),
            instance: inst.to_string(),
        })).ok();
        
        info!("Metrics system initialized with push gateway support");
    } else {
        info!("Metrics system initialized (no push gateway)");
    }
    
    Ok(())
}

// Global state for metrics pushing
use std::sync::OnceLock;
static METRICS_HANDLE: OnceLock<Arc<MetricsState>> = OnceLock::new();

struct MetricsState {
    handle: metrics_exporter_prometheus::PrometheusHandle,
    pushgateway_url: String,
    job: String,
    instance: String,
}

/// Internal function to push a single metric immediately
async fn push_single_metric(name: &str, value: f64, metric_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(state) = METRICS_HANDLE.get() {
        let push_url = format!(
            "{}/metrics/job/{}/instance/{}",
            state.pushgateway_url.trim_end_matches('/'),
            state.job,
            state.instance
        );
        
        let metrics_text = format!(
            "# TYPE {} {}\n{} {}\n",
            name, metric_type, name, value
        );
        
        let client = reqwest::Client::new();
        let _ = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics_text)
            .send()
            .await?;
    }
    Ok(())
}

/// Internal function to push histogram metrics with buckets
async fn push_histogram_metric(name: &str, value: f64) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(state) = METRICS_HANDLE.get() {
        let push_url = format!(
            "{}/metrics/job/{}/instance/{}",
            state.pushgateway_url.trim_end_matches('/'),
            state.job,
            state.instance
        );
        
        // Define standard bucket boundaries (in bytes for payload size)
        let buckets = vec![
            1_000.0,      // 1KB
            10_000.0,     // 10KB
            100_000.0,    // 100KB
            1_000_000.0,  // 1MB
            10_000_000.0, // 10MB
            f64::INFINITY,
        ];
        
        // Build histogram metric text with buckets
        let mut metrics_text = format!("# TYPE {} histogram\n", name);
        
        // Add bucket entries
        let mut cumulative_count = 0u64;
        for bucket_bound in &buckets {
            if value <= *bucket_bound {
                cumulative_count = 1;
            }
            let le_value = if *bucket_bound == f64::INFINITY { 
                "+Inf".to_string() 
            } else { 
                bucket_bound.to_string() 
            };
            metrics_text.push_str(&format!(
                "{}_bucket{{le=\"{}\"}} {}\n",
                name,
                le_value,
                cumulative_count
            ));
        }
        
        // Add sum and count
        metrics_text.push_str(&format!(
            "{}_sum {}\n{}_count 1\n",
            name, value, name
        ));
        
        let client = reqwest::Client::new();
        let _ = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics_text)
            .send()
            .await?;
    }
    Ok(())
}

// Macro to automatically push metrics
macro_rules! counter_and_push {
    ($name:expr) => {{
        ::metrics::counter!($name).increment(1);
        let name = $name.to_string();
        tokio::spawn(async move {
            let _ = push_single_metric(&name, 1.0, "counter").await;
        });
    }};
    ($name:expr, $($label_key:expr => $label_value:expr),+) => {{
        ::metrics::counter!($name, $($label_key => $label_value),+).increment(1);
        let name = $name.to_string();
        tokio::spawn(async move {
            let _ = push_single_metric(&name, 1.0, "counter").await;
        });
    }};
}

macro_rules! gauge_and_push {
    ($name:expr, $value:expr) => {{
        let v = $value as f64;
        ::metrics::gauge!($name).set(v);
        let name = $name.to_string();
        tokio::spawn(async move {
            let _ = push_single_metric(&name, v, "gauge").await;
        });
    }};
}

macro_rules! histogram_and_push {
    ($name:expr, $value:expr) => {{
        let v = $value as f64;
        ::metrics::histogram!($name).record(v);
        let name = $name.to_string();
        tokio::spawn(async move {
            let _ = push_single_metric(&name, v, "gauge").await;
        });
    }};
}

/// Record a heartbeat for testing
pub fn heartbeat() {
    ::metrics::counter!("sms_heartbeat_total").increment(1);
    tokio::spawn(async {
        let _ = push_single_metric("sms_heartbeat_total", 1.0, "counter").await;
    });
}

// ============================================================================
// Sources Metrics
// ============================================================================

pub mod sources {
    use super::{push_single_metric, push_histogram_metric};
    
    /// Record a successful request
    pub fn request_success() {
        ::metrics::counter!("sms_sources_requests_success_total").increment(1);
        // Immediately push this metric
        tokio::spawn(async {
            let _ = push_single_metric("sms_sources_requests_success_total", 1.0, "counter").await;
        });
    }
    
    /// Record a failed request
    pub fn request_error() {
        ::metrics::counter!("sms_sources_requests_error_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_sources_requests_error_total", 1.0, "counter").await;
        });
    }
    
    /// Record request duration
    pub fn request_duration(secs: f64) {
        ::metrics::histogram!("sms_sources_request_duration_seconds").record(secs);
        tokio::spawn(async move {
            let _ = push_single_metric("sms_sources_request_duration_seconds", secs, "gauge").await;
        });
    }
    
    /// Record payload size
    pub fn payload_bytes(bytes: usize) {
        let b = bytes as f64;
        ::metrics::histogram!("sms_sources_payload_bytes").record(b);
        tokio::spawn(async move {
            // Push histogram with buckets instead of single value
            let _ = push_histogram_metric("sms_sources_payload_bytes", b).await;
        });
    }
    
    /// Record successful registry load
    pub fn registry_load_success() {
        counter_and_push!("sms_sources_registry_loads_success_total");
    }
    
    /// Record failed registry load
    pub fn registry_load_error() {
        counter_and_push!("sms_sources_registry_loads_error_total");
    }
}

// ============================================================================
// Gateway Metrics
// ============================================================================

pub mod gateway {
    use super::push_single_metric;
    
    /// Record an accepted envelope
    pub fn envelope_accepted() {
        ::metrics::counter!("sms_gateway_envelopes_accepted_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_gateway_envelopes_accepted_total", 1.0, "counter").await;
        });
    }
    
    /// Record a deduplicated envelope
    pub fn envelope_deduplicated() {
        ::metrics::counter!("sms_gateway_envelopes_deduplicated_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_gateway_envelopes_deduplicated_total", 1.0, "counter").await;
        });
    }
    
    /// Record successful CAS write
    pub fn cas_write_success() {
        ::metrics::counter!("sms_gateway_cas_writes_success_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_gateway_cas_writes_success_total", 1.0, "counter").await;
        });
    }
    
    /// Record failed CAS write
    pub fn cas_write_error() {
        ::metrics::counter!("sms_gateway_cas_writes_error_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_gateway_cas_writes_error_total", 1.0, "counter").await;
        });
    }
    
    /// Record ingested records
    pub fn records_ingested(count: u64) {
        ::metrics::counter!("sms_gateway_records_ingested_total").increment(count);
        let c = count as f64;
        tokio::spawn(async move {
            let _ = push_single_metric("sms_gateway_records_ingested_total", c, "counter").await;
        });
    }
    
    /// Record processing duration
    pub fn processing_duration(secs: f64) {
        ::metrics::histogram!("sms_gateway_processing_duration_seconds").record(secs);
        tokio::spawn(async move {
            let _ = push_single_metric("sms_gateway_processing_duration_seconds", secs, "gauge").await;
        });
    }
    
    /// Record successful ingest for a source
    pub fn ingest_success(source_id: &str) {
        ::metrics::counter!("sms_gateway_ingest_success_total", "source_id" => source_id.to_string()).increment(1);
    }
    
    /// Record failed ingest for a source
    pub fn ingest_error(source_id: &str, error_type: &str) {
        ::metrics::counter!("sms_gateway_ingest_error_total", 
            "source_id" => source_id.to_string(),
            "error_type" => error_type.to_string()
        ).increment(1);
    }
    
    /// Record bytes ingested for a source
    pub fn bytes_ingested(source_id: &str, bytes: u64) {
        ::metrics::histogram!("sms_gateway_bytes_ingested", "source_id" => source_id.to_string()).record(bytes as f64);
    }
    
    /// Record ingest duration for a source
    pub fn duration(source_id: &str, secs: f64) {
        ::metrics::histogram!("sms_gateway_ingest_duration_seconds", "source_id" => source_id.to_string()).record(secs);
    }
}

// ============================================================================
// Ingest Log Metrics
// ============================================================================

pub mod ingest_log {
    use super::push_single_metric;
    
    /// Record successful write
    pub fn write_success() {
        ::metrics::counter!("sms_ingest_log_writes_success_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_ingest_log_writes_success_total", 1.0, "counter").await;
        });
    }
    
    /// Record failed write
    pub fn write_error() {
        ::metrics::counter!("sms_ingest_log_writes_error_total").increment(1);
        tokio::spawn(async {
            let _ = push_single_metric("sms_ingest_log_writes_error_total", 1.0, "counter").await;
        });
    }
    
    /// Record write size
    pub fn write_bytes(bytes: usize) {
        let b = bytes as f64;
        ::metrics::histogram!("sms_ingest_log_write_bytes").record(b);
        tokio::spawn(async move {
            let _ = push_single_metric("sms_ingest_log_write_bytes", b, "gauge").await;
        });
    }
    
    /// Record log rotation
    pub fn rotation() {
        ::metrics::counter!("sms_ingest_log_rotations_total").increment(1);
    }
    
    /// Set current file size
    pub fn current_file_bytes(bytes: u64) {
        ::metrics::gauge!("sms_ingest_log_current_file_bytes").set(bytes as f64);
    }
    
    /// Set active consumers count
    pub fn active_consumers(count: usize) {
        ::metrics::gauge!("sms_ingest_log_active_consumers").set(count as f64);
    }
}

// ============================================================================
// Parser Metrics
// ============================================================================

pub mod parser {
    /// Record successful parse
    pub fn parse_success() {
        ::metrics::counter!("sms_parser_parse_success_total").increment(1);
    }
    
    /// Record parse error
    pub fn parse_error() {
        ::metrics::counter!("sms_parser_parse_error_total").increment(1);
    }
    
    /// Record parse duration
    pub fn duration(secs: f64) {
        ::metrics::histogram!("sms_parser_duration_seconds").record(secs);
    }
    
    /// Record extracted records
    pub fn records_extracted(count: u64) {
        ::metrics::counter!("sms_parser_records_extracted_total").increment(count);
    }
    
    /// Record bytes processed
    pub fn bytes_processed(bytes: usize) {
        ::metrics::histogram!("sms_parser_bytes_processed").record(bytes as f64);
    }
    
    /// Record batch size
    pub fn batch_size(size: usize) {
        ::metrics::histogram!("sms_parser_batch_size").record(size as f64);
    }
}

// ============================================================================
// Pushgateway Support (for short-lived jobs)
// ============================================================================

/// Push ingest metrics - wrapper for compatibility with existing code
pub async fn push_ingest_metrics(
    source_id: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
    envelope_id: &str,
) {
    // Record the metrics locally first
    if success {
        gateway::ingest_success(source_id);
        gateway::bytes_ingested(source_id, bytes as u64);
        gateway::duration(source_id, duration_secs);
        ::metrics::counter!("sms_gateway_envelope_created", 
            "source_id" => source_id.to_string(), 
            "envelope_id" => envelope_id.to_string()
        ).increment(1);
    } else {
        gateway::ingest_error(source_id, "fetch_failed");
    }
    
    // Try to push to pushgateway if configured
    if let Err(e) = push_to_pushgateway(source_id, bytes, duration_secs, success).await {
        warn!("Failed to push metrics to pushgateway: {}", e);
    }
}

/// Push metrics to Pushgateway
pub async fn push_to_pushgateway(
    instance: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("SMS_PUSHGATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:9091".to_string());
    
    let push_url = format!(
        "{}/metrics/job/sms_scraper/instance/{}",
        base.trim_end_matches('/'),
        instance
    );
    
    // Create simple metrics in Prometheus text format
    let timestamp = chrono::Utc::now().timestamp_millis();
    let metrics_text = format!(
        "# HELP sms_ingest_timestamp_ms Last ingest timestamp\n\
         # TYPE sms_ingest_timestamp_ms gauge\n\
         sms_ingest_timestamp_ms {}\n\
         # HELP sms_ingest_bytes Total bytes ingested\n\
         # TYPE sms_ingest_bytes gauge\n\
         sms_ingest_bytes {}\n\
         # HELP sms_ingest_duration_seconds Ingest duration\n\
         # TYPE sms_ingest_duration_seconds gauge\n\
         sms_ingest_duration_seconds {}\n\
         # HELP sms_ingest_success Ingest success (1) or failure (0)\n\
         # TYPE sms_ingest_success gauge\n\
         sms_ingest_success {}\n",
        timestamp,
        bytes,
        duration_secs,
        if success { 1 } else { 0 }
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
    
    info!("Successfully pushed metrics to Pushgateway for instance={}", instance);
    Ok(())
}

/// Push ALL collected metrics to Pushgateway
pub async fn push_all_metrics() -> Result<(), Box<dyn std::error::Error>> {
    push_all_metrics_with_instance("default").await
}

/// Push ALL collected metrics to Pushgateway with custom instance label
pub async fn push_all_metrics_with_instance(instance: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pushgateway_url = std::env::var("SMS_PUSHGATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:9091".to_string());
    
    let push_url = format!(
        "{}/metrics/job/sms_scraper/instance/{}",
        pushgateway_url.trim_end_matches('/'),
        instance
    );
    
    // Build metrics text by iterating through known metrics
    // This is a simplified approach that won't capture histogram buckets,
    // but will ensure all basic metrics are pushed
    let mut metrics_text = String::new();
    
    // Add a timestamp marker
    let timestamp = chrono::Utc::now().timestamp_millis();
    metrics_text.push_str(&format!(
        "# HELP sms_push_timestamp_ms Last push timestamp\n\
         # TYPE sms_push_timestamp_ms gauge\n\
         sms_push_timestamp_ms {}\n",
        timestamp
    ));
    
    // Try to render from the handle if available
    if let Some(state) = METRICS_HANDLE.get() {
        // Try to get rendered metrics directly
        let rendered = state.handle.render();
        if !rendered.is_empty() {
            info!("Rendered {} bytes of metrics directly", rendered.len());
            metrics_text.push_str(&rendered);
        } else {
            // If render() returns empty, we'll push a marker indicating metrics are initialized
            metrics_text.push_str(
                "# HELP sms_metrics_initialized Whether metrics system is initialized\n\
                 # TYPE sms_metrics_initialized gauge\n\
                 sms_metrics_initialized 1\n"
            );
        }
    } else {
        warn!("Metrics not initialized with push gateway support.");
        metrics_text.push_str(
            "# HELP sms_metrics_initialized Whether metrics system is initialized\n\
             # TYPE sms_metrics_initialized gauge\n\
             sms_metrics_initialized 0\n"
        );
    }
    
    let client = reqwest::Client::new();
    let response = client
        .post(&push_url)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_text)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Pushgateway returned status {}: {}", status, body).into());
    }
    
    info!("Successfully pushed metrics to Pushgateway for instance={}", instance);
    Ok(())
}
