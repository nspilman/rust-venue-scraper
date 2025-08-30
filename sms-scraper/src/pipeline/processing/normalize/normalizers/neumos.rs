use chrono::{NaiveDate, NaiveTime, Utc};
use uuid::Uuid;
use anyhow::Result;

use super::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use sms_core::domain::{Artist, Event, Venue};
use crate::pipeline::processing::parser::ParsedRecord;
use crate::pipeline::processing::normalize::NormalizedRecord;

/// Normalizer for Neumos events
/// These are scraped from Neumos' HTML events page
pub struct NeumosNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl NeumosNormalizer {
    pub fn new() -> Self {
        Self {
            venue_state: VenueStateManager::new(),
            artist_state: ArtistStateManager::new(),
        }
    }
}

impl Default for NeumosNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceNormalizer for NeumosNormalizer {
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
                            format!("https://www.neumos.com{}", s)
                        }
                    }))
                .or_else(|| Some("https://www.neumos.com/events".to_string()));

            // Build description from available metadata
            let mut description_parts = Vec::new();
            
            // Add tour name if present
            if let Some(tour) = data.get("tour_name").and_then(|v| v.as_str()) {
                description_parts.push(format!("Tour: {}", tour));
            }
            
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
            // Check if title contains "with" patterns (common in Neumos listings)
            let title_has_with = title.to_lowercase().contains(" with ");
            
            tracing::debug!("Processing title '{}', has_with: {}", title, title_has_with);
            
            if title_has_with {
                // Parse the main artist and supporting artists separately
                let lower_title = title.to_lowercase();
                let with_pos = lower_title.find(" with ").unwrap_or(title.len());
                
                // Extract main artist (everything before "with")
                if with_pos > 0 {
                    let main_artist = title[..with_pos].trim();
                    tracing::debug!("Main artist from title: '{}'", main_artist);
                    if let Some(artist_id) = create_and_track_artist(
                        main_artist, 
                        0.9, 
                        "neumos_artist_headliner".to_string()
                    ) {
                        tracing::debug!("Created main artist with ID: {}", artist_id);
                        event_artist_ids.push(artist_id);
                    }
                }
                
                // Extract supporting artists from the title
                if with_pos < title.len() {
                    let artists_part = &title[with_pos + 6..]; // Skip " with "
                    
                    // Split by common delimiters
                    let supporting_artists: Vec<&str> = artists_part
                        .split(&[',', '&', '+'][..])
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    
                    for supporting_artist in supporting_artists {
                        tracing::debug!("Supporting artist from title: '{}'", supporting_artist);
                        if let Some(artist_id) = create_and_track_artist(
                            supporting_artist, 
                            0.85, 
                            "neumos_artist_supporting".to_string()
                        ) {
                            tracing::debug!("Created supporting artist with ID: {}", artist_id);
                            event_artist_ids.push(artist_id);
                        }
                    }
                }
            } else {
                // No "with" in title, treat the whole title as artist name
                if let Some(artist_id) = create_and_track_artist(
                    title, 
                    0.9, 
                    "neumos_artist_headliner".to_string()
                ) {
                    tracing::debug!("Created headliner artist with ID: {}", artist_id);
                    event_artist_ids.push(artist_id);
                }
            }
            
            // Also process supporting_acts field if present
            if let Some(supporting_acts) = data.get("supporting_acts").and_then(|v| v.as_str()) {
                // Split by common delimiters
                let supporting_artists: Vec<&str> = supporting_acts
                    .split(&[',', '&', '+'][..])
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                for supporting_artist in supporting_artists {
                    tracing::debug!("Supporting artist from field: '{}'", supporting_artist);
                    if let Some(artist_id) = create_and_track_artist(
                        supporting_artist, 
                        0.85, 
                        "neumos_artist_supporting_field".to_string()
                    ) {
                        tracing::debug!("Created supporting artist with ID: {}", artist_id);
                        event_artist_ids.push(artist_id);
                    }
                }
            }

            // Generate a deterministic UUID for Neumos venue based on its slug
            let venue_slug = "neumos";
            let venue_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, venue_slug.as_bytes());
            
            // Create the venue (only if not already created)
            if self.venue_state.should_create_venue() {
                let venue = Venue {
                    id: Some(venue_id),  // Use deterministic UUID
                    name: "Neumos".to_string(),
                    name_lower: "neumos".to_string(),
                    slug: venue_slug.to_string(),
                    latitude: 47.614746,  // Neumos' location on Capitol Hill
                    longitude: -122.319532,
                    address: "925 E Pike St".to_string(),
                    postal_code: "98122".to_string(),
                    city: "Seattle".to_string(),
                    venue_url: Some("https://www.neumos.com".to_string()),
                    venue_image_url: None,
                    description: Some("Legendary Capitol Hill music venue featuring live bands and DJ nights".to_string()),
                    neighborhood: Some("Capitol Hill".to_string()),
                    show_venue: true,
                    created_at: Utc::now(),
                };

                results.push(NormalizerUtils::create_venue_record(
                    venue, 
                    provenance.clone(),
                    0.95, // High confidence for venue data
                    "neumos_venue_creation".to_string()
                ));
            }

            // Create the event with the venue ID properly linked
            let event = Event {
                id: None,
                title: title.to_string(),
                event_day,
                start_time,
                event_url,
                description,
                event_image_url,
                venue_id,  // Use the deterministic venue ID
                artist_ids: event_artist_ids,
                show_event: true,
                finalized: false,
                created_at: Utc::now(),
            };

            results.push(NormalizerUtils::create_event_record(
                event, 
                provenance,
                0.9, // High confidence for event data
                "neumos_event_creation".to_string()
            ));
        }

        Ok(results)
    }

    fn source_id(&self) -> &str {
        "neumos"
    }

    fn name(&self) -> &str {
        "Neumos Events Normalizer"
    }
}
