use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;

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

/// Trait for normalizing parsed records into canonical domain models
pub trait Normalizer {
    /// Transform a parsed record into one or more normalized domain entities
    fn normalize(&self, record: &ParsedRecord) -> anyhow::Result<Vec<NormalizedRecord>>;
}

/// Default normalizer that handles the common venue/event/artist extraction patterns
pub struct DefaultNormalizer {
    /// Geocoder for converting addresses to coordinates (placeholder for now)
    pub geocoder: Option<Box<dyn Geocoder>>,
}

impl DefaultNormalizer {

    /// Extract event information from a parsed record
    fn extract_event(&self, record: &ParsedRecord) -> anyhow::Result<Option<Event>> {
        let data = &record.record;
        
        // Extract basic event fields
        let title = data.get("title")
            .or_else(|| data.get("name"))
            .or_else(|| data.get("event_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Event")
            .to_string();

        // Parse event date
        let event_day = if let Some(day_str) = data.get("event_day").and_then(|v| v.as_str()) {
            NaiveDate::parse_from_str(day_str, "%Y-%m-%d")
                .or_else(|_| NaiveDate::parse_from_str(day_str, "%m/%d/%Y"))
                .or_else(|_| NaiveDate::parse_from_str(day_str, "%m-%d-%Y"))
                .unwrap_or_else(|_| Utc::now().naive_utc().date())
        } else {
            Utc::now().naive_utc().date()
        };

        // Extract start time if available
        let start_time = data.get("start_time")
            .and_then(|v| v.as_str())
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok());

        // Extract other event fields
        let event_url = data.get("event_url")
            .or_else(|| data.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let description = data.get("description")
            .or_else(|| data.get("desc"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let event_image_url = data.get("image_url")
            .or_else(|| data.get("image"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Create the event (venue_id and artist_ids will be resolved later)
        let event = Event {
            id: None, // Will be assigned during persistence
            title,
            event_day,
            start_time,
            event_url,
            description,
            event_image_url,
            venue_id: Uuid::nil(), // Placeholder - will be resolved in conflation
            artist_ids: Vec::new(), // Placeholder - will be resolved in conflation
            show_event: true,
            finalized: false,
            created_at: Utc::now(),
        };

        Ok(Some(event))
    }

    /// Extract venue information from a parsed record
    fn extract_venue(&self, record: &ParsedRecord) -> anyhow::Result<Option<Venue>> {
        let data = &record.record;
        
        // Extract venue name
        let name = data.get("venue")
            .or_else(|| data.get("venue_name"))
            .or_else(|| data.get("location"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if name.is_none() {
            return Ok(None); // No venue information available
        }
        
        let name = name.unwrap();
        let name_lower = name.to_lowercase();
        let slug = name_lower.replace(' ', "-").replace(['\'', '"'], "");

        // Extract address information
        let address = data.get("address")
            .or_else(|| data.get("venue_address"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let city = data.get("city")
            .or_else(|| data.get("venue_city"))
            .and_then(|v| v.as_str())
            .unwrap_or("Seattle") // Default to Seattle for now
            .to_string();

        let postal_code = data.get("postal_code")
            .or_else(|| data.get("zip"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Extract or geocode coordinates
        let (latitude, longitude, _geocoded) = if let (Some(lat), Some(lng)) = (
            data.get("latitude").and_then(|v| v.as_f64()),
            data.get("longitude").and_then(|v| v.as_f64())
        ) {
            (lat, lng, false)
        } else if !address.is_empty() && self.geocoder.is_some() {
            // TODO: Implement actual geocoding
            // For now, return Seattle coordinates as placeholder
            (47.6062, -122.3321, true)
        } else {
            // Default Seattle coordinates
            (47.6062, -122.3321, true)
        };

        let venue_url = data.get("venue_url")
            .or_else(|| data.get("website"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let venue_image_url = data.get("venue_image_url")
            .or_else(|| data.get("venue_image"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let description = data.get("venue_description")
            .or_else(|| data.get("venue_desc"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let neighborhood = data.get("neighborhood")
            .or_else(|| data.get("district"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let venue = Venue {
            id: None, // Will be assigned during persistence
            name,
            name_lower,
            slug,
            latitude,
            longitude,
            address,
            postal_code,
            city,
            venue_url,
            venue_image_url,
            description,
            neighborhood,
            show_venue: true,
            created_at: Utc::now(),
        };

        Ok(Some(venue))
    }

    /// Extract artist information from a parsed record
    fn extract_artists(&self, record: &ParsedRecord) -> anyhow::Result<Vec<Artist>> {
        let data = &record.record;
        let mut artists = Vec::new();

        // Look for artist information in various fields
        let artist_names = if let Some(artists_array) = data.get("artists").and_then(|v| v.as_array()) {
            artists_array.iter()
                .filter_map(|a| a.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        } else if let Some(artist_str) = data.get("artist")
            .or_else(|| data.get("performers"))
            .or_else(|| data.get("bands"))
            .and_then(|v| v.as_str()) {
            // Split on common delimiters
            artist_str.split(&[',', '&', '+'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        for name in artist_names {
            if name.is_empty() {
                continue;
            }

            let name_slug = name.to_lowercase().replace(' ', "-").replace(['\'', '"'], "");
            
            let artist = Artist {
                id: None, // Will be assigned during persistence
                name,
                name_slug,
                bio: None, // Not usually available in source data
                artist_image_url: None, // Not usually available in source data
                created_at: Utc::now(),
            };

            artists.push(artist);
        }

        Ok(artists)
    }
}

impl Normalizer for DefaultNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> anyhow::Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let mut warnings = Vec::new();
        let normalized_at = Utc::now();

        // Create provenance info
        let provenance = RecordProvenance {
            envelope_id: record.envelope_id.clone(),
            source_id: record.source_id.clone(),
            payload_ref: record.payload_ref.clone(),
            record_path: record.record_path.clone(),
            normalized_at,
        };

        // Extract event
        if let Some(event) = self.extract_event(record)? {
            let metadata = NormalizationMetadata {
                confidence: 0.9, // High confidence for event extraction
                warnings: warnings.clone(),
                geocoded: false,
                strategy: "default_event_extraction".to_string(),
            };

            results.push(NormalizedRecord {
                entity: NormalizedEntity::Event(event),
                provenance: provenance.clone(),
                normalization: metadata,
            });
        }

        // Extract venue
        if let Some(venue) = self.extract_venue(record)? {
            let geocoded = venue.latitude != 47.6062 || venue.longitude != -122.3321;
            let confidence = if geocoded { 0.9 } else { 0.7 }; // Lower confidence for default coords
            
            if !geocoded {
                warnings.push("Using default Seattle coordinates - address geocoding not available".to_string());
            }

            let metadata = NormalizationMetadata {
                confidence,
                warnings: warnings.clone(),
                geocoded,
                strategy: "default_venue_extraction".to_string(),
            };

            results.push(NormalizedRecord {
                entity: NormalizedEntity::Venue(venue),
                provenance: provenance.clone(),
                normalization: metadata,
            });
        }

        // Extract artists
        let artists = self.extract_artists(record)?;
        for artist in artists {
            let metadata = NormalizationMetadata {
                confidence: 0.8, // Medium confidence for artist extraction
                warnings: warnings.clone(),
                geocoded: false,
                strategy: "default_artist_extraction".to_string(),
            };

            results.push(NormalizedRecord {
                entity: NormalizedEntity::Artist(artist),
                provenance: provenance.clone(),
                normalization: metadata,
            });
        }

        Ok(results)
    }
}

/// Trait for geocoding addresses to coordinates
/// This is a placeholder for future geocoding implementation
pub trait Geocoder: Send + Sync {
    fn geocode(&self, address: &str) -> anyhow::Result<Option<(f64, f64)>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_simple_event() {
        let normalizer = DefaultNormalizer { geocoder: None };
        
        let record = ParsedRecord {
            source_id: "test_source".to_string(),
            envelope_id: "test_envelope".to_string(),
            payload_ref: "test_payload".to_string(),
            record_path: "$.events[0]".to_string(),
            record: json!({
                "title": "Test Concert",
                "event_day": "2025-08-15",
                "venue": "Test Venue",
                "artist": "Test Artist"
            }),
        };

        let results = normalizer.normalize(&record).unwrap();
        
        // Should extract event, venue, and artist
        assert_eq!(results.len(), 3);
        
        // Check that we have one of each type
        let event_count = results.iter().filter(|r| matches!(r.entity, NormalizedEntity::Event(_))).count();
        let venue_count = results.iter().filter(|r| matches!(r.entity, NormalizedEntity::Venue(_))).count();
        let artist_count = results.iter().filter(|r| matches!(r.entity, NormalizedEntity::Artist(_))).count();
        
        assert_eq!(event_count, 1);
        assert_eq!(venue_count, 1);
        assert_eq!(artist_count, 1);
    }

    #[test]
    fn test_normalize_multiple_artists() {
        let normalizer = DefaultNormalizer { geocoder: None };
        
        let record = ParsedRecord {
            source_id: "test_source".to_string(),
            envelope_id: "test_envelope".to_string(),
            payload_ref: "test_payload".to_string(),
            record_path: "$.events[0]".to_string(),
            record: json!({
                "title": "Multi-Artist Show",
                "event_day": "2025-08-15",
                "venue": "Test Venue",
                "artist": "Artist One, Artist Two & Artist Three"
            }),
        };

        let results = normalizer.normalize(&record).unwrap();
        
        // Should extract event, venue, and 3 artists
        assert_eq!(results.len(), 5);
        
        let artist_count = results.iter().filter(|r| matches!(r.entity, NormalizedEntity::Artist(_))).count();
        assert_eq!(artist_count, 3);
    }
}
