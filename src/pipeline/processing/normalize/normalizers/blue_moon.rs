use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for Blue Moon Tavern events  
pub struct BlueMoonNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl BlueMoonNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl SourceNormalizer for BlueMoonNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let data = &record.record;
        let provenance = NormalizerUtils::create_provenance(record);

        // Blue Moon data structure is similar to Sea Monster (Wix calendar)
        // Extract event
        if let Some(title) = NormalizerUtils::extract_title(data) {
            let event_day = data.get("scheduling")
                .and_then(|s| s.get("startDate"))
                .and_then(|v| v.as_str())
                .and_then(|date_str| {
                    // Try to parse ISO format first
                    DateTime::parse_from_rfc3339(date_str)
                        .map(|dt| dt.naive_utc().date())
                        .ok()
                })
                .unwrap_or_else(|| Utc::now().naive_utc().date());

            let event = Event {
                id: None,
                title: title.clone(),
                event_day,
                start_time: None,
                event_url: Some("https://www.bluemoonseattle.com".to_string()),
                description: data.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
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
                "blue_moon_event".to_string()
            ));

            // Extract artist from title, but only create if not already seen
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
                    "blue_moon_artist".to_string()
                ));
            }
        }

        // Create the Blue Moon venue only once
        // Use the venue state manager to ensure thread safety
        if self.venue_state.should_create_venue() {
            let venue = Venue {
                id: None,
                name: "Blue Moon Tavern".to_string(),
                name_lower: "blue moon tavern".to_string(),
                slug: "blue-moon-tavern".to_string(),
                latitude: 47.6608, // U-District location
                longitude: -122.3126,
                address: "712 NE 45th St".to_string(),
                postal_code: "98105".to_string(),
                city: "Seattle".to_string(),
                venue_url: Some("https://www.bluemoonseattle.com".to_string()),
                venue_image_url: None,
                description: Some("Historic tavern and live music venue in the University District".to_string()),
                neighborhood: Some("University District".to_string()),
                show_venue: true,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_venue_record(
                venue, 
                provenance.clone(), 
                1.0, 
                "blue_moon_venue_hardcoded".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "blue_moon"
    }

    fn name(&self) -> &str {
        "Blue Moon Tavern Normalizer"
    }
}

impl Default for BlueMoonNormalizer {
    fn default() -> Self {
        Self::new()
    }
}
