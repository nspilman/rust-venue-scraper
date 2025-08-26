//! Dashboard generation module for creating Grafana dashboards from metrics
//! 
//! This module provides functionality to automatically generate Grafana dashboard
//! JSON from the metrics defined in our system.

use serde_json::{json, Value};
use std::collections::HashMap;
use super::MetricName;

/// Represents a metric type for dashboard panel generation
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Histogram,
    Gauge,
}

/// Represents a metric definition
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MetricDef {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: Option<String>,
    pub phase: String,
}

/// Dashboard builder for generating Grafana dashboards
#[allow(dead_code)]
pub struct DashboardBuilder {
    title: String,
    metrics: Vec<MetricDef>,
    datasource: String,
}

impl DashboardBuilder {
    /// Create a new dashboard builder
    #[allow(dead_code)]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            metrics: Vec::new(),
            datasource: "Prometheus".to_string(),
        }
    }

    /// Set the datasource name
    #[allow(dead_code)]
    pub fn with_datasource(mut self, datasource: impl Into<String>) -> Self {
        self.datasource = datasource.into();
        self
    }

    /// Add a metric definition
    #[allow(dead_code)]
    pub fn add_metric(mut self, metric: MetricDef) -> Self {
        self.metrics.push(metric);
        self
    }

    /// Create from our metrics catalog (static version)
    #[allow(dead_code)]
    pub fn from_catalog() -> Self {
        let mut builder = Self::new("SMS Scraper Metrics Dashboard");
        
        // Sources metrics
        builder = builder
            .add_metric(MetricDef {
                name: MetricName::SourcesRequestsSuccess.to_string(),
                metric_type: MetricType::Counter,
                description: "Total successful source requests".to_string(),
                unit: None,
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::SourcesRequestsError.to_string(),
                metric_type: MetricType::Counter,
                description: "Total failed source requests".to_string(),
                unit: None,
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::SourcesRequestDuration.to_string(),
                metric_type: MetricType::Histogram,
                description: "Request duration in seconds".to_string(),
                unit: Some("s".to_string()),
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::SourcesPayloadBytes.to_string(),
                metric_type: MetricType::Histogram,
                description: "Payload size in bytes".to_string(),
                unit: Some("bytes".to_string()),
                phase: "sources".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::SourcesRegistryLoadsSuccess.to_string(),
                metric_type: MetricType::Counter,
                description: "Successful registry loads".to_string(),
                unit: None,
                phase: "sources".to_string(),
            });

        // Gateway metrics
        builder = builder
            .add_metric(MetricDef {
                name: MetricName::GatewayEnvelopesAccepted.to_string(),
                metric_type: MetricType::Counter,
                description: "Total envelopes accepted".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::GatewayEnvelopesDeduplicated.to_string(),
                metric_type: MetricType::Counter,
                description: "Total envelopes deduplicated".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::GatewayCasWritesSuccess.to_string(),
                metric_type: MetricType::Counter,
                description: "Successful CAS writes".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::GatewayRecordsIngested.to_string(),
                metric_type: MetricType::Counter,
                description: "Total records ingested".to_string(),
                unit: None,
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::GatewayProcessingDuration.to_string(),
                metric_type: MetricType::Histogram,
                description: "Gateway processing duration".to_string(),
                unit: Some("s".to_string()),
                phase: "gateway".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::GatewayBytesIngested.to_string(),
                metric_type: MetricType::Histogram,
                description: "Bytes ingested per source".to_string(),
                unit: Some("bytes".to_string()),
                phase: "gateway".to_string(),
            });

        // Ingest log metrics
        builder = builder
            .add_metric(MetricDef {
                name: MetricName::IngestLogWritesSuccess.to_string(),
                metric_type: MetricType::Counter,
                description: "Successful log writes".to_string(),
                unit: None,
                phase: "ingest_log".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::IngestLogWriteBytes.to_string(),
                metric_type: MetricType::Histogram,
                description: "Log write size".to_string(),
                unit: Some("bytes".to_string()),
                phase: "ingest_log".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::IngestLogCurrentFileBytes.to_string(),
                metric_type: MetricType::Gauge,
                description: "Current log file size".to_string(),
                unit: Some("bytes".to_string()),
                phase: "ingest_log".to_string(),
            });

        // Parser metrics
        builder = builder
            .add_metric(MetricDef {
                name: MetricName::ParserParseSuccess.to_string(),
                metric_type: MetricType::Counter,
                description: "Successful parses".to_string(),
                unit: None,
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::ParserParseError.to_string(),
                metric_type: MetricType::Counter,
                description: "Parse errors".to_string(),
                unit: None,
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::ParserDuration.to_string(),
                metric_type: MetricType::Histogram,
                description: "Parse duration".to_string(),
                unit: Some("s".to_string()),
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::ParserRecordsExtracted.to_string(),
                metric_type: MetricType::Counter,
                description: "Records extracted".to_string(),
                unit: None,
                phase: "parser".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::ParserBatchSize.to_string(),
                metric_type: MetricType::Histogram,
                description: "Parse batch size".to_string(),
                unit: None,
                phase: "parser".to_string(),
            });

        // Normalize metrics
        builder = builder
            .add_metric(MetricDef {
                name: MetricName::NormalizeRecordsProcessed.to_string(),
                metric_type: MetricType::Counter,
                description: "Records processed with normalization".to_string(),
                unit: None,
                phase: "normalize".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::NormalizeConfidence.to_string(),
                metric_type: MetricType::Histogram,
                description: "Normalization confidence level".to_string(),
                unit: None,
                phase: "normalize".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::NormalizeGeocoding.to_string(),
                metric_type: MetricType::Counter,
                description: "Geocoding operations performed".to_string(),
                unit: None,
                phase: "normalize".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::NormalizeWarnings.to_string(),
                metric_type: MetricType::Counter,
                description: "Normalization warnings".to_string(),
                unit: None,
                phase: "normalize".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::NormalizeBatchesProcessed.to_string(),
                metric_type: MetricType::Counter,
                description: "Batches processed".to_string(),
                unit: None,
                phase: "normalize".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::NormalizeBatchSize.to_string(),
                metric_type: MetricType::Histogram,
                description: "Normalization batch size".to_string(),
                unit: None,
                phase: "normalize".to_string(),
            });

        // Quality Gate metrics
        builder = builder
            .add_metric(MetricDef {
                name: MetricName::QualityGateRecordsAccepted.to_string(),
                metric_type: MetricType::Counter,
                description: "Records accepted by quality gate".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::QualityGateRecordsAcceptedWithWarnings.to_string(),
                metric_type: MetricType::Counter,
                description: "Records accepted with warnings".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::QualityGateRecordsQuarantined.to_string(),
                metric_type: MetricType::Counter,
                description: "Records quarantined by quality gate".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::QualityGateQualityScore.to_string(),
                metric_type: MetricType::Histogram,
                description: "Quality score distribution".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::QualityGateIssuesDetected.to_string(),
                metric_type: MetricType::Counter,
                description: "Quality issues detected".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::QualityGateBatchesProcessed.to_string(),
                metric_type: MetricType::Counter,
                description: "Batches processed through quality gate".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            })
            .add_metric(MetricDef {
                name: MetricName::QualityGateBatchSize.to_string(),
                metric_type: MetricType::Histogram,
                description: "Quality gate batch size".to_string(),
                unit: None,
                phase: "quality_gate".to_string(),
            });

        builder
    }

    /// Create dashboard dynamically from the MetricName enum
    /// This automatically discovers all metrics and generates appropriate panels
    #[allow(dead_code)]
    pub fn from_metrics_enum() -> Self {
        let mut builder = Self::new("SMS Scraper Metrics Dashboard (Auto-Generated)");
        
        // Iterate through all metrics in the enum
        for metric in super::MetricName::all_metrics() {
            let (phase, description, unit) = metric.metadata();
            let metric_type = metric.infer_metric_type();
            
            builder = builder.add_metric(MetricDef {
                name: metric.to_string(),
                metric_type,
                description: description.to_string(),
                unit: unit.map(|u| u.to_string()),
                phase: phase.to_string(),
            });
        }
        
        builder
    }

    /// Generate a Grafana panel for a counter metric
    fn generate_counter_panel(&self, metric: &MetricDef, panel_id: u32, x: u32, y: u32) -> Value {
        // Determine if this should be a cumulative counter or rate counter
        // Cumulative counters: totals, extracted, ingested, success, error counts
        // Rate counters: requests, writes, operations that benefit from per-second rates
        let should_show_cumulative = metric.name.contains("_total") || 
                                   metric.name.contains("extracted") || 
                                   metric.name.contains("ingested") || 
                                   metric.description.to_lowercase().contains("total");
        
        if should_show_cumulative {
            // Show cumulative total value
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
                        "unit": "short",
                        "thresholds": {
                            "mode": "absolute",
                            "steps": [
                                {
                                    "color": "green",
                                    "value": null
                                },
                                {
                                    "color": "yellow",
                                    "value": 1000
                                },
                                {
                                    "color": "red",
                                    "value": 10000
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
        } else {
            // Show rate of change
            json!({
                "id": panel_id,
                "gridPos": {
                    "x": x,
                    "y": y,
                    "w": 12,
                    "h": 8
                },
                "type": "graph",
                "title": format!("{} (Rate)", metric.description),
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
    #[allow(dead_code)]
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
        
        // Build the complete dashboard JSON (for Grafana provisioning)
        json!({
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
        })
    }
}
