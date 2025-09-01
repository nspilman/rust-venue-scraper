use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, error};
use sms_core::storage::Storage;
use sms_core::domain::RawData;
use crate::registry::source_loader::SourceRegistry;
use super::{PipelineStep, StepResult};

/// Pipeline step for ingesting raw data from external sources
pub struct IngestionStep {
    source_registry: SourceRegistry,
}

impl IngestionStep {
    pub fn new(source_registry: SourceRegistry) -> Self {
        Self { source_registry }
    }
}

#[async_trait]
impl PipelineStep for IngestionStep {
    async fn execute(&self, source_id: &str, storage: &dyn Storage) -> Result<StepResult> {
        info!("🔄 Running ingestion for source: {}", source_id);
        
        // Map user-friendly source names to internal API names
        let internal_api_name = crate::common::constants::api_name_to_internal(source_id);
        
        // Create crawler for this source
        let crawler = crate::apis::factory::create_crawler(source_id, self.source_registry.clone())?
            .ok_or_else(|| anyhow::anyhow!("Failed to create crawler for source: {}", source_id))?;
        
        // Fetch raw event data
        let raw_event_data = crawler.get_event_list().await?;
        
        if raw_event_data.is_empty() {
            let message = format!("No raw data fetched for source: {}", source_id);
            info!("{}", message);
            return Ok(StepResult::success(0, message));
        }
        
        info!("📊 Fetched {} raw data items for {}", raw_event_data.len(), source_id);
        
        let mut stored_count = 0;
        let mut failed_count = 0;
        
        // Store each raw data item
        for (index, event_data) in raw_event_data.iter().enumerate() {
            let timestamp = chrono::Utc::now();
            let raw_data_id = format!("{}_{}_raw_{}", source_id, timestamp.timestamp(), index);
            
            let mut raw_data = RawData {
                id: None,
                event_api_id: raw_data_id,
                event_name: format!("Raw data from {}", source_id),
                venue_name: source_id.to_string(),
                event_day: chrono::Utc::now().date_naive(),
                api_name: internal_api_name.clone(),
                data: event_data.clone(),
                processed: false,
                event_id: None,
                created_at: timestamp,
            };
            
            match storage.create_raw_data(&mut raw_data).await {
                Ok(()) => {
                    stored_count += 1;
                }
                Err(e) => {
                    error!("Failed to store raw data item {}: {}", index, e);
                    failed_count += 1;
                }
            }
        }
        
        let message = format!(
            "Ingestion completed for {}: {} items stored, {} failed",
            source_id, stored_count, failed_count
        );
        
        info!("✅ {}", message);
        Ok(StepResult::with_errors(stored_count, failed_count, 0, message))
    }
    
    fn step_name(&self) -> &'static str {
        "ingestion"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec![] // No dependencies - this is the first step
    }
}
