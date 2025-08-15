# Metrics Architecture: Single Source of Truth

## Vision

A unified metrics system where each metric is defined once and used everywhere:
1. **Definition** - Central catalog with all metric metadata
2. **Runtime** - Direct references in code for tracking events  
3. **Visualization** - Automatic dashboard generation from definitions

## Current Problem

The existing implementation has metrics scattered across multiple files with:
- Metric names hardcoded as strings in multiple places
- No compile-time verification that dashboards match actual metrics
- Manual synchronization required between code and dashboards
- Risk of drift between what we measure and what we visualize

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Metrics Catalog                         │
│                  (Single Source of Truth)                   │
├─────────────────────────────────────────────────────────────┤
│  • Metric definitions with all metadata                     │
│  • Type-safe metric keys                                    │
│  • Labels, units, descriptions                              │
│  • Dashboard hints (panel type, aggregations)               │
└────────────┬────────────────────────┬───────────────────────┘
             │                        │
             ▼                        ▼
    ┌────────────────┐       ┌────────────────┐
    │  Runtime Code  │       │Dashboard Builder│
    ├────────────────┤       ├────────────────┤
    │ Uses metric    │       │ Reads catalog  │
    │ keys directly  │       │ Generates JSON │
    └────────────────┘       └────────────────┘
             │                        │
             ▼                        ▼
      [Prometheus]              [Grafana]
```

## Implementation Plan

### Phase 1: Metric Catalog Structure

```rust
// src/metrics/catalog.rs

use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MetricDefinition {
    /// Full metric name (e.g., "sms_sources_requests_success_total")
    pub name: &'static str,
    
    /// Metric type
    pub metric_type: MetricType,
    
    /// Human-readable description
    pub help: &'static str,
    
    /// Labels this metric accepts
    pub labels: &'static [&'static str],
    
    /// Unit of measurement
    pub unit: MetricUnit,
    
    /// Dashboard visualization hints
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram { buckets: &'static [f64] },
}

#[derive(Debug, Clone)]
pub enum MetricUnit {
    None,
    Bytes,
    Seconds,
    Requests,
    Milliseconds,
}

#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Suggested panel type in Grafana
    pub panel_type: PanelType,
    
    /// Common queries for this metric
    pub queries: &'static [QueryTemplate],
    
    /// Suggested aggregations
    pub aggregations: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub enum PanelType {
    Stat,
    TimeSeries,
    Heatmap,
    Table,
}

#[derive(Debug, Clone)]
pub struct QueryTemplate {
    pub name: &'static str,
    pub expr: &'static str,
}

// Static metric definitions
pub static METRICS: Lazy<HashMap<&'static str, MetricDefinition>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // Sources Phase Metrics
    m.insert("sources.requests.success", MetricDefinition {
        name: "sms_sources_requests_success_total",
        metric_type: MetricType::Counter,
        help: "Total number of successful requests to data sources",
        labels: &["source_id"],
        unit: MetricUnit::Requests,
        dashboard: DashboardConfig {
            panel_type: PanelType::TimeSeries,
            queries: &[
                QueryTemplate {
                    name: "Rate per minute",
                    expr: "rate(sms_sources_requests_success_total[5m]) * 60",
                },
            ],
            aggregations: &["sum", "rate"],
        },
    });
    
    m.insert("sources.requests.error", MetricDefinition {
        name: "sms_sources_requests_error_total",
        metric_type: MetricType::Counter,
        help: "Total number of failed requests to data sources",
        labels: &["source_id", "error_type"],
        unit: MetricUnit::Requests,
        dashboard: DashboardConfig {
            panel_type: PanelType::TimeSeries,
            queries: &[
                QueryTemplate {
                    name: "Error rate",
                    expr: "rate(sms_sources_requests_error_total[5m]) * 60",
                },
            ],
            aggregations: &["sum", "rate"],
        },
    });
    
    m.insert("sources.request.duration", MetricDefinition {
        name: "sms_sources_request_duration_seconds",
        metric_type: MetricType::Histogram {
            buckets: &[0.1, 0.5, 1.0, 2.5, 5.0, 10.0],
        },
        help: "Duration of requests to data sources",
        labels: &["source_id"],
        unit: MetricUnit::Seconds,
        dashboard: DashboardConfig {
            panel_type: PanelType::TimeSeries,
            queries: &[
                QueryTemplate {
                    name: "p50",
                    expr: "histogram_quantile(0.50, rate(sms_sources_request_duration_seconds_bucket[5m]))",
                },
                QueryTemplate {
                    name: "p95",
                    expr: "histogram_quantile(0.95, rate(sms_sources_request_duration_seconds_bucket[5m]))",
                },
                QueryTemplate {
                    name: "p99",
                    expr: "histogram_quantile(0.99, rate(sms_sources_request_duration_seconds_bucket[5m]))",
                },
            ],
            aggregations: &["histogram_quantile"],
        },
    });
    
    // Gateway Phase Metrics
    m.insert("gateway.envelopes.accepted", MetricDefinition {
        name: "sms_gateway_envelopes_accepted_total",
        metric_type: MetricType::Counter,
        help: "Total number of envelopes accepted by the gateway",
        labels: &["source_id"],
        unit: MetricUnit::None,
        dashboard: DashboardConfig {
            panel_type: PanelType::TimeSeries,
            queries: &[
                QueryTemplate {
                    name: "Acceptance rate",
                    expr: "rate(sms_gateway_envelopes_accepted_total[5m]) * 60",
                },
            ],
            aggregations: &["sum", "rate"],
        },
    });
    
    // ... more metrics ...
    
    m
});

