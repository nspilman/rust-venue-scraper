# Metrics Library

A unified, abstracted metrics library for the SMS scraper system that provides clean interfaces for metric collection, Prometheus exporting, and Pushgateway integration.

## Architecture

The metrics system is organized into several layers:

1. **`lib.rs`** - Core metrics library with `MetricsSystem` and configuration
2. **Phase-specific modules** - Domain-specific metrics for each pipeline phase
3. **`mod.rs`** - Main module that re-exports everything and maintains backward compatibility

## Key Components

### MetricsSystem

The main entry point for metrics operations:

```rust
use crate::metrics::{MetricsConfig, MetricsSystem};

// Initialize with default configuration
let metrics = MetricsSystem::new();
metrics.init()?;

// Or with custom configuration
let config = MetricsConfig {
    http_addr: Some("0.0.0.0:9090".to_string()),
    pushgateway_url: Some("http://pushgateway:9091".to_string()),
    job_name: "my_job".to_string(),
    debug: true,
};
let metrics = MetricsSystem::with_config(config);
metrics.init()?;
```

### Recording Metrics

Use phase-specific metric structs for type-safe recording:

```rust
use crate::metrics::{GatewayMetrics, ParserMetrics, SourcesMetrics};

// Gateway metrics
GatewayMetrics::record_envelope_accepted("blue_moon", 1024, 0.5);
GatewayMetrics::record_rate_limit_throttle("blue_moon", 1.0);

// Parser metrics
ParserMetrics::record_parse_success("blue_moon", "html_parser", 10);
ParserMetrics::record_parse_error("blue_moon", "json_parser", "invalid_json");

// Sources metrics
SourcesMetrics::record_registry_load_success("blue_moon");
```

### Pushing to Pushgateway

For short-lived jobs that need to push metrics:

```rust
// Push all current metrics
metrics.push_to_pushgateway("job_instance").await?;

// Push specific ingest metrics
metrics.push_ingest_metrics(
    "blue_moon",     // source_id
    167064,          // bytes
    1.38,            // duration_secs
    true,            // success
    "envelope-123",  // envelope_id
).await?;
```

## Usage Patterns

### Long-Running Services (Server)

```rust
#[tokio::main]
async fn main() {
    // Initialize metrics with HTTP server for scraping
    let config = MetricsConfig {
        http_addr: Some("0.0.0.0:9464".to_string()),
        ..Default::default()
    };
    
    let metrics = MetricsSystem::with_config(config);
    metrics.init().expect("Failed to init metrics");
    
    // Record metrics throughout the service lifetime
    loop {
        // ... do work ...
        GatewayMetrics::record_envelope_accepted("source", bytes, duration);
    }
}
```

### Short-Lived Jobs (Ingester)

```rust
#[tokio::main]
async fn main() {
    // Initialize metrics
    let metrics = MetricsSystem::new();
    metrics.init().expect("Failed to init metrics");
    
    // Record heartbeat
    metrics.record_heartbeat();
    
    // Do work and record metrics
    let start = std::time::Instant::now();
    let result = ingest_data().await;
    let duration = start.elapsed().as_secs_f64();
    
    // Push results to Pushgateway
    match result {
        Ok(bytes) => {
            metrics.push_ingest_metrics(
                "source_id",
                bytes,
                duration,
                true,
                "envelope_id"
            ).await?;
        }
        Err(_) => {
            metrics.push_ingest_metrics(
                "source_id",
                0,
                duration,
                false,
                "error"
            ).await?;
        }
    }
}
```

## Environment Variables

- `PROMETHEUS_ADDR` - Address for HTTP metrics server (default: "127.0.0.1:9898")
- `SMS_PUSHGATEWAY_URL` - URL of Pushgateway for pushing metrics
- `SMS_METRICS_SCRAPE_URL` - Override URL for scraping metrics

## Convenience Functions

For simpler use cases, convenience functions are available:

```rust
use crate::metrics;

// Initialize with defaults
metrics::init()?;

// Record heartbeat
metrics::heartbeat();

// Push to Pushgateway
metrics::push_to_pushgateway("instance").await?;

// Push ingest metrics
metrics::push_ingest_metrics(
    "source_id",
    bytes,
    duration_secs,
    success,
    envelope_id
).await?;
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        let config = MetricsConfig::default();
        let metrics = MetricsSystem::with_config(config);
        assert!(metrics.init().is_ok());
    }
}
```

## Metric Naming Convention

All metrics follow a consistent naming pattern:
- **Counters**: `sms_{phase}_{name}_total`
- **Histograms**: `sms_{phase}_{name}`
- **Gauges**: `sms_{phase}_{name}`

Examples:
- `sms_gateway_envelopes_accepted_total`
- `sms_parser_duration_seconds`
- `sms_ingest_log_consumer_lag_bytes`
