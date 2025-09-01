use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error, warn};
use sms_core::storage::{Storage, DatabaseStorage};
use crate::registry::source_loader::SourceRegistry;
use super::pipeline_config::{PipelineConfig, PipelineStepConfig, ErrorHandlingStrategy};
use super::steps::{
    PipelineStep, StepResult,
    IngestionStep, ParseStep, NormalizeStep, QualityGateStep, 
    EnrichStep, ConflationStep, CatalogStep
};

/// Lightweight orchestrator for running declarative pipelines
pub struct PipelineOrchestrator {
    storage: Arc<dyn Storage>,
    source_registry: SourceRegistry,
}

impl PipelineOrchestrator {
    /// Create a new pipeline orchestrator
    pub async fn new() -> Result<Self> {
        let storage = Arc::new(DatabaseStorage::new().await?);
        let source_registry = SourceRegistry::load_from_directory("registry/sources")?;
        Ok(Self { storage, source_registry })
    }

    /// Run a complete pipeline based on configuration
    pub async fn run_pipeline(&self, config: PipelineConfig, source_id: &str) -> Result<PipelineExecutionResult> {
        info!("🚀 Starting pipeline '{}' for source: {}", config.name, source_id);
        info!("📋 Pipeline description: {}", config.description);
        
        // Validate pipeline configuration
        config.validate()?;
        
        let mut execution_result = PipelineExecutionResult::new(config.name.clone(), source_id.to_string());
        let mut should_continue = true;
        
        for (step_index, step_config) in config.steps.iter().enumerate() {
            if !should_continue {
                warn!("⏹️ Stopping pipeline execution due to previous error");
                break;
            }
            
            info!("🔄 Executing step {}/{}: {}", step_index + 1, config.steps.len(), step_config.step_name());
            
            let step = self.create_step(step_config.clone())?;
            
            match step.execute(source_id, &*self.storage).await {
                Ok(step_result) => {
                    info!("✅ Step '{}' completed: {}", step_config.step_name(), step_result.message);
                    execution_result.add_step_result(step_config.step_name().to_string(), step_result.clone());
                    
                    // Check if we should continue based on error handling strategy
                    if !step_result.success {
                        match config.error_handling {
                            ErrorHandlingStrategy::StopOnFirstError => {
                                error!("❌ Stopping pipeline due to step failure: {}", step_result.message);
                                should_continue = false;
                                execution_result.success = false;
                            }
                            ErrorHandlingStrategy::ContinueOnError => {
                                warn!("⚠️ Step failed but continuing: {}", step_result.message);
                            }
                            ErrorHandlingStrategy::SkipFailedItems => {
                                info!("⏭️ Skipping failed items and continuing: {}", step_result.message);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Step '{}' failed with error: {}", step_config.step_name(), e);
                    let step_result = StepResult::failure(format!("Step failed: {}", e));
                    execution_result.add_step_result(step_config.step_name().to_string(), step_result);
                    
                    match config.error_handling {
                        ErrorHandlingStrategy::StopOnFirstError => {
                            should_continue = false;
                            execution_result.success = false;
                        }
                        ErrorHandlingStrategy::ContinueOnError | ErrorHandlingStrategy::SkipFailedItems => {
                            warn!("⚠️ Step failed but continuing due to error handling strategy");
                        }
                    }
                }
            }
        }
        
        let total_processed = execution_result.step_results.values()
            .map(|r| r.processed_count)
            .sum::<usize>();
        
        let total_failed = execution_result.step_results.values()
            .map(|r| r.failed_count + r.error_count)
            .sum::<usize>();
        
        execution_result.total_processed = total_processed;
        execution_result.total_failed = total_failed;
        
        if execution_result.success {
            info!("🎉 Pipeline '{}' completed successfully for {}: {} processed, {} failed", 
                  config.name, source_id, total_processed, total_failed);
        } else {
            error!("💥 Pipeline '{}' failed for {}: {} processed, {} failed", 
                   config.name, source_id, total_processed, total_failed);
        }
        
        Ok(execution_result)
    }
    
    /// Run a single step independently
    pub async fn run_step(&self, step_config: PipelineStepConfig, source_id: &str) -> Result<StepResult> {
        info!("🔄 Running single step '{}' for source: {}", step_config.step_name(), source_id);
        
        let step = self.create_step(step_config)?;
        step.execute(source_id, &*self.storage).await
    }
    
    /// Create a step instance from configuration
    fn create_step(&self, step_config: PipelineStepConfig) -> Result<Box<dyn PipelineStep>> {
        let step: Box<dyn PipelineStep> = match step_config {
            PipelineStepConfig::Ingestion => {
                Box::new(IngestionStep::new(self.source_registry.clone()))
            }
            PipelineStepConfig::Parse => {
                Box::new(ParseStep::new(self.source_registry.clone()))
            }
            PipelineStepConfig::Normalize => {
                Box::new(NormalizeStep::new().map_err(|e| anyhow::anyhow!("Failed to create normalize step: {}", e))?)
            }
            PipelineStepConfig::QualityGate { threshold } => {
                Box::new(QualityGateStep::new(threshold.unwrap_or(0.8)))
            }
            PipelineStepConfig::Enrich => {
                Box::new(EnrichStep::new())
            }
            PipelineStepConfig::Conflation { confidence_threshold } => {
                Box::new(ConflationStep::new(confidence_threshold.unwrap_or(0.85)))
            }
            PipelineStepConfig::Catalog { validate_graph } => {
                Box::new(CatalogStep::new(validate_graph))
            }
        };
        
        Ok(step)
    }
}

/// Result of executing a complete pipeline
#[derive(Debug, Clone)]
pub struct PipelineExecutionResult {
    pub pipeline_name: String,
    pub source_id: String,
    pub success: bool,
    pub total_processed: usize,
    pub total_failed: usize,
    pub step_results: std::collections::HashMap<String, StepResult>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl PipelineExecutionResult {
    pub fn new(pipeline_name: String, source_id: String) -> Self {
        Self {
            pipeline_name,
            source_id,
            success: true,
            total_processed: 0,
            total_failed: 0,
            step_results: std::collections::HashMap::new(),
            started_at: chrono::Utc::now(),
            completed_at: None,
        }
    }
    
    pub fn add_step_result(&mut self, step_name: String, result: StepResult) {
        self.step_results.insert(step_name, result);
    }
    
    pub fn complete(&mut self) {
        self.completed_at = Some(chrono::Utc::now());
    }
    
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at.map(|end| end - self.started_at)
    }
}
