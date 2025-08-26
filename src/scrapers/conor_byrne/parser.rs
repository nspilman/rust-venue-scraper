use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    errors::ScraperError,
    models::raw_event::{RawArtist, RawEvent},
    scrapers::traits::VenueParser,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct GraphQLResponse {
    pub data: GraphQLData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GraphQLData {
    #[serde(rename = "paginatedEvents")]
    pub paginated_events: PaginatedEvents,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaginatedEvents {
    pub collection: Vec<ConorByrneEvent>,
    pub metadata: EventMetadata,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventMetadata {
    #[serde(rename = "currentPage")]
    pub current_page: Option<i32>,
    #[serde(rename = "limitValue")]
    pub limit_value: Option<i32>,
    #[serde(rename = "totalCount")]
    pub total_count: Option<i32>,
    #[serde(rename = "totalPages")]
    pub total_pages: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConorByrneEvent {
    pub id: i64,
    pub name: String,
    pub date: String,
    #[serde(rename = "doorTime")]
    pub door_time: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "minimumAge")]
    pub minimum_age: Option<i32>,
    pub promoter: Option<String>,
    pub support: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    #[serde(rename = "twitterUrl")]
    pub twitter_url: Option<String>,
    #[serde(rename = "instagramUrl")]
    pub instagram_url: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "announceArtists")]
    pub announce_artists: Option<Vec<AnnounceArtist>>,
    pub artists: Option<Vec<Artist>>,
    pub venue: Option<Venue>,
    #[serde(rename = "footerContent")]
    pub footer_content: Option<String>,
    #[serde(rename = "ticketsUrl")]
    pub tickets_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnnounceArtist {
    pub name: String,
    pub applemusic: Option<String>,
    pub bandcamp: Option<String>,
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub lastfm: Option<String>,
    pub songkick: Option<String>,
    pub spotify: Option<String>,
    pub twitter: Option<String>,
    pub website: Option<String>,
    pub wikipedia: Option<String>,
    pub youtube: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub bio: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Venue {
    pub name: String,
}

pub struct ConorByrneParser;

impl ConorByrneParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VenueParser for ConorByrneParser {
    async fn parse_events(&self, json_data: &str) -> Result<Vec<RawEvent>, ScraperError> {
        // Parse the GraphQL response
        let response: GraphQLResponse = serde_json::from_str(json_data)
            .map_err(|e| ScraperError::ParseError(format!("Failed to parse JSON: {}", e)))?;

        let mut raw_events = Vec::new();

        for event in response.data.paginated_events.collection {
            // Parse date
            let date = NaiveDate::parse_from_str(&event.date, "%Y-%m-%d")
                .map_err(|e| ScraperError::ParseError(format!("Failed to parse date: {}", e)))?;

            // Combine door time with date for full datetime
            let datetime_str = if let Some(door_time) = &event.door_time {
                format!("{} {}", event.date, door_time)
            } else if let Some(start_time) = &event.start_time {
                format!("{} {}", event.date, start_time)
            } else {
                format!("{} 19:00:00", event.date) // Default to 7 PM if no time specified
            };

            // Collect all artists
            let mut artists = Vec::new();
            
            // Add artists from the artists field
            if let Some(event_artists) = event.artists {
                for artist in event_artists {
                    artists.push(RawArtist {
                        name: artist.name,
                        url: None,
                    });
                }
            }

            // Add artists from announce_artists field if no regular artists
            if artists.is_empty() {
                if let Some(announce_artists) = event.announce_artists {
                    for artist in announce_artists {
                        artists.push(RawArtist {
                            name: artist.name,
                            url: artist.website.or(artist.spotify).or(artist.bandcamp),
                        });
                    }
                }
            }

            // Extract ticket link
            let ticket_link = event.tickets_url.or_else(|| {
                // Try to extract from status if it contains a URL
                event.status.as_ref().and_then(|s| {
                    if s.contains("http") {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
            });

            // Clean up HTML from description
            let description = event.description.map(|desc| {
                // Basic HTML stripping - you might want to use a proper HTML parser
                desc.replace("<p>", "")
                    .replace("</p>", "\n")
                    .replace("<br>", "\n")
                    .replace("&nbsp;", " ")
                    .replace("&amp;", "&")
                    .trim()
                    .to_string()
            });

            let raw_event = RawEvent {
                title: event.name,
                date,
                time: Some(datetime_str),
                artists,
                venue_name: event.venue.map(|v| v.name).unwrap_or_else(|| "Conor Byrne Pub".to_string()),
                description,
                ticket_link,
                price: event.status.filter(|s| s != "Tickets" && s != "Free"),
                age_restriction: event.minimum_age.map(|age| format!("{}+", age)),
                additional_info: None,
            };

            raw_events.push(raw_event);
        }

        Ok(raw_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_events() {
        let sample_json = r#"{
            "data": {
                "paginatedEvents": {
                    "collection": [
                        {
                            "id": 122860,
                            "name": "Test Event",
                            "date": "2025-01-01",
                            "doorTime": "17:00:00",
                            "startTime": "17:00:00",
                            "endTime": "19:00:00",
                            "minimumAge": 21,
                            "promoter": null,
                            "support": "",
                            "description": "<p>Test description</p>",
                            "websiteUrl": null,
                            "twitterUrl": null,
                            "instagramUrl": null,
                            "status": "Free",
                            "announceArtists": [
                                {
                                    "name": "Test Artist",
                                    "applemusic": null,
                                    "bandcamp": "https://testartist.bandcamp.com",
                                    "facebook": null,
                                    "instagram": null,
                                    "lastfm": null,
                                    "songkick": null,
                                    "spotify": null,
                                    "twitter": null,
                                    "website": null,
                                    "wikipedia": null,
                                    "youtube": null
                                }
                            ],
                            "artists": [],
                            "venue": {
                                "name": "Conor Byrne Cooperative"
                            },
                            "footerContent": null,
                            "ticketsUrl": null
                        }
                    ],
                    "metadata": {
                        "currentPage": 1,
                        "limitValue": 10,
                        "totalCount": 1,
                        "totalPages": 1
                    }
                }
            }
        }"#;

        let parser = ConorByrneParser::new();
        let result = parser.parse_events(sample_json).await;
        
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Test Event");
        assert_eq!(events[0].artists.len(), 1);
        assert_eq!(events[0].artists[0].name, "Test Artist");
    }
}
