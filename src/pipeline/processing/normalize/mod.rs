use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{Artist, Event, Venue};

pub mod normalizers;
pub mod registry;

pub use registry::NormalizationRegistry;

/// A normalized record that has been converted into canonical domain shapes
/// but retains lineage back to the source envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRecord {
    /// The canonical domain entity extracted from the raw record
    pub entity: NormalizedEntity,
    /// Provenance information linking back to the source
    pub provenance: RecordProvenance,
    /// Normalization metadata (confidence, processing time, etc.)
    pub normalization: NormalizationMetadata,
}

/// The canonical domain entities that can be extracted from source records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizedEntity {
    Event(Event),
    Venue(Venue),
    Artist(Artist),
}

/// Provenance information tracking the lineage of this normalized record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordProvenance {
    /// The envelope ID that introduced this record
    pub envelope_id: String,
    /// The source system that provided the data
    pub source_id: String,
    /// Reference to the raw payload in the content-addressed store
    pub payload_ref: String,
    /// JSONPath to this specific record within the payload
    pub record_path: String,
    /// When this record was processed into its canonical form
    pub normalized_at: DateTime<Utc>,
}

/// Metadata about the normalization process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationMetadata {
    /// Confidence level in the normalization (0.0 to 1.0)
    pub confidence: f64,
    /// Any warnings or notes from the normalization process
    pub warnings: Vec<String>,
    /// Whether coordinates were geocoded from an address
    pub geocoded: bool,
    /// The normalization strategy used
    pub strategy: String,
}
