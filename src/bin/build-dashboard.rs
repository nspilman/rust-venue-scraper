//! Dashboard Builder Binary
//! 
//! Generates a Grafana dashboard JSON file from our metrics catalog.
//! This ensures the dashboard stays in sync with the metrics we actually collect.

use sms_scraper::metrics::dashboard::DashboardBuilder;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Use the dashboard builder from our metrics catalog
    let builder = DashboardBuilder::from_catalog();
    let dashboard_json = builder.build();
    
    // Pretty print the JSON
    let json_string = serde_json::to_string_pretty(&dashboard_json)
        .expect("Failed to serialize dashboard");
    
    // Print to stdout by default
    println!("{}", json_string);
    
    // Also save to a file
    let output_path = PathBuf::from("grafana-dashboard.json");
    fs::write(&output_path, &json_string)
        .expect("Failed to write dashboard file");
    
    eprintln!("✅ Dashboard generated successfully!");
    eprintln!("📄 Saved to: {}", output_path.display());
    eprintln!("📊 Import this JSON file into Grafana to create the dashboard");
}
