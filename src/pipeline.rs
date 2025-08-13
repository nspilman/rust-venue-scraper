use crate::carpenter::RawData;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::{EventApi, EventArgs, RawDataInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use metrics::{counter, histogram};

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
    async fn push_pushgateway_metrics(api: &str, processed: usize, skipped: usize, errors: usize, duration_secs: f64) {
        let base = match std::env::var("SMS_PUSHGATEWAY_URL") {
            Ok(v) if !v.trim().is_empty() => v,
            _ => return,
        };
        let instance = api; // use api name as instance label for clarity
        let push_url = format!("{}/metrics/job/{}/instance/{}", base.trim_end_matches('/'), "sms_scraper", instance);
        
        // Get current timestamp for freshness tracking
        let timestamp_secs = chrono::Utc::now().timestamp() as f64;
        
        let body = format!(
            "# TYPE sms_ingest_runs_total counter\n\
             sms_ingest_runs_total 1\n\
             # TYPE sms_events_processed_total counter\n\
             sms_events_processed_total {}\n\
             # TYPE sms_events_skipped_total counter\n\
             sms_events_skipped_total {}\n\
             # TYPE sms_pipeline_errors_total counter\n\
             sms_pipeline_errors_total {}\n\
             # TYPE sms_pipeline_duration_seconds gauge\n\
             sms_pipeline_duration_seconds {}\n\
             # TYPE sms_pipeline_last_run_timestamp_seconds gauge\n\
             sms_pipeline_last_run_timestamp_seconds {}\n",
            processed, skipped, errors, duration_secs, timestamp_secs
        );
        
        let client = reqwest::Client::new();
        
        // Step 1: Push metrics to Pushgateway
        let push_res = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(body)
            .send()
            .await;
            
        match push_res {
            Ok(r) if r.status().is_success() => {
                tracing::info!("Pushed metrics to Pushgateway for api={}", api);
            }
            Ok(r) => {
                tracing::warn!("Pushgateway push responded with status {} for api={}", r.status().as_u16(), api);
            }
            Err(e) => {
                tracing::warn!("Failed to push metrics to Pushgateway for api={}: {}", api, e);
            }
        }
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
        counter!("sms_pipeline_runs_total", "api" => api_name.clone()).increment(1);
        let t_pipeline = std::time::Instant::now();

        // Step 1: Fetch raw events
        info!("📡 Fetching events from {}...", api_name);
        println!("📡 Fetching events from {}...", api_name);
        let t_fetch = std::time::Instant::now();
        let raw_events = api.get_event_list().await?;
        let fetch_secs = t_fetch.elapsed().as_secs_f64();
        histogram!("sms_fetch_events_duration_seconds", "api" => api_name.clone()).record(fetch_secs);
        info!("✅ Fetched {} raw events", raw_events.len());
        println!("✅ Fetched {} raw events", raw_events.len());
        histogram!("sms_raw_events_per_run", "api" => api_name.clone()).record(raw_events.len() as f64);

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
        // metrics: counts
        counter!("sms_events_processed_total", "api" => api_name.clone()).increment(processed_events.len() as u64);
        counter!("sms_events_skipped_total", "api" => api_name.clone()).increment(skipped as u64);
        counter!("sms_event_errors_total", "api" => api_name.clone()).increment(errors.len() as u64);

        // Step 3: Save raw data to storage
        info!("💾 Saving raw data to storage...");
        for processed_event in &processed_events {
            let mut raw_data = RawData::from_processed_event(processed_event);
            // Map API names to the format expected by carpenter
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
        histogram!("sms_pipeline_duration_seconds", "api" => api_name.clone()).record(total_secs);

        // Push a minimal metrics snapshot to Pushgateway if configured
        Self::push_pushgateway_metrics(&api_name, processed_events.len(), skipped, errors.len(), total_secs).await;

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
