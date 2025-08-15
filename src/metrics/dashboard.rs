//! Dashboard generation module for creating Grafana dashboards from metrics
//! 
//! This module provides functionality to automatically generate Grafana dashboard
//! JSON from the metrics defined in our system.

use serde_json::{json, Value};
use std::collections::HashMap;

/// Represents a metric type for dashboard panel generation
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Histogram,
    Gauge,
}

/// Represents a metric definition
#[derive(Debug, Clone)]
pub struct MetricDef {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: Option<String>,
    pub phase: String,
}

/// Dashboard builder for generating Grafana dashboards
pub struct DashboardBuilder {
    title: String,
    metrics: Vec<MetricDef>,
    datasource: String,
}

impl DashboardBuilder {
    /// Create a new dashboard builder
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            metrics: Vec::new(),
            datasource: "Prometheus".to_string(),
        }
    }

    /// Set the datasource name
    pub fn with_datasource(mut self, datasource: impl Into<String>) -> Self {
        self.datasource = datasource.into();
        self
    }

    /// Add a metric definition
    pub fn add_metric(mut self, metric: MetricDef) -> Self {
        self.metrics.push(metric);
        self
    }

    /// Create from our metrics catalog
    pub fn from_catalog() -> Self {
        let mut builder = Self::new("SMS Scraper Metrics Dashboard");
        
        // Sources metrics
        builder = builder
            .add_metric(MetricDef {
                name: "sms_sources_requests_success_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Total successful source requests".to_string(),
                unit: None,
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_sources_requests_error_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Total failed source requests".to_string(),
                unit: None,
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_sources_request_duration_seconds".to_string(),
                metric_type: MetricType::Histogram,
                description: "Request duration in seconds".to_string(),
                unit: Some("s".to_string()),
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_sources_payload_bytes".to_string(),
                metric_type: MetricType::Histogram,
                description: "Payload size in bytes".to_string(),
                unit: Some("bytes".to_string()),
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_sources_registry_loads_success_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Successful registry loads".to_string(),
                unit: None,
                phase: "sources".to_string(),
            });

        // Gateway metrics
        builder = builder
            .add_metric(MetricDef {
                name: "sms_gateway_envelopes_accepted_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Total envelopes accepted".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_gateway_envelopes_deduplicated_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Total envelopes deduplicated".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_gateway_cas_writes_success_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Successful CAS writes".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_gateway_processing_duration_seconds".to_string(),
                metric_type: MetricType::Histogram,
                description: "Gateway processing duration".to_string(),
                unit: Some("s".to_string()),
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_gateway_bytes_ingested".to_string(),
                metric_type: MetricType::Histogram,
                description: "Bytes ingested per source".to_string(),
                unit: Some("bytes".to_string()),
                phase: "gateway".to_string(),
            });

        // Ingest log metrics
        builder = builder
            .add_metric(MetricDef {
                name: "sms_ingest_log_writes_success_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Successful log writes".to_string(),
                unit: None,
                phase: "ingest_log".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_ingest_log_write_bytes".to_string(),
                metric_type: MetricType::Histogram,
                description: "Log write size".to_string(),
                unit: Some("bytes".to_string()),
                phase: "ingest_log".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_ingest_log_current_file_bytes".to_string(),
                metric_type: MetricType::Gauge,
                description: "Current log file size".to_string(),
                unit: Some("bytes".to_string()),
                phase: "ingest_log".to_string(),
            });

        // Parser metrics
        builder = builder
            .add_metric(MetricDef {
                name: "sms_parser_parse_success_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Successful parses".to_string(),
                unit: None,
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_parser_parse_error_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Parse errors".to_string(),
                unit: None,
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_parser_duration_seconds".to_string(),
                metric_type: MetricType::Histogram,
                description: "Parse duration".to_string(),
                unit: Some("s".to_string()),
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_parser_records_extracted_total".to_string(),
                metric_type: MetricType::Counter,
                description: "Records extracted".to_string(),
                unit: None,
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: "sms_parser_batch_size".to_string(),
                metric_type: MetricType::Histogram,
                description: "Parse batch size".to_string(),
                unit: None,
                phase: "parser".to_string(),
            });

        builder
    }

    /// Generate a Grafana panel for a counter metric
    fn generate_counter_panel(&self, metric: &MetricDef, panel_id: u32, x: u32, y: u32) -> Value {
        json!({
            "id": panel_id,
            "gridPos": {
                "x": x,
                "y": y,
                "w": 12,
                "h": 8
            },
            "type": "graph",
            "title": metric.description.clone(),
            "datasource": self.datasource.clone(),
            "targets": [
                {
                    "expr": format!("rate({}[5m])", metric.name),
                    "legendFormat": "{{instance}}",
                    "refId": "A"
                }
            ],
            "fieldConfig": {
                "defaults": {
                    "unit": "ops",
                    "custom": {
                        "drawStyle": "line",
                        "lineInterpolation": "linear",
                        "lineWidth": 1,
                        "fillOpacity": 10,
                        "spanNulls": true
                    }
                }
            },
            "options": {
                "legend": {
                    "calcs": ["mean", "lastNotNull"],
                    "displayMode": "table",
                    "placement": "bottom"
                }
            }
        })
    }

    /// Generate a Grafana panel for a histogram metric
    fn generate_histogram_panel(&self, metric: &MetricDef, panel_id: u32, x: u32, y: u32) -> Value {
        let unit = metric.unit.as_deref().unwrap_or("short");
        
        json!({
            "id": panel_id,
            "gridPos": {
                "x": x,
                "y": y,
                "w": 12,
                "h": 8
            },
            "type": "graph",
            "title": metric.description.clone(),
            "datasource": self.datasource.clone(),
            "targets": [
                {
                    "expr": format!("histogram_quantile(0.5, sum(rate({}_bucket[5m])) by (le))", metric.name),
                    "legendFormat": "p50",
                    "refId": "A"
                },
                {
                    "expr": format!("histogram_quantile(0.95, sum(rate({}_bucket[5m])) by (le))", metric.name),
                    "legendFormat": "p95",
                    "refId": "B"
                },
                {
                    "expr": format!("histogram_quantile(0.99, sum(rate({}_bucket[5m])) by (le))", metric.name),
                    "legendFormat": "p99",
                    "refId": "C"
                }
            ],
            "fieldConfig": {
                "defaults": {
                    "unit": unit,
                    "custom": {
                        "drawStyle": "line",
                        "lineInterpolation": "linear",
                        "lineWidth": 1,
                        "fillOpacity": 10,
                        "spanNulls": true
                    }
                }
            },
            "options": {
                "legend": {
                    "calcs": ["mean", "lastNotNull"],
                    "displayMode": "table",
                    "placement": "bottom"
                }
            }
        })
    }

    /// Generate a Grafana panel for a gauge metric
    fn generate_gauge_panel(&self, metric: &MetricDef, panel_id: u32, x: u32, y: u32) -> Value {
        let unit = metric.unit.as_deref().unwrap_or("short");
        
        json!({
            "id": panel_id,
            "gridPos": {
                "x": x,
                "y": y,
                "w": 12,
                "h": 8
            },
            "type": "stat",
            "title": metric.description.clone(),
            "datasource": self.datasource.clone(),
            "targets": [
                {
                    "expr": metric.name.clone(),
                    "legendFormat": "{{instance}}",
                    "refId": "A"
                }
            ],
            "fieldConfig": {
                "defaults": {
                    "unit": unit,
                    "thresholds": {
                        "mode": "absolute",
                        "steps": [
                            {
                                "color": "green",
                                "value": null
                            }
                        ]
                    }
                }
            },
            "options": {
                "orientation": "auto",
                "textMode": "auto",
                "colorMode": "background",
                "graphMode": "area",
                "justifyMode": "auto"
            }
        })
    }

    /// Build the complete Grafana dashboard JSON
    pub fn build(&self) -> Value {
        let mut panels = Vec::new();
        let mut panel_id = 1;
        
        // Group metrics by phase
        let mut phases: HashMap<String, Vec<&MetricDef>> = HashMap::new();
        for metric in &self.metrics {
            phases.entry(metric.phase.clone()).or_insert_with(Vec::new).push(metric);
        }
        
        // Sort phases for consistent ordering
        let mut phase_names: Vec<String> = phases.keys().cloned().collect();
        phase_names.sort();
        
        let mut current_y = 0;
        
        // Generate panels for each phase
        for phase in phase_names {
            // Add a row panel for the phase
            panels.push(json!({
                "id": panel_id,
                "type": "row",
                "title": format!("{} Metrics", phase.to_uppercase()),
                "gridPos": {
                    "x": 0,
                    "y": current_y,
                    "w": 24,
                    "h": 1
                },
                "collapsed": false
            }));
            panel_id += 1;
            current_y += 1;
            
            // Add metrics panels for this phase
            let phase_metrics = phases.get(&phase).unwrap();
            let mut x_offset = 0;
            
            for metric in phase_metrics {
                let panel = match metric.metric_type {
                    MetricType::Counter => self.generate_counter_panel(metric, panel_id, x_offset, current_y),
                    MetricType::Histogram => self.generate_histogram_panel(metric, panel_id, x_offset, current_y),
                    MetricType::Gauge => self.generate_gauge_panel(metric, panel_id, x_offset, current_y),
                };
                panels.push(panel);
                panel_id += 1;
                
                // Move to next column or next row
                x_offset += 12;
                if x_offset >= 24 {
                    x_offset = 0;
                    current_y += 8;
                }
            }
            
            // Move to next row for next phase
            if x_offset > 0 {
                current_y += 8;
            }
        }
        
        // Build the complete dashboard
        json!({
            "dashboard": {
                "id": null,
                "uid": null,
                "title": self.title.clone(),
                "tags": ["sms-scraper", "generated"],
                "timezone": "browser",
                "schemaVersion": 27,
                "version": 0,
                "refresh": "10s",
                "time": {
                    "from": "now-1h",
                    "to": "now"
                },
                "timepicker": {
                    "refresh_intervals": ["5s", "10s", "30s", "1m", "5m", "15m", "30m", "1h", "2h", "1d"]
                },
                "templating": {
                    "list": []
                },
                "annotations": {
                    "list": [
                        {
                            "builtIn": 1,
                            "datasource": "-- Grafana --",
                            "enable": true,
                            "hide": true,
                            "iconColor": "rgba(0, 211, 255, 1)",
                            "name": "Annotations & Alerts",
                            "type": "dashboard"
                        }
                    ]
                },
                "panels": panels
            },
            "overwrite": true
        })
    }
}
