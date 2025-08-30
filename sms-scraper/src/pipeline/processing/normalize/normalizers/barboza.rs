use chrono::{NaiveDate, NaiveTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use sms_core::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for Barboza events
/// These are scraped from The Barboza's HTML events page
pub struct BarbozaNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl BarbozaNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl Default for BarbozaNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceNormalizer for BarbozaNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        let mut results = Vec::new();
        let data = &record.record;
        let provenance = NormalizerUtils::create_provenance(record);

        // Extract title
        if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
            // Parse event date from event_day field (already formatted as YYYY-MM-DD)
            let event_day = data.get("event_day")
                .and_then(|v| v.as_str())
                .and_then(|date_str| NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok())
                .unwrap_or_else(|| Utc::now().naive_utc().date());

            // Parse start time from event_time field
            let start_time = data.get("event_time")
                .and_then(|v| v.as_str())
                .and_then(|time_str| {
                    // Try common time formats (e.g., "7:00 PM", "19:00")
                    NaiveTime::parse_from_str(time_str, "%I:%M %p")
                        .or_else(|_| NaiveTime::parse_from_str(time_str, "%l:%M %p"))
                        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M"))
                        .ok()
                });

            // Build event URL from ticket_url or detail_url
            let event_url = data.get("ticket_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| data.get("detail_url")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        if s.starts_with("http") {
                            s.to_string()
                        } else {
                            format!("https://www.thebarboza.com{}", s)
                        }
                    }))
                .or_else(|| Some("https://www.thebarboza.com/events".to_string()));

            // Build description from available metadata
            let mut description_parts = Vec::new();
            
            if let Some(supporting) = data.get("supporting_acts").and_then(|v| v.as_str()) {
                description_parts.push(format!("With {}", supporting));
            }
            
            if let Some(promoter) = data.get("promoter").and_then(|v| v.as_str()) {
                description_parts.push(promoter.to_string());
            }
            
            if let Some(age) = data.get("age_restriction").and_then(|v| v.as_str()) {
                description_parts.push(format!("Age: {}", age));
            }
            
            let description = if !description_parts.is_empty() {
                Some(description_parts.join(" | "))
            } else {
                None
            };

            // Get image URL if available
            let event_image_url = data.get("image_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // We'll collect artist IDs as we create artists
            let mut event_artist_ids = Vec::new();
            
            // Helper function to create artist and track its ID
            let mut create_and_track_artist = |artist_name: &str, confidence: f64, strategy: String| -> Option<Uuid> {
                if artist_name.is_empty() || NormalizerUtils::is_non_artist_event(artist_name) {
                    return None;
                }
                
                let name_slug = NormalizerUtils::generate_slug(artist_name);
                if !self.artist_state.should_create_artist(&name_slug) {
                    // Artist already created, but we still need its ID
                    // Generate deterministic UUID from the slug
                    return Some(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name_slug.as_bytes()));
                }
                
                // Generate a deterministic UUID based on the artist slug
                let artist_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name_slug.as_bytes());
                
                let artist = Artist {
                    id: Some(artist_id),
                    name: artist_name.to_string(),
                    name_slug,
                    bio: None,
                    artist_image_url: None,
                    created_at: Utc::now(),
                };

                results.push(NormalizerUtils::create_artist_record(
                    artist, 
                    provenance.clone(), 
                    confidence,
                    strategy
                ));
                
                Some(artist_id)
            };

            // Extract artists from both title and supporting_acts
            // Check if title contains "Feat" or "featuring" patterns
            let title_has_featuring = title.to_lowercase().contains("feat");
            
            tracing::debug!("Processing title '{}', has_featuring: {}", title, title_has_featuring);
            
            if title_has_featuring {
                // Parse the main artist and featured artists separately
                // Find where "Feat" starts in the title
                let lower_title = title.to_lowercase();
                let feat_pos = lower_title.find("feat").unwrap_or(title.len());
                
                // Extract main artist (everything before "Feat")
                if feat_pos > 0 {
                    let main_artist = title[..feat_pos].trim().trim_end_matches('!');
                    tracing::debug!("Main artist from title: '{}'", main_artist);
                    if let Some(artist_id) = create_and_track_artist(
                        main_artist, 
                        0.9, 
                        "barboza_artist_headliner".to_string()
                    ) {
                        tracing::debug!("Created main artist with ID: {}", artist_id);
                        event_artist_ids.push(artist_id);
                    }
                }
                
                // Extract featured artists from the title
                if feat_pos < title.len() {
                    // Find the end of "Feat" or "featuring" word and skip it
                    let feat_part = &title[feat_pos..];
                    let artists_start = feat_part
                        .find(|c: char| c.is_alphabetic())
                        .and_then(|idx| {
                            // Skip past "Feat" or "featuring"
                            let word_end = feat_part[idx..].find(|c: char| !c.is_alphabetic()).unwrap_or(feat_part.len() - idx);
                            feat_part[idx + word_end..].find(|c: char| c.is_alphabetic()).map(|i| idx + word_end + i)
                        })
                        .unwrap_or(feat_part.len());
                    
                    let artists_part = feat_part[artists_start..].trim();
                    
                    // Split by common delimiters
                    let featured_artists: Vec<&str> = artists_part
                        .split(&[',', '&', '+'][..])
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    
                    for featured_artist in featured_artists {
                        tracing::debug!("Featured artist: '{}'", featured_artist);
                        if let Some(artist_id) = create_and_track_artist(
                            featured_artist, 
                            0.85, 
                            "barboza_artist_featured".to_string()
                        ) {
                            tracing::debug!("Created featured artist with ID: {}", artist_id);
                            event_artist_ids.push(artist_id);
                        }
                    }
                }
            } else {
                // No "Feat" in title, treat the whole title as artist name
                if let Some(artist_id) = create_and_track_artist(
                    title, 
                    0.9, 
                    "barboza_artist_headliner".to_string()
                ) {
                    event_artist_ids.push(artist_id);
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
                    if let Some(artist_id) = create_and_track_artist(
                        act, 
                        0.85, 
                        "barboza_artist_supporting".to_string()
                    ) {
                        event_artist_ids.push(artist_id);
                    }
                }
            }
            
            // Now create the event with the linked artist IDs
            tracing::debug!("Event '{}' linked to {} artists: {:?}", title, event_artist_ids.len(), event_artist_ids);
            let event = Event {
                id: None,
                title: title.to_string(),
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
                0.95,  // High confidence for Barboza events
                "barboza_event".to_string()
            ));
        }

        // Create The Barboza venue only once
        if self.venue_state.should_create_venue() {
            let venue = Venue {
                id: None,
                name: "The Barboza".to_string(),
                name_lower: "the barboza".to_string(),
                slug: "the-barboza".to_string(),
                latitude: 47.6133,  // The Barboza's location in Capitol Hill
                longitude: -122.3185,
                address: "925 E Pike St".to_string(),
                postal_code: "98122".to_string(),
                city: "Seattle".to_string(),
                venue_url: Some("https://www.thebarboza.com".to_string()),
                venue_image_url: None,
                description: Some("Underground music venue in Capitol Hill featuring live performances and DJ nights".to_string()),
                neighborhood: Some("Capitol Hill".to_string()),
                show_venue: true,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_venue_record(
                venue, 
                provenance.clone(), 
                1.0,  // Maximum confidence for known venue
                "barboza_venue_hardcoded".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "barboza"
    }

    fn name(&self) -> &str {
        "Barboza Events Normalizer"
    }
}
