// Base trait and utilities for source-specific normalizers
pub mod base;

// Individual normalizer implementations
pub mod sea_monster;
pub mod darrells_tavern;
pub mod blue_moon;
pub mod kexp;
pub mod barboza;

// Re-export the main components
pub use base::{SourceNormalizer, MetricsNormalizer};
pub use sea_monster::SeaMonsterNormalizer;
pub use darrells_tavern::DarrellsTavernNormalizer;
pub use blue_moon::BlueMoonNormalizer;
pub use kexp::KexpNormalizer;
pub use barboza::BarbozaNormalizer;
