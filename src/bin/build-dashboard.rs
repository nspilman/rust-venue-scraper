//! Dashboard Builder Binary
//! 
//! Generates a Grafana dashboard JSON file from our metrics catalog.
//! This ensures the dashboard stays in sync with the metrics we actually collect.
//!
//! Usage:
//!   cargo run --bin build-dashboard           # Uses manual catalog (static)
//!   cargo run --bin build-dashboard dynamic   # Uses automatic enum discovery (dynamic)

use sms_scraper::observability::metrics::dashboard::DashboardBuilder;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Check if "dynamic" argument is provided
    let args: Vec<String> = env::args().collect();
    let use_dynamic = args.get(1).map(|s| s == "dynamic").unwrap_or(false);
    
    let (builder, filename) = if use_dynamic {
        eprintln!("🤖 Using DYNAMIC generation from MetricName enum");
        (DashboardBuilder::from_metrics_enum(), "grafana-dashboard-dynamic.json")
    } else {
        eprintln!("📋 Using STATIC generation from manual catalog");
        (DashboardBuilder::from_catalog(), "grafana-dashboard.json")
    };
    
    let dashboard_json = builder.build();
    
    // Pretty print the JSON
    let json_string = serde_json::to_string_pretty(&dashboard_json)
        .expect("Failed to serialize dashboard");
    
    // Print to stdout by default
    println!("{}", json_string);
    
    // Also save to a file
    let output_path = PathBuf::from(filename);
    fs::write(&output_path, &json_string)
        .expect("Failed to write dashboard file");
    
    eprintln!("✅ Dashboard generated successfully!");
    eprintln!("📄 Saved to: {}", output_path.display());
    eprintln!("📊 Import this JSON file into Grafana to create the dashboard");
    
    if use_dynamic {
        eprintln!("🔄 Dynamic generation automatically discovered all metrics from the enum!");
    }
}
