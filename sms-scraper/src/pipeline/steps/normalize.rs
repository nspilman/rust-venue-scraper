use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, error};
use sms_core::storage::Storage;
use super::{PipelineStep, StepResult};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::registry::UnifiedSourceRegistry;

/// Pipeline step for normalizing parsed events into consistent format
pub struct NormalizeStep {
    registry: UnifiedSourceRegistry,
}

impl NormalizeStep {
    pub fn new() -> Result<Self> {
        let registry = UnifiedSourceRegistry::new("registry/sources")?;
        Ok(Self { registry })
    }
}

#[async_trait]
impl PipelineStep for NormalizeStep {
    async fn execute(&self, source_id: &str, storage: &dyn Storage) -> Result<StepResult> {
        info!("🔧 Running normalize step for source: {}", source_id);
        
        // 1. Get all processed raw data for this source
        let internal_api_name = crate::common::constants::api_name_to_internal(source_id);
        let processed_raw_data = storage.get_processed_raw_data(&internal_api_name, None).await
            .map_err(|e| anyhow::anyhow!("Failed to get processed raw data for source {}: {}", source_id, e))?;
        
        debug!("Found {} processed raw data items for normalization", processed_raw_data.len());
        
        // 2. Get the appropriate normalizer from unified registry
        let normalizer = self.registry.get_normalizer_for_source(source_id)
            .map_err(|e| anyhow::anyhow!("Failed to get normalizer for source {}: {}", source_id, e))?;
        
        let mut normalized_count = 0;
        let mut errors = 0;
        
        // 3. Process each raw data item through normalization
        for raw_data in processed_raw_data {
            // Convert raw data to ParsedRecord format expected by normalizer
            let parsed_record = ParsedRecord {
                source_id: raw_data.api_name.clone(),
                envelope_id: raw_data.id.map(|id| id.to_string()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                payload_ref: if raw_data.event_api_id.is_empty() { "unknown".to_string() } else { raw_data.event_api_id.clone() },
                record_path: "$.events[*]".to_string(),
                record: raw_data.data,
            };
            
            // Apply normalization
            match normalizer.normalize(&parsed_record) {
                Ok(normalized_records) => {
                    normalized_count += normalized_records.len();
                    debug!("Normalized {} records from raw data {}", normalized_records.len(), if raw_data.event_name.is_empty() { "unknown" } else { &raw_data.event_name });
                    // TODO: Store normalized records in database
                },
                Err(e) => {
                    error!("Failed to normalize raw data {}: {}", if raw_data.event_name.is_empty() { "unknown" } else { &raw_data.event_name }, e);
                    errors += 1;
                }
            }
        }
        
        let message = format!(
            "Normalize completed for {}: {} records normalized ({} errors)",
            source_id, normalized_count, errors
        );
        info!("✅ {}", message);
        
        Ok(StepResult::success(normalized_count, message))
    }
    
    fn step_name(&self) -> &'static str {
        "normalize"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["parse"]
    }
}
