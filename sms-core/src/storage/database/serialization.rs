use crate::domain::*;
use crate::error::{Result, ScraperError};
use uuid::Uuid;

/// Helper functions for serializing and deserializing domain objects to/from database nodes
pub struct Serialization;

impl Serialization {
    /// Convert venue to node data
    pub fn venue_to_node_data(venue: &Venue) -> Result<String> {
        serde_json::to_string(venue).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize venue: {e}"),
        })
    }

    /// Convert node data to venue
    pub fn node_data_to_venue(id: &str, data: &str) -> Result<Venue> {
        let mut venue: Venue = serde_json::from_str(data).map_err(|e| ScraperError::Database {
            message: format!("Failed to deserialize venue: {e}"),
        })?;
        venue.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid venue UUID: {e}"),
        })?);
        Ok(venue)
    }

    /// Convert artist to node data
    pub fn artist_to_node_data(artist: &Artist) -> Result<String> {
        serde_json::to_string(artist).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize artist: {e}"),
        })
    }

    /// Convert node data to artist
    pub fn node_data_to_artist(id: &str, data: &str) -> Result<Artist> {
        let mut artist: Artist =
            serde_json::from_str(data).map_err(|e| ScraperError::Database {
                message: format!("Failed to deserialize artist: {e}"),
            })?;
        artist.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid artist UUID: {e}"),
        })?);
        Ok(artist)
    }

    /// Convert event to node data
    pub fn event_to_node_data(event: &Event) -> Result<String> {
        serde_json::to_string(event).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize event: {e}"),
        })
    }

    /// Convert node data to event
    pub fn node_data_to_event(id: &str, data: &str) -> Result<Event> {
        let mut event: Event = serde_json::from_str(data).map_err(|e| ScraperError::Database {
            message: format!("Failed to deserialize event: {e}"),
        })?;
        event.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid event UUID: {e}"),
        })?);
        Ok(event)
    }

    /// Convert raw data to node data
    pub fn raw_data_to_node_data(raw_data: &RawData) -> Result<String> {
        serde_json::to_string(raw_data).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize raw data: {e}"),
        })
    }

    /// Convert node data to raw data
    pub fn node_data_to_raw_data(id: &str, data: &str) -> Result<RawData> {
        let mut raw_data: RawData =
            serde_json::from_str(data).map_err(|e| ScraperError::Database {
                message: format!("Failed to deserialize raw data: {e}"),
            })?;
        raw_data.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid raw data UUID: {e}"),
        })?);
        Ok(raw_data)
    }

    /// Convert process run to node data
    pub fn process_run_to_node_data(run: &ProcessRun) -> Result<String> {
        serde_json::to_string(run).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize process run: {e}"),
        })
    }

    /// Convert process record to node data
    pub fn process_record_to_node_data(record: &ProcessRecord) -> Result<String> {
        serde_json::to_string(record).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize process record: {e}"),
        })
    }
}
