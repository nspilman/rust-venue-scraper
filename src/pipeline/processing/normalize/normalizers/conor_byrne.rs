use chrono::{NaiveDate, NaiveTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for Conor Byrne events
pub struct ConorByrneNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl ConorByrneNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl Default for ConorByrneNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceNormalizer for ConorByrneNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let data = &record.record;
        let provenance = NormalizerUtils::create_provenance(record);

        // Extract title
        if let Some(title) = NormalizerUtils::extract_title(data) {
            // Parse event date and time (assuming similar format to other venues)
            let event_day = data.get("event_day")
                .and_then(|v| v.as_str())
                .and_then(|date_str| NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok())
                .unwrap_or_else(|| Utc::now().naive_utc().date());

            let start_time = data.get("event_time")
                .and_then(|v| v.as_str())
                .and_then(|time_str| {
                    // Try common time formats
                    NaiveTime::parse_from_str(time_str, "%I:%M %p")
                        .or_else(|_| NaiveTime::parse_from_str(time_str, "%l:%M %p"))
                        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M"))
                        .ok()
                });

            // Build event description from available metadata
            let mut description_parts = Vec::new();
            
            if let Some(supporting) = data.get("supporting_acts")
                .and_then(|v| v.as_str()) 
                .filter(|s| !s.is_empty()) {
                description_parts.push(format!("With {}", supporting));
            }

            if let Some(age) = data.get("age_restriction")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()) {
                description_parts.push(format!("Age: {}", age));
            }

            let description = if !description_parts.is_empty() {
                Some(description_parts.join(" | "))
            } else {
                None
            };

            // Get event URL if available
            let event_url = data.get("event_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| Some("https://www.conorbyrnepub.com".to_string()));

            // Get image URL if available
            let event_image_url = data.get("image_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Collect artist IDs as we create artists
            let mut event_artist_ids = Vec::new();
            
            // Extract artists from title and supporting acts
            if !NormalizerUtils::is_non_artist_event(&title) {
                let name_slug = NormalizerUtils::generate_slug(&title);
                
                // Generate deterministic UUID from the artist slug
                let artist_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name_slug.as_bytes());
                
                // Track this artist ID for the event
                event_artist_ids.push(artist_id);
                
                // Only create the artist entity if we haven't seen it before
                if self.artist_state.should_create_artist(&name_slug) {
                    let artist = Artist {
                        id: Some(artist_id),
                        name: title.clone(),
                        name_slug,
                        bio: None,
                        artist_image_url: None,
                        created_at: Utc::now(),
                    };

                    results.push(NormalizerUtils::create_artist_record(
                        artist, 
                        provenance.clone(), 
                        0.9, 
                        "conor_byrne_artist_headliner".to_string()
                    ));
                }
            }

            // Extract supporting acts as artists
            if let Some(supporting) = data.get("supporting_acts").and_then(|v| v.as_str()) {
                // Split supporting acts by common delimiters
                let acts: Vec<&str> = supporting.split(&[',', '&'][..])
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                for act in acts {
                    if !NormalizerUtils::is_non_artist_event(act) {
                        let name_slug = NormalizerUtils::generate_slug(act);
                        
                        // Generate deterministic UUID from the artist slug
                        let artist_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name_slug.as_bytes());
                        
                        // Track this artist ID for the event
                        event_artist_ids.push(artist_id);
                        
                        // Only create the artist entity if we haven't seen it before
                        if self.artist_state.should_create_artist(&name_slug) {
                            let artist = Artist {
                                id: Some(artist_id),
                                name: act.to_string(),
                                name_slug,
                                bio: None,
                                artist_image_url: None,
                                created_at: Utc::now(),
                            };

                            results.push(NormalizerUtils::create_artist_record(
                                artist, 
                                provenance.clone(), 
                                0.85, 
                                "conor_byrne_artist_supporting".to_string()
                            ));
                        }
                    }
                }
            }

            // Generate deterministic venue ID for Conor Byrne
            let venue_slug = "conor-byrne-pub";
            let venue_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, venue_slug.as_bytes());

            // Now create the event with the linked artist IDs and proper venue ID
            let event = Event {
                id: None,
                title: title.clone(),
                event_day,
                start_time,
                event_url,
                description,
                event_image_url,
                venue_id,  // Use the proper venue ID, not nil!
                artist_ids: event_artist_ids,  // Link the artists!
                show_event: true,
                finalized: false,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_event_record(
                event, 
                provenance.clone(), 
                0.95,  // High confidence for Conor Byrne events
                "conor_byrne_event".to_string()
            ));
        }

        // Create the Conor Byrne venue only once with the same deterministic ID
        if self.venue_state.should_create_venue() {
            let venue_slug = "conor-byrne-pub";
            let venue_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, venue_slug.as_bytes());

            let venue = Venue {
                id: Some(venue_id),  // Set the deterministic UUID
                name: "Conor Byrne Pub".to_string(),
                name_lower: "conor byrne pub".to_string(),
                slug: venue_slug.to_string(),
                latitude: 47.6686,  // Conor Byrne's location in Ballard
                longitude: -122.3842,
                address: "5140 Ballard Ave NW".to_string(),
                postal_code: "98107".to_string(),
                city: "Seattle".to_string(),
                venue_url: Some("https://www.conorbyrnepub.com".to_string()),
                venue_image_url: None,
                description: Some("Historic Irish pub featuring live music in the heart of Ballard".to_string()),
                neighborhood: Some("Ballard".to_string()),
                show_venue: true,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_venue_record(
                venue, 
                provenance.clone(), 
                1.0,  // Maximum confidence for known venue
                "conor_byrne_venue_hardcoded".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "conor_byrne"
    }

    fn name(&self) -> &str {
        "Conor Byrne Pub Normalizer"
    }
}
