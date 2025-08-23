use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
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
    pub venue: Option<Venue>,
    pub artists: Vec<Artist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: String,
    pub name: String,
    pub address: String,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
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
}
