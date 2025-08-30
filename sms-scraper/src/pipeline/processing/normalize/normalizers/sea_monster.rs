use chrono::{NaiveDate, NaiveTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use sms_core::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for Sea Monster Lounge events
/// These have rich location data in nested JSON structure
pub struct SeaMonsterNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl SeaMonsterNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl Default for SeaMonsterNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceNormalizer for SeaMonsterNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let data = &record.record;
        let provenance = NormalizerUtils::create_provenance(record);

        // Extract event from title and scheduling fields
        if let Some(title) = NormalizerUtils::extract_title(data) {
            // Parse event date from scheduling.startDateFormatted
            let event_day = data.get("scheduling")
                .and_then(|s| s.get("startDateFormatted"))
                .and_then(|v| v.as_str())
                .and_then(|date_str| {
                    NaiveDate::parse_from_str(date_str, "%B %d, %Y").ok()
                })
                .unwrap_or_else(|| Utc::now().naive_utc().date());

            // Parse start time
            let start_time = data.get("scheduling")
                .and_then(|s| s.get("startTimeFormatted"))
                .and_then(|v| v.as_str())
                .and_then(|time_str| {
                    NaiveTime::parse_from_str(time_str, "%I:%M %p").ok()
                });

            let event_url = data.get("slug")
                .and_then(|v| v.as_str())
                .map(|slug| format!("https://www.seamonsterlounge.com/event-info/{}", slug));

            let description = data.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let event_image_url = data.get("mainImage")
                .and_then(|i| i.get("url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Collect artist IDs as we create artists
            let mut event_artist_ids = Vec::new();
            
            // Extract artist from title (excluding known non-artist events)
            if !NormalizerUtils::is_non_artist_event(&title) && !title.to_lowercase().contains("la luz") {
                let name_slug = NormalizerUtils::generate_slug(&title);
                
                // Generate deterministic UUID from the artist slug
                let artist_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name_slug.as_bytes());
                
                // Track this artist ID for the event
                event_artist_ids.push(artist_id);
                
                // Only create the artist entity if we haven't seen it before
                if self.artist_state.should_create_artist(&name_slug) {
                    let artist = Artist {
                        id: Some(artist_id),  // Use the deterministic ID
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
                        "sea_monster_artist_from_title".to_string()
                    ));
                }
            }

            // Now create the event with the linked artist IDs
            let event = Event {
                id: None,
                title: title.clone(),
                event_day,
                start_time,
                event_url,
                description,
                event_image_url,
                venue_id: Uuid::nil(),
                artist_ids: event_artist_ids,  // Link the artists!
                show_event: true,
                finalized: false,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_event_record(
                event, 
                provenance.clone(), 
                0.95, 
                "sea_monster_event".to_string()
            ));
        }

        // Create the Sea Monster venue only once
        // Use the venue state manager to ensure thread safety
        if self.venue_state.should_create_venue() {
            // Try to extract venue details from location field if available, otherwise use defaults
            let (name, address, city, postal_code, neighborhood, latitude, longitude) = 
                if let Some(location) = data.get("location").and_then(|l| l.as_object()) {
                    let name = location.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Sea Monster Lounge")
                        .to_string();

                    // Extract detailed address information
                    let (address, city, postal_code, neighborhood) = 
                        if let Some(full_addr) = location.get("fullAddress").and_then(|f| f.as_object()) {
                            let address = full_addr.get("formattedAddress")
                                .and_then(|v| v.as_str())
                                .unwrap_or("2202 N 45th St, Seattle, WA 98103")
                                .to_string();
                            let city = full_addr.get("city")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Seattle")
                                .to_string();
                            let postal_code = full_addr.get("postalCode")
                                .and_then(|v| v.as_str())
                                .unwrap_or("98103")
                                .to_string();
                            let neighborhood = full_addr.get("subdivisions")
                                .and_then(|s| s.as_array())
                                .and_then(|arr| arr.iter().find(|s| s["type"] == 4))
                                .and_then(|s| s.get("name"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or(Some("Wallingford".to_string()));
                            (address, city, postal_code, neighborhood)
                        } else {
                            ("2202 N 45th St, Seattle, WA 98103".to_string(), 
                             "Seattle".to_string(), 
                             "98103".to_string(), 
                             Some("Wallingford".to_string()))
                        };

                    // Extract coordinates
                    let (latitude, longitude) = if let Some(coords) = location.get("coordinates").and_then(|c| c.as_object()) {
                        (
                            coords.get("lat").and_then(|v| v.as_f64()).unwrap_or(47.6615064),
                            coords.get("lng").and_then(|v| v.as_f64()).unwrap_or(-122.3323427),
                        )
                    } else {
                        (47.6615064, -122.3323427) // Sea Monster default coords
                    };

                    (name, address, city, postal_code, neighborhood, latitude, longitude)
                } else {
                    // Default values if no location data is available
                    ("Sea Monster Lounge".to_string(),
                     "2202 N 45th St, Seattle, WA 98103".to_string(),
                     "Seattle".to_string(),
                     "98103".to_string(),
                     Some("Wallingford".to_string()),
                     47.6615064,
                     -122.3323427)
                };

            let name_lower = name.to_lowercase();
            let slug = NormalizerUtils::generate_slug(&name);

            let venue = Venue {
                id: None,
                name,
                name_lower,
                slug,
                latitude,
                longitude,
                address,
                postal_code,
                city,
                venue_url: Some("https://www.seamonsterlounge.com".to_string()),
                venue_image_url: None,
                description: Some("Live music venue in Wallingford".to_string()),
                neighborhood,
                show_venue: true,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_venue_record(
                venue, 
                provenance.clone(), 
                0.95, 
                "sea_monster_venue".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "sea_monster"
    }

    fn name(&self) -> &str {
        "Sea Monster Lounge Normalizer"
    }
}