// Type-safe metric keys
pub struct Metrics;

impl Metrics {
    pub const SOURCES_REQUESTS_SUCCESS: &'static str = "sources.requests.success";
    pub const SOURCES_REQUESTS_ERROR: &'static str = "sources.requests.error";
    pub const SOURCES_REQUEST_DURATION: &'static str = "sources.request.duration";
    
    pub const GATEWAY_ENVELOPES_ACCEPTED: &'static str = "gateway.envelopes.accepted";
    pub const GATEWAY_ENVELOPES_DEDUPLICATED: &'static str = "gateway.envelopes.deduplicated";
    pub const GATEWAY_CAS_WRITES_SUCCESS: &'static str = "gateway.cas.writes.success";
    
    pub const INGEST_LOG_WRITES_SUCCESS: &'static str = "ingest_log.writes.success";
    pub const INGEST_LOG_WRITES_ERROR: &'static str = "ingest_log.writes.error";
    pub const INGEST_LOG_CURRENT_FILE_SIZE: &'static str = "ingest_log.current_file.size";
}
```

### Phase 2: Runtime Metric Recording

```rust
// src/metrics/recorder.rs

use crate::metrics::catalog::{METRICS, MetricDefinition, MetricType};
use std::collections::HashMap;

pub struct MetricRecorder;

impl MetricRecorder {
    /// Initialize all metrics at startup
    pub fn init() {
        for (key, def) in METRICS.iter() {
            match def.metric_type {
                MetricType::Counter => {
                    metrics::describe_counter!(def.name, def.help);
                }
                MetricType::Gauge => {
                    metrics::describe_gauge!(def.name, def.help);
                }
                MetricType::Histogram { buckets } => {
                    metrics::describe_histogram!(def.name, def.help);
                    // Note: buckets would need to be configured via the exporter
                }
            }
        }
    }
    
    /// Record a metric by its catalog key
    pub fn record(key: &str, value: f64, labels: &[(&str, &str)]) {
        if let Some(def) = METRICS.get(key) {
            let label_pairs: Vec<_> = labels.iter()
                .map(|(k, v)| (*k, v.to_string()))
                .collect();
                
            match def.metric_type {
                MetricType::Counter => {
                    metrics::counter!(def.name, &label_pairs).increment(value as u64);
                }
                MetricType::Gauge => {
                    metrics::gauge!(def.name, &label_pairs).set(value);
                }
                MetricType::Histogram { .. } => {
                    metrics::histogram!(def.name, &label_pairs).record(value);
                }
            }
        } else {
            tracing::warn!("Unknown metric key: {}", key);
        }
    }
    
    /// Convenience method for incrementing counters
    pub fn increment(key: &str, labels: &[(&str, &str)]) {
        Self::record(key, 1.0, labels);
    }
}

// Usage in application code
use crate::metrics::catalog::Metrics;
use crate::metrics::recorder::MetricRecorder;

fn handle_source_request(source_id: &str) {
    let start = std::time::Instant::now();
    
    match fetch_from_source(source_id) {
        Ok(data) => {
            MetricRecorder::increment(
                Metrics::SOURCES_REQUESTS_SUCCESS,
                &[("source_id", source_id)]
            );
        }
        Err(e) => {
            MetricRecorder::increment(
                Metrics::SOURCES_REQUESTS_ERROR,
                &[("source_id", source_id), ("error_type", e.type_str())]
            );
        }
    }
    
    MetricRecorder::record(
        Metrics::SOURCES_REQUEST_DURATION,
        start.elapsed().as_secs_f64(),
        &[("source_id", source_id)]
    );
}
```

### Phase 3: Dashboard Generation

```rust
// src/metrics/dashboard.rs

