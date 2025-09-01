use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, error, debug};
use sms_core::storage::Storage;
use sms_core::domain::RawData;
use sms_core::common::types::{RawDataInfo, EventArgs};
use crate::registry::source_loader::SourceRegistry;
use super::{PipelineStep, StepResult};

/// Pipeline step for parsing raw data into structured events
pub struct ParseStep {
    source_registry: SourceRegistry,
}

impl ParseStep {
    pub fn new(source_registry: SourceRegistry) -> Self {
        Self { source_registry }
    }
    
    /// Parse raw data from a single RawData record
    async fn parse_raw_data(&self, raw_data: &RawData) -> Result<Vec<ParsedEventData>> {
        let mut parsed_data_list = Vec::new();
        
        // Map internal API names back to parser names
        let api_name = match raw_data.api_name.as_str() {
            "crawler_neumos" => "neumos",
            "crawler_barboza" => "barboza", 
            "crawler_blue_moon" => "blue_moon",
            "crawler_conor_byrne" => "conor_byrne",
            "crawler_crocodile_cafe" => "crocodile_cafe",
            "crawler_neptune" => "neptune",
            "crawler_showbox" => "showbox",
            "crawler_tractor_tavern" => "tractor_tavern",
            other => {
                error!("Unknown API name for parsing: {}", other);
                return Err(anyhow::anyhow!("Unknown API name: {}", other));
            }
        };
        
        let parser = crate::apis::factory::create_parser(api_name)
            .ok_or_else(|| anyhow::anyhow!("Failed to create parser for API: {}", api_name))?;
        
        // Handle both pre-parsed JSON objects and raw JSON strings
        if raw_data.data.is_object() || raw_data.data.is_array() {
            // Data is already parsed JSON - extract events directly
            let events = if let Some(array) = raw_data.data.as_array() {
                array.clone()
            } else {
                vec![raw_data.data.clone()]
            };
            
            for event_json in events {
                let raw_data_info = parser.extract_raw_data_info(&event_json)?;
                let event_args = parser.extract_event_args(&event_json)?;
                parsed_data_list.push(ParsedEventData {
                    raw_data_info,
                    event_args,
                    source_api: raw_data.api_name.clone(),
                });
            }
        } else {
            // Data is a JSON string - parse it first
            let json_string;
            let bytes_slice = if let Some(bytes_str) = raw_data.data.as_str() {
                bytes_str.as_bytes()
            } else {
                // Convert JSON value to string
                json_string = serde_json::to_string(&raw_data.data)?;
                json_string.as_bytes()
            };
            
            let parsed_events = parser.parse_events(bytes_slice).await?;
            
            for event_json in parsed_events {
                let raw_data_info = parser.extract_raw_data_info(&event_json)?;
                let event_args = parser.extract_event_args(&event_json)?;
                parsed_data_list.push(ParsedEventData {
                    raw_data_info,
                    event_args,
                    source_api: raw_data.api_name.clone(),
                });
            }
        }
        
        Ok(parsed_data_list)
    }
}

#[async_trait]
impl PipelineStep for ParseStep {
    async fn execute(&self, source_id: &str, storage: &dyn Storage) -> Result<StepResult> {
        info!("📄 Running parse step for source: {}", source_id);
        
        // Map user-friendly source names to internal API names for database lookup
        let internal_api_name = crate::common::constants::api_name_to_internal(source_id);
        
        // Get all unprocessed raw data for this source
        let raw_data_items = storage.get_unprocessed_raw_data(&internal_api_name, None).await?;
        
        if raw_data_items.is_empty() {
            let message = format!("No unprocessed raw data found for source: {}", source_id);
            info!("{}", message);
            return Ok(StepResult::success(0, message));
        }
        
        info!("📊 Found {} unprocessed raw data items for {}", raw_data_items.len(), source_id);
        
        let mut total_parsed = 0;
        let total_failed = 0;
        let mut processing_errors = 0;
        
        // Process each raw data item
        for raw_data in &raw_data_items {
            match self.parse_raw_data(raw_data).await {
                Ok(parsed_events) => {
                    debug!("✅ Parsed {} events from raw data ID: {}", parsed_events.len(), raw_data.event_api_id);
                    
                    // Store parsed events (this would typically go to a parsed_events table)
                    // For now, we'll mark the raw data as processed
                    total_parsed += parsed_events.len();
                }
                Err(e) => {
                    error!("❌ Failed to parse raw data ID {}: {}", raw_data.event_api_id, e);
                    processing_errors += 1;
                }
            }
        }
        
        // Mark all raw data as processed
        for mut raw_data in raw_data_items {
            raw_data.processed = true;
            // Note: Storage trait doesn't have update_raw_data method yet
            // This would need to be implemented in the storage layer
            // For now, we'll skip this step
            debug!("Would mark raw data as processed: {}", raw_data.event_api_id);
        }
        
        let message = format!(
            "Parse completed for {}: {} events parsed, {} failed, {} processing errors",
            source_id, total_parsed, total_failed, processing_errors
        );
        
        info!("✅ {}", message);
        Ok(StepResult::with_errors(total_parsed, total_failed, processing_errors, message))
    }
    
    fn step_name(&self) -> &'static str {
        "parse"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["ingestion"]
    }
}

/// Parsed event data structure
#[derive(Debug, Clone)]
pub struct ParsedEventData {
    pub raw_data_info: RawDataInfo,
    pub event_args: EventArgs,
    pub source_api: String,
}
