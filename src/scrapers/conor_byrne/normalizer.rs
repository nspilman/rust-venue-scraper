use async_trait::async_trait;
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::US::Pacific;
use uuid::Uuid;

use crate::{
    errors::ScraperError,
    models::{
        artist::Artist,
        event::Event,
        raw_event::RawEvent,
        venue::Venue,
    },
    scrapers::traits::VenueNormalizer,
    utils::id_generator::generate_deterministic_id,
};

pub struct ConorByrneNormalizer;

impl ConorByrneNormalizer {
    pub fn new() -> Self {
        Self
    }

    fn get_venue_id() -> Uuid {
        generate_deterministic_id("conor-byrne")
    }

    fn create_venue() -> Venue {
        Venue {
            id: Self::get_venue_id(),
            name: "Conor Byrne Pub".to_string(),
            address: Some("5140 Ballard Ave NW".to_string()),
            city: Some("Seattle".to_string()),
            state: Some("WA".to_string()),
            zip: Some("98107".to_string()),
            country: Some("USA".to_string()),
            url: Some("https://www.conorbyrnepub.com".to_string()),
            phone: Some("206-784-3640".to_string()),
            capacity: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

#[async_trait]
impl VenueNormalizer for ConorByrneNormalizer {
    async fn normalize_events(
        &self,
        raw_events: Vec<RawEvent>,
    ) -> Result<(Vec<Event>, Vec<Venue>, Vec<Artist>), ScraperError> {
        let venue_id = Self::get_venue_id();
        let venue = Self::create_venue();

        let mut events = Vec::new();
        let mut artists_map = std::collections::HashMap::new();

        for raw_event in raw_events {
            // Parse datetime
            let datetime = if let Some(time_str) = &raw_event.time {
                NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| ScraperError::ParseError(format!("Failed to parse datetime: {}", e)))?
            } else {
                // If no time provided, default to 7 PM
                raw_event.date.and_hms_opt(19, 0, 0)
                    .ok_or_else(|| ScraperError::ParseError("Failed to create default datetime".to_string()))?
            };

            // Convert to UTC (assuming events are in Pacific time)
            let pacific_datetime = Pacific
                .from_local_datetime(&datetime)
                .single()
                .ok_or_else(|| ScraperError::ParseError("Failed to convert to Pacific time".to_string()))?;
            let utc_datetime = pacific_datetime.with_timezone(&chrono::Utc);

            // Generate event ID based on title and date
            let event_id = generate_deterministic_id(&format!(
                "conor-byrne-{}-{}",
                raw_event.title.to_lowercase().replace(' ', "-"),
                raw_event.date
            ));

            // Collect artist IDs
            let mut artist_ids = Vec::new();
            for raw_artist in &raw_event.artists {
                let artist_id = generate_deterministic_id(&format!(
                    "artist-{}",
                    raw_artist.name.to_lowercase().replace(' ', "-")
                ));
                artist_ids.push(artist_id);

                // Add artist to map if not already present
                artists_map.entry(artist_id).or_insert_with(|| Artist {
                    id: artist_id,
                    name: raw_artist.name.clone(),
                    genre: None,
                    url: raw_artist.url.clone(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                });
            }

            // Parse price if it's a numeric value
            let price = raw_event.price.as_ref().and_then(|p| {
                // Try to extract numeric price from strings like "$15" or "15"
                let cleaned = p.replace("$", "").replace(",", "").trim().to_string();
                cleaned.parse::<f64>().ok()
            });

            let event = Event {
                id: event_id,
                venue_id,
                title: raw_event.title.clone(),
                date: utc_datetime,
                artist_ids,
                description: raw_event.description.clone(),
                ticket_link: raw_event.ticket_link.clone(),
                price,
                age_restriction: raw_event.age_restriction.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            events.push(event);
        }

        let artists: Vec<Artist> = artists_map.into_values().collect();

        Ok((events, vec![venue], artists))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::models::raw_event::RawArtist;

    #[tokio::test]
    async fn test_normalize_events() {
        let normalizer = ConorByrneNormalizer::new();

        let raw_events = vec![
            RawEvent {
                title: "Test Event".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                time: Some("2025-01-15 20:00:00".to_string()),
                artists: vec![
                    RawArtist {
                        name: "Test Artist 1".to_string(),
                        url: Some("https://example.com/artist1".to_string()),
                    },
                    RawArtist {
                        name: "Test Artist 2".to_string(),
                        url: None,
                    },
                ],
                venue_name: "Conor Byrne Pub".to_string(),
                description: Some("Test description".to_string()),
                ticket_link: Some("https://example.com/tickets".to_string()),
                price: Some("$15".to_string()),
                age_restriction: Some("21+".to_string()),
                additional_info: None,
            },
        ];

        let result = normalizer.normalize_events(raw_events).await;
        assert!(result.is_ok());

        let (events, venues, artists) = result.unwrap();
        
        assert_eq!(events.len(), 1);
        assert_eq!(venues.len(), 1);
        assert_eq!(artists.len(), 2);

        // Check event
        let event = &events[0];
        assert_eq!(event.title, "Test Event");
        assert_eq!(event.venue_id, ConorByrneNormalizer::get_venue_id());
        assert_eq!(event.artist_ids.len(), 2);
        assert_eq!(event.price, Some(15.0));
        assert_eq!(event.age_restriction, Some("21+".to_string()));

        // Check venue
        let venue = &venues[0];
        assert_eq!(venue.name, "Conor Byrne Pub");
        assert_eq!(venue.city, Some("Seattle".to_string()));

        // Check artists
        let artist_names: Vec<String> = artists.iter().map(|a| a.name.clone()).collect();
        assert!(artist_names.contains(&"Test Artist 1".to_string()));
        assert!(artist_names.contains(&"Test Artist 2".to_string()));
    }
}
