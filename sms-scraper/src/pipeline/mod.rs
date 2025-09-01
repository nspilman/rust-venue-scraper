// Pipeline orchestration and processing modules

pub mod full_pipeline_orchestrator;
pub mod ingestion;
pub mod steps;
pub mod pipeline_config;
pub mod orchestrator;
pub mod utils;
pub mod storage; // Storage traits and implementations
pub mod processing; // Legacy processing module for backward compatibility
// pub mod parquet_out; // Disabled due to missing parquet dependency

// Re-export key types for convenience
pub use pipeline_config::{PipelineConfig, PipelineStepConfig, ErrorHandlingStrategy};
pub use orchestrator::PipelineOrchestrator;
pub use steps::{PipelineStep, StepResult};

// Re-export all step types
pub use steps::{
    IngestionStep,
    ParseStep,
    NormalizeStep,
    QualityGateStep,
    EnrichStep,
    ConflationStep,
    CatalogStep,
};

// Re-export utility types and functions
pub use utils::{
    StringUtils,
    EntityResolver,
    EventCategorizer,
    QualityValidator,
    CatalogValidator,
    QualityResult,
    CatalogEntry,
    LocationInfo,
};

// Re-export full pipeline orchestrator for backward compatibility
pub use full_pipeline_orchestrator::FullPipelineOrchestrator;
