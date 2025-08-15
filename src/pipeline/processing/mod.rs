// Pipeline processing: data parsing, validation, and transformation

pub mod parser;
pub mod normalize;

// Re-export key types and functions
pub use normalize::{NormalizedRecord, NormalizedEntity, Normalizer, DefaultNormalizer};
