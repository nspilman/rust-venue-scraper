use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// Raw event data as returned from external APIs/crawlers
pub type RawEventData = serde_json::Value;

/// Information needed to identify and store raw event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDataInfo {
    pub event_api_id: String,
    pub event_name: String,
    pub venue_name: String,
    pub event_day: NaiveDate,
}

/// Arguments for creating/updating a venue
#[derive(Debug, Clone)]
pub struct VenueArgs {
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub api_id: Option<String>,
}

/// Arguments for creating/updating an artist
#[derive(Debug, Clone)]
pub struct ArtistArgs {
    pub name: String,
    pub bio: Option<String>,
    pub artist_image_url: Option<String>,
}

/// Arguments for creating/updating an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventArgs {
    pub title: String,
    pub event_day: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub event_url: Option<String>,
    pub description: Option<String>,
    pub event_image_url: Option<String>,
}

/// Types of changes that can occur during processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Updated,
    NoChange,
    Skip,
    Error,
}

/// Result of a data operation with change tracking
#[derive(Debug, Clone)]
pub struct DataResult<T> {
    pub change_type: ChangeType,
    pub change_log: String,
    pub data: T,
}

/// Core trait that all event data sources must implement
#[async_trait::async_trait]
pub trait EventApi: Send + Sync {
    /// Unique identifier for this API/crawler
    fn api_name(&self) -> &'static str;
    
    /// Whether this API provides venue information
    fn has_venues(&self) -> bool {
        true
    }
    
    /// Whether this API provides artist information
    fn has_artists(&self) -> bool {
        false
    }
    
    /// Fetch all events from this data source
    async fn get_event_list(&self) -> Result<Vec<RawEventData>>;
    
    /// Extract raw data info for storage identification
    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo>;
    
    /// Extract venue arguments from raw data
    fn get_venue_args(&self, _raw_data: &RawEventData) -> Result<VenueArgs> {
        Err(crate::error::ScraperError::Config("This API does not provide venue data".to_string()))
    }
    
    /// Extract artist arguments from raw data
    fn get_artists_args(&self, _raw_data: &RawEventData) -> Result<Vec<ArtistArgs>> {
        Ok(vec![])
    }
    
    /// Extract event arguments from raw data
    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs>;
    
    /// Determine if an event should be skipped
    fn should_skip(&self, _raw_data: &RawEventData) -> (bool, String) {
        (false, String::new())
    }
    
    /// Get a fixed venue for crawlers that target a single venue
    fn get_venue(&self) -> Option<String> {
        None
    }
}

/// Configuration for API rate limiting and delays
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub delay_ms: Option<u64>,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            delay_ms: None,
            max_retries: 3,
            timeout_seconds: 15,
        }
    }
}

/// Represents the priority order of APIs for processing
pub const API_PRIORITY_ORDER: &[&str] = &[
    "manual",
    "dice", 
    "axs",
    "tixr",
    "venuepilot",
    "songkick",
    "bandsintown",
    "crawler_blue_moon",
    "crawler_darrells_tavern", 
    "crawler_little_red_hen",
    "crawler_sea_monster_lounge",
    "crawler_skylark",
    "crawler_the_royal_room",
    "eventbrite",
    "ticketmaster",
];
