use chrono::Utc;
use std::sync::{Arc, Mutex};
use anyhow::Result;

use sms_core::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::observability::metrics;
use super::super::{NormalizedRecord, NormalizedEntity, RecordProvenance, NormalizationMetadata};

/// Base trait for source-specific normalizers
pub trait SourceNormalizer: Send + Sync {
    /// Normalize a parsed record into normalized entities
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>>;
    
    /// Get the source ID this normalizer handles
    fn source_id(&self) -> &str;
    
    /// Get a human-readable name for this normalizer
    fn name(&self) -> &str;
}

/// A wrapper that adds metrics to any normalizer implementation
pub struct MetricsNormalizer<N: SourceNormalizer> {
    inner: N,
}

impl<N: SourceNormalizer> MetricsNormalizer<N> {
    pub fn new(inner: N) -> Self {
        Self { inner }
    }
}

impl<N: SourceNormalizer> SourceNormalizer for MetricsNormalizer<N> {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let _start_time = std::time::Instant::now();
        
        match self.inner.normalize(record) {
            Ok(normalized_records) => {
                // Record successful normalization with strategy
                let strategy = self.inner.source_id();
                metrics::normalize::record_normalized(strategy);
                
                // Record confidence levels
                for record in &normalized_records {
                    metrics::normalize::confidence_recorded(record.normalization.confidence);
                    
                    // Check if geocoding was performed
                    if record.normalization.geocoded {
                        metrics::normalize::geocoding_performed();
                    }
                    
                    // Record warnings
                    for warning in &record.normalization.warnings {
                        metrics::normalize::warning_logged(warning);
                    }
                }
                
                Ok(normalized_records)
            }
            Err(e) => {
                // Record normalization error
                metrics::normalize::warning_logged("normalization_error");
                Err(e)
            }
        }
    }
    
    fn source_id(&self) -> &str {
        self.inner.source_id()
    }
    
    fn name(&self) -> &str {
        self.inner.name()
    }
}

/// Shared venue state manager for single-venue sources
/// Prevents creating duplicate venue records when processing multiple events from the same venue
pub struct VenueStateManager {
    venue_created: Arc<Mutex<bool>>,
}

impl VenueStateManager {
    pub fn new() -> Self {
        Self {
            venue_created: Arc::new(Mutex::new(false)),
        }
    }

    /// Check if venue has already been created, and mark it as created if not
    /// Returns true if the venue should be created (first time), false if it's already been created
    pub fn should_create_venue(&self) -> bool {
        if let Ok(mut created) = self.venue_created.lock() {
            if !*created {
                *created = true;
                true
            } else {
                false
            }
        } else {
            // If we can't acquire the lock, err on the side of caution and don't create
            false
        }
    }

    /// Test-only: reset state
    #[cfg(test)]
    pub fn reset(&self) {
        if let Ok(mut created) = self.venue_created.lock() {
            *created = false;
        }
    }
}

