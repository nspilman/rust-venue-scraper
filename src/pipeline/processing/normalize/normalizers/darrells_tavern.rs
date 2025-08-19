use chrono::{NaiveDate, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for Darrell's Tavern events
/// These have minimal data - just title and event_day
pub struct DarrellsTavernNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl DarrellsTavernNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl SourceNormalizer for DarrellsTavernNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let data = &record.record;
        let provenance = NormalizerUtils::create_provenance(record);

        // Extract event
        if let Some(title) = NormalizerUtils::extract_title(data) {
            let event_day = data.get("event_day")
                .and_then(|v| v.as_str())
                .and_then(|date_str| NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok())
                .unwrap_or_else(|| Utc::now().naive_utc().date());

            let event = Event {
                id: None,
                title: title.clone(),
                event_day,
                start_time: None, // Not available in source data
                event_url: Some("https://www.darrellstavern.com".to_string()),
                description: None,
                event_image_url: None,
                venue_id: Uuid::nil(),
                artist_ids: Vec::new(),
                show_event: true,
                finalized: false,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_event_record(
                event, 
                provenance.clone(), 
                0.9, 
                "darrells_event".to_string()
            ));

            // Extract artist from title, but only create if not already seen
            // The title is typically the artist/band name for Darrell's
            let name_slug = NormalizerUtils::generate_slug(&title);
            if self.artist_state.should_create_artist(&name_slug) {
                let artist = Artist {
                    id: None,
                    name: title.clone(),
                    name_slug,
                    bio: None,
                    artist_image_url: None,
                    created_at: Utc::now(),
                };

                results.push(NormalizerUtils::create_artist_record(
                    artist, 
                    provenance.clone(), 
                    0.85, 
                    "darrells_artist".to_string()
                ));
            }
        }

        // Create the venue only once for Darrell's Tavern
        // Use the venue state manager to ensure thread safety
        if self.venue_state.should_create_venue() {
            let venue = Venue {
                id: None,
                name: "Darrell's Tavern".to_string(),
                name_lower: "darrell's tavern".to_string(),
                slug: "darrells-tavern".to_string(),
                latitude: 47.6780, // Approximate location in Shoreline
                longitude: -122.3460,
                address: "18041 Aurora Ave N".to_string(),
                postal_code: "98133".to_string(),
                city: "Shoreline".to_string(),
                venue_url: Some("https://www.darrellstavern.com".to_string()),
                venue_image_url: None,
                description: Some("Live music venue in Shoreline".to_string()),
                neighborhood: Some("Shoreline".to_string()),
                show_venue: true,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_venue_record(
                venue, 
                provenance.clone(), 
                1.0, // We know this is Darrell's
                "darrells_venue_hardcoded".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "darrells_tavern"
    }

    fn name(&self) -> &str {
        "Darrell's Tavern Normalizer"
    }
}

impl Default for DarrellsTavernNormalizer {
    fn default() -> Self {
        Self::new()
    }
}
