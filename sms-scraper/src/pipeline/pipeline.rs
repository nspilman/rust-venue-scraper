use sms_core::domain::RawData;
use sms_core::common::error::Result;
use crate::pipeline::storage::Storage;
use sms_core::common::types::{EventApi, EventArgs, RawDataInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

/// Processed event ready for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedEvent {
    pub raw_data_info: RawDataInfo,
    pub event_args: EventArgs,
    pub processed_at: DateTime<Utc>,
    pub api_name: String,
}

/// Result of a complete pipeline run
#[derive(Debug, Serialize)]
pub struct PipelineResult {
    pub api_name: String,
    pub total_events: usize,
    pub processed_events: usize,
    pub skipped_events: usize,
    pub errors: Vec<String>,
    pub output_file: String,
}

pub struct Pipeline;

impl Pipeline {
    // Removed unused push_pushgateway_metrics function
    /// Process a single raw event into a ProcessedEvent
    #[instrument(skip(api, raw_event), fields(api_name = %api.api_name()))]
    fn process_event(
        api: &dyn EventApi,
        raw_event: &serde_json::Value,
    ) -> Result<Option<ProcessedEvent>> {
        // Check if event should be skipped
        let (should_skip, skip_reason) = api.should_skip(raw_event);
        if should_skip {
            debug!("Skipping event: {}", skip_reason);
            println!("   Skipping event: {skip_reason}");
            return Ok(None);
        }

        // Extract structured data
        let raw_data_info = api.get_raw_data_info(raw_event)?;
        let event_args = api.get_event_args(raw_event)?;

        debug!("Successfully processed event: {}", event_args.title);

        Ok(Some(ProcessedEvent {
            raw_data_info,
            event_args,
            processed_at: Utc::now(),
            api_name: api.api_name().to_string(),
        }))
    }

    /// Run minimal ingestion for a given API - only fetch and store raw bytes
    #[instrument(skip(api, storage), fields(api_name = %api.api_name()))]
    pub async fn run_ingestion_only(
        api: Box<dyn EventApi>,
        output_dir: &str,
        storage: Arc<dyn Storage>,
    ) -> Result<PipelineResult> {
        let api_name = api.api_name().to_string();
        info!("📥 Starting minimal ingestion for {}", api_name);
        println!("📥 Starting minimal ingestion for {}", api_name);
        // metrics: count pipeline runs
        crate::observability::metrics::sources::registry_load_success();
        let t_pipeline = std::time::Instant::now();

        // Step 1: Fetch raw events (raw JSON/HTML bytes only)
        info!("📡 Fetching raw bytes from {}...", api_name);
        println!("📡 Fetching raw bytes from {}...", api_name);
        let t_fetch = std::time::Instant::now();
        let raw_events = api.get_event_list().await?;
        let fetch_secs = t_fetch.elapsed().as_secs_f64();
        crate::observability::metrics::sources::request_duration(fetch_secs);
        info!("✅ Fetched {} raw event records", raw_events.len());
        println!("✅ Fetched {} raw event records", raw_events.len());
        // Record payload size through Sources metrics
        crate::observability::metrics::sources::request_success();

        // Step 2: Store raw bytes only (no processing)
        info!("💾 Storing raw bytes to database...");
        println!("💾 Storing raw bytes to database...");
        let mut stored_count = 0;
        let mut errors = Vec::new();

        for (i, raw_event) in raw_events.iter().enumerate() {
            // Store raw JSON bytes directly without any processing
            let mut raw_data = RawData {
                id: None,
                api_name: sms_core::common::constants::api_name_to_internal(&api_name),
                event_api_id: format!("{}-raw-{}", api_name, i), // Temporary ID for raw data
                event_name: "Raw Event Data".to_string(), // Will be parsed later
                venue_name: api_name.clone(), // Will be parsed later
                event_day: chrono::Utc::now().date_naive(), // Will be parsed later
                data: raw_event.clone(), // Store the raw JSON as-is
                processed: false, // Mark as unprocessed for full pipeline
                event_id: None,
                created_at: chrono::Utc::now(),
            };
            
            match storage.create_raw_data(&mut raw_data).await {
                Ok(_) => {
                    stored_count += 1;
                    if (i + 1) % 10 == 0 {
                        debug!("Stored {}/{} raw records", i + 1, raw_events.len());
                        println!("   Stored {}/{} raw records", i + 1, raw_events.len());
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to store raw event {i}: {e}");
                    error!("Storage failed for event {}: {}", i, e);
                    errors.push(error_msg);
                }
            }
        }

        info!("✅ Stored {} raw records ({} errors)", stored_count, errors.len());
        println!("✅ Stored {} raw records ({} errors)", stored_count, errors.len());
        
        // Step 3: Persist to JSON (legacy) - store raw events
        let output_file = Self::persist_raw_to_json(&raw_events, &api_name, output_dir)?;
        info!("💾 Saved raw events to {}", output_file);
        println!("💾 Saved raw events to {}", output_file);

        // metrics: total pipeline duration
        let total_secs = t_pipeline.elapsed().as_secs_f64();
        crate::observability::metrics::sources::request_duration(total_secs);

        // Ensure snapshot is non-empty at push time
        crate::observability::heartbeat();

        Ok(PipelineResult {
            api_name,
            total_events: raw_events.len(),
            processed_events: 0, // No processing done in ingestion
            skipped_events: 0,
            errors,
            output_file,
        })
    }

    /// Persist raw events to JSON file
    fn persist_raw_to_json(
        events: &[serde_json::Value],
        api_name: &str,
        output_dir: &str,
    ) -> Result<String> {
        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        // Generate filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{api_name}_raw_{timestamp}.json");
        let filepath = Path::new(output_dir).join(&filename);

        // Serialize and write raw events
        let json_content = serde_json::to_string_pretty(events)?;
        fs::write(&filepath, json_content)?;

        Ok(filepath.to_string_lossy().to_string())
    }

    /// Persist processed events to JSON file (legacy method)
    fn persist_to_json(
        events: &[ProcessedEvent],
        api_name: &str,
        output_dir: &str,
    ) -> Result<String> {
        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        // Generate filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{api_name}_{timestamp}.json");
        let filepath = Path::new(output_dir).join(&filename);

        // Serialize and write
        let json_content = serde_json::to_string_pretty(events)?;
        fs::write(&filepath, json_content)?;

        Ok(filepath.to_string_lossy().to_string())
    }
}
