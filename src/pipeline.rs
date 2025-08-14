use crate::domain::RawData;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::{EventApi, EventArgs, RawDataInfo};
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
    async fn push_pushgateway_metrics(
        _api: &str,
        _processed: usize,
        _skipped: usize,
        _errors: usize,
        _duration_secs: f64,
    ) {
        // Deprecated: kept for compatibility; use metrics::push_all_to_pushgateway instead
        return;
    }
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

    /// Run the complete pipeline for a given API with storage integration
    #[instrument(skip(api, storage), fields(api_name = %api.api_name()))]
    pub async fn run_for_api_with_storage(
        api: Box<dyn EventApi>,
        output_dir: &str,
        storage: Arc<dyn Storage>,
    ) -> Result<PipelineResult> {
        let api_name = api.api_name().to_string();
        info!("🚀 Starting pipeline with storage for {}", api_name);
        println!("🚀 Starting pipeline for {}", api_name);
        // metrics: count pipeline runs
        crate::metrics::SourcesMetrics::record_registry_load_success("");
        let t_pipeline = std::time::Instant::now();

        // Step 1: Fetch raw events
        info!("📡 Fetching events from {}...", api_name);
        println!("📡 Fetching events from {}...", api_name);
        let t_fetch = std::time::Instant::now();
        let raw_events = api.get_event_list().await?;
        let fetch_secs = t_fetch.elapsed().as_secs_f64();
        crate::metrics::SourcesMetrics::record_request_duration("", fetch_secs);
        info!("✅ Fetched {} raw events", raw_events.len());
        println!("✅ Fetched {} raw events", raw_events.len());
        // Record payload size through Sources metrics
        crate::metrics::SourcesMetrics::record_request_success("", 0.0, raw_events.len());

        // Step 2: Process events
        info!("🔧 Processing events...");
        println!("🔧 Processing events...");
        let mut processed_events = Vec::new();
        let mut errors = Vec::new();
        let mut skipped = 0;

        for (i, raw_event) in raw_events.iter().enumerate() {
            match Self::process_event(&*api, raw_event) {
                Ok(Some(processed)) => {
                    processed_events.push(processed);
                    if (i + 1) % 10 == 0 {
                        debug!("Processed {}/{} events", i + 1, raw_events.len());
                        println!("   Processed {}/{} events", i + 1, raw_events.len());
                    }
                }
                Ok(None) => {
                    skipped += 1;
                }
                Err(e) => {
                    let error_msg = format!("Failed to process event {i}: {e}");
                    error!("Processing failed for event {}: {}", i, e);
                    errors.push(error_msg);
                }
            }
        }

        info!(
            "✅ Processed {} events ({} skipped, {} errors)",
            processed_events.len(),
            skipped,
            errors.len()
        );
        println!(
            "✅ Processed {} events ({} skipped, {} errors)",
            processed_events.len(),
            skipped,
            errors.len()
        );
        // metrics: counts using new phase-based system
        crate::metrics::ParserMetrics::record_batch_run_success(processed_events.len(), 0.0);
        crate::metrics::ParserMetrics::record_envelopes_skipped(skipped);
        if errors.len() > 0 {
            crate::metrics::ParserMetrics::record_parse_error("", "", "processing_error");
        }

        // Step 3: Save raw data to storage
        info!("💾 Saving raw data to storage...");
        for processed_event in &processed_events {
            let mut raw_data = RawData::from_processed_event(processed_event);
            // Map API names to the internal storage format
            raw_data.api_name = crate::constants::api_name_to_internal(&raw_data.api_name);
            if let Err(e) = storage.create_raw_data(&mut raw_data).await {
                warn!("Failed to save raw data to storage: {}", e);
            }
        }

        // Step 4: Persist to JSON (legacy)
        let output_file = Self::persist_to_json(&processed_events, &api_name, output_dir)?;
        info!("💾 Saved events to {}", output_file);
        println!("💾 Saved events to {}", output_file);

        // metrics: total pipeline duration
        let total_secs = t_pipeline.elapsed().as_secs_f64();
        crate::metrics::ParserMetrics::record_batch_run_success(0, total_secs);

        // Ensure snapshot is non-empty at push time
        crate::metrics::bump_run_heartbeat();
        // Push full exporter snapshot to Pushgateway (if configured)
        crate::metrics::push_all_to_pushgateway(&api_name).await;

        Ok(PipelineResult {
            api_name,
            total_events: raw_events.len(),
            processed_events: processed_events.len(),
            skipped_events: skipped,
            errors,
            output_file,
        })
    }

    /// Persist processed events to JSON file
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
