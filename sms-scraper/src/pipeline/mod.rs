// Data processing pipeline: ingestion, processing, and storage

pub mod ingestion;
pub mod processing;
pub mod storage;
// pub mod parquet_out; // Temporarily disabled - requires parquet dependency
pub mod pipeline;
pub mod tasks;
pub mod full_pipeline_orchestrator;

// Re-export key types and functions from each stage
pub use full_pipeline_orchestrator::{FullPipelineOrchestrator, ProcessingResult};
