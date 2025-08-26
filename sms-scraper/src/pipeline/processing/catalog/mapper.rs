use std::sync::Arc;
use uuid::Uuid;

use crate::common::error::{Result, ScraperError};
use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::conflation::ConflatedRecord;
use crate::pipeline::processing::normalize::NormalizedEntity;

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

// Mapper traits per entity type
pub trait VenueMapper: Send + Sync {
    fn to_venue(&self, record: &ConflatedRecord) -> Result<Venue>;
}

pub trait EventMapper: Send + Sync {
    fn to_event(&self, record: &ConflatedRecord, venue_id: Uuid) -> Result<Event>;
}

pub trait ArtistMapper: Send + Sync {
    fn to_artist(&self, record: &ConflatedRecord) -> Result<Artist>;
}

// Default mapper implementations leveraging normalized/enriched data
pub struct DefaultVenueMapper;
impl VenueMapper for DefaultVenueMapper {
    fn to_venue(&self, record: &ConflatedRecord) -> Result<Venue> {
        let normalized = &record
            .enriched_record
            .quality_assessed_record
            .normalized_record;
        let enrichment = &record.enriched_record.enrichment;

        let mut venue = match &normalized.entity {
            NormalizedEntity::Venue(v) => v.clone(),
            _ => {
                return Err(ScraperError::MissingField(
                    "Expected Venue entity in normalized record".to_string(),
                ))
            }
        };

        if let Some(city) = &enrichment.city {
            venue.city = city.clone();
        }
        if let Some(district) = &enrichment.district {
            venue.neighborhood = Some(district.clone());
        }
        Ok(venue)
    }
}

pub struct DefaultEventMapper;
impl EventMapper for DefaultEventMapper {
    fn to_event(&self, record: &ConflatedRecord, venue_id: Uuid) -> Result<Event> {
        let normalized = &record
            .enriched_record
            .quality_assessed_record
            .normalized_record;

        let event = match &normalized.entity {
            NormalizedEntity::Event(e) => e.clone(),
            _ => {
                return Err(ScraperError::MissingField(
                    "Expected Event entity in normalized record".to_string(),
                ))
            }
        };

        let artist_ids = if event.artist_ids.is_empty() {
            Vec::new()
        } else {
            event.artist_ids.clone()
        };

        Ok(Event {
            id: None,
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
}

pub struct DefaultArtistMapper;
impl ArtistMapper for DefaultArtistMapper {
    fn to_artist(&self, record: &ConflatedRecord) -> Result<Artist> {
        let normalized = &record
            .enriched_record
            .quality_assessed_record
            .normalized_record;

        let artist = match &normalized.entity {
            NormalizedEntity::Artist(a) => a.clone(),
            _ => {
                return Err(ScraperError::MissingField(
                    "Expected Artist entity in normalized record".to_string(),
                ))
            }
        };
        Ok(artist)
    }
}

/// Small registry for mappers per entity type
#[derive(Clone)]
pub struct MapperRegistry {
    pub venue_mapper: Arc<dyn VenueMapper>,
    pub event_mapper: Arc<dyn EventMapper>,
    pub artist_mapper: Arc<dyn ArtistMapper>,
}

impl Default for MapperRegistry {
    fn default() -> Self {
        Self {
            venue_mapper: Arc::new(DefaultVenueMapper),
            event_mapper: Arc::new(DefaultEventMapper),
            artist_mapper: Arc::new(DefaultArtistMapper),
        }
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
