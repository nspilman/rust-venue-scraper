// Pipeline processing: data parsing, validation, and transformation

pub mod parser;
pub mod normalize;
pub mod quality_gate;
pub mod enrich;
pub mod conflation;
pub mod catalog;
pub mod pipeline_steps;

// Re-export key types and functions

// Re-export pipeline steps for easy access
pub use pipeline_steps::{
    process_raw_data,
    ProcessedData,
    ParsedRecord,
    NormalizedRecord,
    QualityAssessedRecord,
    QualityDecision,
};
