use crate::error::Result;
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

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

/// Core trait that all event data sources must implement
#[async_trait::async_trait]
pub trait EventApi: Send + Sync {
    /// Unique identifier for this API/crawler
    fn api_name(&self) -> &'static str;

    /// Fetch all events from this data source
    async fn get_event_list(&self) -> Result<Vec<RawEventData>>;

    /// Extract raw data info for storage identification
    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo>;

    /// Extract event arguments from raw data
    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs>;

    /// Determine if an event should be skipped
    fn should_skip(&self, _raw_data: &RawEventData) -> (bool, String) {
        (false, String::new())
    }
}

// Removed legacy API priority list; registry and feature flags drive execution now.
