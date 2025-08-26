use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: Option<Uuid>,
    pub name: String,
    pub name_lower: String,
    pub slug: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    pub venue_url: Option<String>,
    pub venue_image_url: Option<String>,
    pub description: Option<String>,
    pub neighborhood: Option<String>,
    pub show_venue: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: Option<Uuid>,
    pub name: String,
    pub name_slug: String,
    pub bio: Option<String>,
    pub artist_image_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<Uuid>,
    pub title: String,
    pub event_day: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub event_url: Option<String>,
    pub description: Option<String>,
    pub event_image_url: Option<String>,
    pub venue_id: Uuid,
    pub artist_ids: Vec<Uuid>,
    pub show_event: bool,
    pub finalized: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawData {
    pub id: Option<Uuid>,
    pub api_name: String,
    pub event_api_id: String,
    pub event_name: String,
    pub venue_name: String,
    pub event_day: NaiveDate,
    pub data: serde_json::Value,
    pub processed: bool,
    pub event_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRun {
    pub id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub id: Option<Uuid>,
    pub process_run_id: Uuid,
    pub api_name: String,
    pub raw_data_id: Option<Uuid>,
    pub change_type: String,
    pub change_log: String,
    pub field_changed: String,
    pub event_id: Option<Uuid>,
    pub venue_id: Option<Uuid>,
    pub artist_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl RawData {
    // Pipeline-specific conversion methods moved to scraper crate
}

