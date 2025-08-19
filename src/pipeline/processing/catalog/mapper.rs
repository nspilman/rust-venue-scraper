use uuid::Uuid;

use crate::common::error::{Result, ScraperError};
use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::conflation::ConflatedRecord;
use crate::pipeline::processing::normalize::NormalizedEntity;

/// Maps conflated records to domain entities
pub struct EntityMapper;

impl EntityMapper {
    /// Build a Venue from a conflated record
    pub fn map_to_venue(conflated_record: &ConflatedRecord) -> Result<Venue> {
        let normalized = &conflated_record.enriched_record.quality_assessed_record.normalized_record;
        let enrichment = &conflated_record.enriched_record.enrichment;
        
        // Extract the venue from the normalized entity
        let venue = match &normalized.entity {
            NormalizedEntity::Venue(v) => v.clone(),
            _ => return Err(ScraperError::MissingField(
                "Expected Venue entity in normalized record".to_string()
            )),
        };
        
        // Update with enrichment data if available
        let mut result = venue;
        if let Some(city) = &enrichment.city {
            result.city = city.clone();
        }
        if let Some(district) = &enrichment.district {
            result.neighborhood = Some(district.clone());
        }
        
        Ok(result)
    }

    /// Build an Event from a conflated record
    pub fn map_to_event(
        conflated_record: &ConflatedRecord,
        venue_id: Uuid
    ) -> Result<Event> {
        let normalized = &conflated_record.enriched_record.quality_assessed_record.normalized_record;
        
        // Extract the event from the normalized entity
        let event = match &normalized.entity {
            NormalizedEntity::Event(e) => e.clone(),
            _ => return Err(ScraperError::MissingField(
                "Expected Event entity in normalized record".to_string()
            )),
        };
        
        // The event's artist_ids need to be resolved/created
        let artist_ids = if event.artist_ids.is_empty() {
            // Try to extract from other sources if available
            Vec::new()
        } else {
            event.artist_ids.clone()
        };
        
        Ok(Event {
            id: None, // Will be set by storage
            title: event.title.clone(),
            event_day: event.event_day,
            start_time: event.start_time,
            event_url: event.event_url.clone(),
            description: event.description.clone(),
            event_image_url: event.event_image_url.clone(),
            venue_id,
            artist_ids,
            show_event: event.show_event,
            finalized: event.finalized,
            created_at: event.created_at,
        })
    }

    /// Build an Artist from a conflated record
    pub fn map_to_artist(conflated_record: &ConflatedRecord) -> Result<Artist> {
        let normalized = &conflated_record.enriched_record.quality_assessed_record.normalized_record;
        
        // Extract the artist from the normalized entity
        let artist = match &normalized.entity {
            NormalizedEntity::Artist(a) => a.clone(),
            _ => return Err(ScraperError::MissingField(
                "Expected Artist entity in normalized record".to_string()
            )),
        };
        
        Ok(artist)
    }

    /// Extract the normalized entity from a conflated record
    pub fn extract_entity(conflated_record: &ConflatedRecord) -> &NormalizedEntity {
        &conflated_record.enriched_record.quality_assessed_record.normalized_record.entity
    }
}

/// Utility functions for entity processing
pub struct EntityUtils;

impl EntityUtils {
    /// Generate a URL-friendly slug from a name
    pub fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_generation() {
        assert_eq!(EntityUtils::generate_slug("The Crocodile"), "the-crocodile");
        assert_eq!(EntityUtils::generate_slug("Blue Moon Tavern"), "blue-moon-tavern");
        assert_eq!(EntityUtils::generate_slug("Rock & Roll Club"), "rock-roll-club");
        assert_eq!(EntityUtils::generate_slug("  Spaces  Between  "), "spaces-between");
    }
}