/// Shared artist state manager to prevent duplicate artist records
/// Tracks artist names that have already been created within a normalization batch
pub struct ArtistStateManager {
    created_artists: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl ArtistStateManager {
    pub fn new() -> Self {
        Self {
            created_artists: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Check if artist has already been created, and mark it as created if not
    /// Returns true if the artist should be created (first time), false if it's already been created
    /// Uses the artist name_slug for comparison to handle case/punctuation variations
    pub fn should_create_artist(&self, name_slug: &str) -> bool {
        if let Ok(mut created_set) = self.created_artists.lock() {
            if !created_set.contains(name_slug) {
                created_set.insert(name_slug.to_string());
                true
            } else {
                false
            }
        } else {
            // If we can't acquire the lock, err on the side of caution and don't create
            false
        }
    }

    /// Test-only: reset state
    #[cfg(test)]
    pub fn reset(&self) {
        if let Ok(mut created_set) = self.created_artists.lock() {
            created_set.clear();
        }
    }
}

impl Default for VenueStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ArtistStateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared utilities for normalizers
pub struct NormalizerUtils;

impl NormalizerUtils {
    /// Generate a URL-friendly slug from a name
    pub fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .replace(' ', "-")
            .replace(['\'', '"', '.', ',', '!', '?', '&'], "")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    /// Create a base record provenance from a parsed record
    pub fn create_provenance(record: &ParsedRecord) -> RecordProvenance {
        RecordProvenance {
            envelope_id: record.envelope_id.clone(),
            source_id: record.source_id.clone(),
            payload_ref: record.payload_ref.clone(),
            record_path: record.record_path.clone(),
            normalized_at: Utc::now(),
        }
    }

    /// Create a normalized venue record with standard metadata
    pub fn create_venue_record(
        venue: Venue,
        provenance: RecordProvenance,
        confidence: f64,
        strategy: String,
    ) -> NormalizedRecord {
        NormalizedRecord {
            entity: NormalizedEntity::Venue(venue),
            provenance,
            normalization: NormalizationMetadata {
                confidence,
                warnings: Vec::new(),
                geocoded: false,
                strategy,
            },
        }
    }

    /// Create a normalized event record with standard metadata
    pub fn create_event_record(
        event: Event,
        provenance: RecordProvenance,
        confidence: f64,
        strategy: String,
    ) -> NormalizedRecord {
        NormalizedRecord {
            entity: NormalizedEntity::Event(event),
            provenance,
            normalization: NormalizationMetadata {
                confidence,
                warnings: Vec::new(),
                geocoded: false,
                strategy,
            },
        }
    }

    /// Create a normalized artist record with standard metadata
    pub fn create_artist_record(
        artist: Artist,
        provenance: RecordProvenance,
        confidence: f64,
        strategy: String,
    ) -> NormalizedRecord {
        NormalizedRecord {
            entity: NormalizedEntity::Artist(artist),
            provenance,
            normalization: NormalizationMetadata {
                confidence,
                warnings: Vec::new(),
                geocoded: false,
                strategy,
            },
        }
    }

    /// Extract and clean title from data, returning None if not found or invalid
    pub fn extract_title(data: &serde_json::Value) -> Option<String> {
        data.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Check if a title represents a non-artist event (like open mic, karaoke, etc.)
    pub fn is_non_artist_event(title: &str) -> bool {
        let title_lower = title.to_lowercase();
        title_lower.contains("open mic") ||
        title_lower.contains("karaoke") ||
        title_lower.contains("trivia") ||
        title_lower.contains("comedy") ||
        title_lower.contains("open jam") ||
        title_lower.contains("dj night") ||
        title_lower.contains("bingo")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug() {
        assert_eq!(NormalizerUtils::generate_slug("Darrell's Tavern"), "darrells-tavern");
        assert_eq!(NormalizerUtils::generate_slug("Sea Monster Lounge!"), "sea-monster-lounge");
        assert_eq!(NormalizerUtils::generate_slug("Blue Moon Tavern & Grill"), "blue-moon-tavern-grill");
        assert_eq!(NormalizerUtils::generate_slug("The Venue, Seattle"), "the-venue-seattle");
    }

    #[test]
    fn test_venue_state_manager() {
        let manager = VenueStateManager::new();
        
        // First call should return true (create venue)
        assert!(manager.should_create_venue());
        
        // Subsequent calls should return false (don't create)
        assert!(!manager.should_create_venue());
        assert!(!manager.should_create_venue());
        
        // Reset and try again
        manager.reset();
        assert!(manager.should_create_venue());
        assert!(!manager.should_create_venue());
    }

    #[test]
    fn test_artist_state_manager() {
        let manager = ArtistStateManager::new();
        
        // First call with a slug should return true (create artist)
        assert!(manager.should_create_artist("the-beatles"));
        
        // Same slug should return false (don't create duplicate)
        assert!(!manager.should_create_artist("the-beatles"));
        assert!(!manager.should_create_artist("the-beatles"));
        
        // Different slug should return true
        assert!(manager.should_create_artist("rolling-stones"));
        assert!(!manager.should_create_artist("rolling-stones"));
        
        // Reset and try again
        manager.reset();
        assert!(manager.should_create_artist("the-beatles"));
        assert!(!manager.should_create_artist("the-beatles"));
    }


    #[test]
    fn test_is_non_artist_event() {
        assert!(NormalizerUtils::is_non_artist_event("Open Mic Night"));
        assert!(NormalizerUtils::is_non_artist_event("Karaoke Thursday"));
        assert!(NormalizerUtils::is_non_artist_event("Trivia Night"));
        assert!(NormalizerUtils::is_non_artist_event("Comedy Show"));
        assert!(NormalizerUtils::is_non_artist_event("Open Jam Session"));
        
        assert!(!NormalizerUtils::is_non_artist_event("The Beatles"));
        assert!(!NormalizerUtils::is_non_artist_event("Rock Band"));
        assert!(!NormalizerUtils::is_non_artist_event("Jazz Ensemble"));
    }

    #[test]
    fn test_extract_title() {
        let data = serde_json::json!({"title": "Test Event"});
        assert_eq!(NormalizerUtils::extract_title(&data), Some("Test Event".to_string()));
        
        let data = serde_json::json!({"title": "  Spaced  "});
        assert_eq!(NormalizerUtils::extract_title(&data), Some("Spaced".to_string()));
        
        let data = serde_json::json!({"title": ""});
        assert_eq!(NormalizerUtils::extract_title(&data), None);
        
        let data = serde_json::json!({"name": "Test Event"});
        assert_eq!(NormalizerUtils::extract_title(&data), None);
    }
}
