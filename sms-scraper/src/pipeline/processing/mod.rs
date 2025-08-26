// Pipeline processing: data parsing, validation, and transformation

pub mod parser;
pub mod normalize;
pub mod quality_gate;
pub mod enrich;
pub mod conflation;
pub mod catalog;

// Re-export key types and functions
