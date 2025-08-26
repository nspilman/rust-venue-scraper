// Base trait and utilities for source-specific normalizers
pub mod base;

// Individual normalizer implementations
pub mod barboza;
pub mod blue_moon;
pub mod conor_byrne;
pub mod darrells_tavern;
pub mod kexp;
pub mod neumos;
pub mod sea_monster;

// Re-export the main components
pub use base::{SourceNormalizer, MetricsNormalizer};
pub use barboza::BarbozaNormalizer;
pub use blue_moon::BlueMoonNormalizer;
pub use conor_byrne::ConorByrneNormalizer;
pub use darrells_tavern::DarrellsTavernNormalizer;
pub use kexp::KexpNormalizer;
pub use neumos::NeumosNormalizer;
pub use sea_monster::SeaMonsterNormalizer;
