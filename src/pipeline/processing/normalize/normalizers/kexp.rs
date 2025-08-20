use chrono::{NaiveDate, NaiveTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for KEXP events
/// These are scraped from KEXP's HTML events page
pub struct KexpNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl KexpNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl Default for KexpNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceNormalizer for KexpNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let data = &record.record;
        let provenance = NormalizerUtils::create_provenance(record);

        // Extract event from title
        if let Some(title) = NormalizerUtils::extract_title(data) {
            // Parse event date from date field
            let event_day = data.get("date")
                .and_then(|v| v.as_str())
                .and_then(|date_str| {
                    // Try to parse various date formats that might come from KEXP HTML
                    NaiveDate::parse_from_str(date_str, "%B %d, %Y")
                        .or_else(|_| NaiveDate::parse_from_str(date_str, "%m/%d/%Y"))
                        .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y-%m-%d"))
                        .ok()
                })
                .unwrap_or_else(|| Utc::now().naive_utc().date());

            // Parse start time from time field
            let start_time = data.get("time")
                .and_then(|v| v.as_str())
                .and_then(|time_str| {
                    // Try common time formats
                    NaiveTime::parse_from_str(time_str, "%I:%M %p")
                        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M"))
                        .ok()
                });

            // Get location for event URL construction or description
            let location = data.get("location")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let description = data.get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| location.clone()); // Use location as description if no description

            let event = Event {
                id: None,
                title: title.clone(),
                event_day,
                start_time,
                event_url: Some("https://www.kexp.org/events/".to_string()),
                description,
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
                "kexp_event".to_string()
            ));

            // Extract artist from title (excluding known non-artist events), but only create if not already seen
            if !NormalizerUtils::is_non_artist_event(&title) {
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
                        "kexp_artist_from_title".to_string()
                    ));
                }
            }
        }

        // Create the KEXP venue only once
        // Use the venue state manager to ensure thread safety
        if self.venue_state.should_create_venue() {
            let venue = Venue {
                id: None,
                name: "KEXP Events".to_string(),
                name_lower: "kexp events".to_string(),
                slug: "kexp-events".to_string(),
                latitude: 47.6205, // KEXP location in Lower Queen Anne
                longitude: -122.3493,
                address: "472 1st Ave N".to_string(),
                postal_code: "98109".to_string(),
                city: "Seattle".to_string(),
                venue_url: Some("https://www.kexp.org/events/".to_string()),
                venue_image_url: None,
                description: Some("KEXP Radio Station and live event venue in Lower Queen Anne".to_string()),
                neighborhood: Some("Lower Queen Anne".to_string()),
                show_venue: true,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_venue_record(
                venue, 
                provenance.clone(), 
                1.0, 
                "kexp_venue_hardcoded".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "kexp"
    }

    fn name(&self) -> &str {
        "KEXP Events Normalizer"
    }
}
