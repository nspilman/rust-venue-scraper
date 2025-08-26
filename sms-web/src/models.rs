// Re-export the shared domain types (currently unused, keeping for future use)
// pub use sms_core::domain::{Event, Venue, Artist};
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

// Web-specific models for GraphQL responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebEvent {
    pub id: String,
    pub title: String,
    #[serde(rename = "eventDay")]
    pub event_day: NaiveDate,
    #[serde(rename = "startTime")]
    pub start_time: Option<NaiveTime>,
    #[serde(rename = "eventUrl")]
    pub event_url: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "eventImageUrl")]
    pub event_image_url: Option<String>,
    pub venue: Option<WebVenue>,
    pub artists: Vec<WebArtist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebVenue {
    pub id: String,
    pub name: String,
    pub address: String,
    pub city: String,
    #[serde(skip)]
    pub slug: String,
}

impl WebVenue {
    pub fn create_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' => c,
                _ => '-',
            })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }

    pub fn with_slug(mut self) -> Self {
        self.slug = Self::create_slug(&self.name);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebArtist {
    pub id: String,
    pub name: String,
    #[serde(rename = "nameSlug")]
    pub name_slug: String,
    pub bio: Option<String>,
    #[serde(rename = "artistImageUrl")]
    pub artist_image_url: Option<String>,
}

#[derive(Serialize)]
pub struct GraphQLRequest {
    pub query: String,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EventFilter {
    pub venue: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
