use anyhow::Result;
use async_trait::async_trait;
use sms_core::storage::Storage;

/// Common trait for all pipeline steps
#[async_trait]
pub trait PipelineStep: Send + Sync {
    /// Execute this pipeline step for a given source
    async fn execute(&self, source_id: &str, storage: &dyn Storage) -> Result<StepResult>;
    
    /// Get the name of this pipeline step
    fn step_name(&self) -> &'static str;
    
    /// Get the dependencies this step requires (previous steps that must complete)
    fn dependencies(&self) -> Vec<&'static str>;
    
    /// Check if this step can run in parallel with other steps
    fn can_run_parallel(&self) -> bool {
        false
    }
}

/// Result of executing a pipeline step
#[derive(Debug, Clone)]
pub struct StepResult {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub error_count: usize,
    pub message: String,
    pub metadata: std::collections::HashMap<String, String>,
}

impl StepResult {
    pub fn success(processed: usize, message: String) -> Self {
        Self {
            success: true,
            processed_count: processed,
            failed_count: 0,
            error_count: 0,
            message,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_errors(processed: usize, failed: usize, errors: usize, message: String) -> Self {
        Self {
            success: errors == 0 && failed == 0,
            processed_count: processed,
            failed_count: failed,
            error_count: errors,
            message,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            processed_count: 0,
            failed_count: 0,
            error_count: 1,
            message,
            metadata: std::collections::HashMap::new(),
        }
    }
}

// Re-export all pipeline steps
pub mod ingestion;
pub mod parse;
pub mod normalize;
pub mod quality_gate;
pub mod enrich;
pub mod conflation;
pub mod catalog;

pub use ingestion::IngestionStep;
pub use parse::ParseStep;
pub use normalize::NormalizeStep;
pub use quality_gate::QualityGateStep;
pub use enrich::EnrichStep;
pub use conflation::ConflationStep;
pub use catalog::CatalogStep;