use crate::metrics::catalog::{METRICS, PanelType, MetricType};
use serde_json::{json, Value};

pub struct DashboardBuilder {
    pub title: String,
    pub panels: Vec<Value>,
}

impl DashboardBuilder {
    pub fn from_catalog(title: &str) -> Self {
        let mut builder = Self {
            title: title.to_string(),
            panels: Vec::new(),
        };
        
        // Group metrics by phase
        let mut phases: HashMap<&str, Vec<&MetricDefinition>> = HashMap::new();
        
        for def in METRICS.values() {
            let phase = def.name.split('_').nth(1).unwrap_or("unknown");
            phases.entry(phase).or_default().push(def);
        }
        
        // Create panels for each phase
        for (phase, metrics) in phases {
            builder.add_row(&format!("📊 {} Phase", phase));
            
            for metric in metrics {
                match metric.dashboard.panel_type {
                    PanelType::TimeSeries => {
                        let queries = metric.dashboard.queries.iter()
                            .map(|q| json!({
                                "expr": q.expr,
                                "legendFormat": q.name,
                            }))
                            .collect();
                            
                        builder.add_timeseries_panel(
                            &metric.help,
                            queries,
                        );
                    }
                    PanelType::Stat => {
                        builder.add_stat_panel(
                            &metric.help,
                            metric.name,
                        );
                    }
                    _ => {}
                }
            }
        }
        
        builder
    }
    
    pub fn to_json(&self) -> Value {
        json!({
            "dashboard": {
                "title": self.title,
                "panels": self.panels,
                "schemaVersion": 30,
                // ... other dashboard config
            }
        })
    }
}

// Binary to generate dashboard
fn main() {
    let dashboard = DashboardBuilder::from_catalog("SMS Pipeline Dashboard");
    let json = dashboard.to_json();
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
```

## Benefits

### 1. Single Source of Truth
- Metrics defined once, used everywhere
- No duplication or drift between code and dashboards
- Central place to update metric definitions

### 2. Type Safety
- Compile-time verification of metric keys
- IDE autocomplete for metric names
- Impossible to reference non-existent metrics

### 3. Consistency
- Guaranteed alignment between:
  - What we measure (runtime)
  - What we visualize (dashboards)
  - What we document (help text)

### 4. Maintainability
- Add a new metric in one place
- Dashboard automatically includes it
- Documentation stays in sync

### 5. Discoverability
- All metrics visible in one file
- Easy to see what's available
- Clear relationships between metrics

## Migration Path

1. **Phase 1**: Create metric catalog with existing metrics
2. **Phase 2**: Update runtime code to use catalog keys
3. **Phase 3**: Generate dashboards from catalog
4. **Phase 4**: Remove hardcoded metric strings
5. **Phase 5**: Add validation tests

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn all_metrics_have_unique_names() {
        let mut names = HashSet::new();
        for def in METRICS.values() {
            assert!(names.insert(def.name), "Duplicate metric: {}", def.name);
        }
    }
    
    #[test]
    fn all_metrics_have_dashboard_config() {
        for (key, def) in METRICS.iter() {
            assert!(!def.dashboard.queries.is_empty(), 
                "Metric {} has no dashboard queries", key);
        }
    }
    
    #[test]
    fn metric_keys_match_definitions() {
        assert!(METRICS.contains_key(Metrics::SOURCES_REQUESTS_SUCCESS));
        assert!(METRICS.contains_key(Metrics::GATEWAY_ENVELOPES_ACCEPTED));
        // ... etc
    }
}
```

## Future Enhancements

1. **Metric Validation**
   - Enforce naming conventions
   - Validate label cardinality
   - Check for unused metrics

2. **Alert Generation**
   - Define alert thresholds in catalog
   - Generate Prometheus alert rules
   - Include runbook links

3. **Documentation Generation**
   - Auto-generate metrics documentation
   - Create metric relationship diagrams
   - Export OpenMetrics format

4. **Dynamic Registration**
   - Allow runtime metric discovery
   - Support plugin metrics
   - Enable feature-flagged metrics

## Conclusion

This architecture provides a robust, type-safe, and maintainable approach to metrics management. By treating the metric catalog as the single source of truth, we ensure consistency across the entire observability stack while reducing maintenance burden and preventing drift.

The key insight is that metrics are not just runtime concerns—they're a cross-cutting aspect that touches code, monitoring, visualization, and documentation. By centralizing their definition, we can maintain consistency across all these domains.
