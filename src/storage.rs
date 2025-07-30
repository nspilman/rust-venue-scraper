use crate::carpenter::*;
use crate::error::{Result, ScraperError};
use async_trait::async_trait;
use chrono::{DateTime, Utc, NaiveDate};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use tracing::{info, warn, error, debug, instrument};

/// Storage trait for persisting carpenter data
#[async_trait]
pub trait Storage: Send + Sync {
    // Venue operations
    async fn create_venue(&self, venue: &mut Venue) -> Result<()>;
    async fn get_venue_by_name(&self, name: &str) -> Result<Option<Venue>>;
    async fn get_venue_by_coordinates(&self, lat: f64, lon: f64) -> Result<Option<Venue>>;
    async fn update_venue(&self, venue: &Venue) -> Result<()>;

    // Artist operations
    async fn create_artist(&self, artist: &mut Artist) -> Result<()>;
    async fn get_artist_by_name(&self, name: &str) -> Result<Option<Artist>>;
    async fn update_artist(&self, artist: &Artist) -> Result<()>;

    // Event operations  
    async fn create_event(&self, event: &mut Event) -> Result<()>;
    async fn get_event_by_venue_date_title(&self, venue_id: Uuid, event_day: NaiveDate, title: &str) -> Result<Option<Event>>;
    async fn update_event(&self, event: &Event) -> Result<()>;

    // Raw data operations
    async fn create_raw_data(&self, raw_data: &mut RawData) -> Result<()>;
    async fn get_raw_data_by_api_and_id(&self, api_name: &str, event_api_id: &str) -> Result<Option<RawData>>;
    async fn get_unprocessed_raw_data(&self, api_name: &str, min_date: Option<NaiveDate>) -> Result<Vec<RawData>>;
    async fn mark_raw_data_processed(&self, raw_data_id: Uuid) -> Result<()>;

    // Carpenter run operations
    async fn create_carpenter_run(&self, run: &mut CarpenterRun) -> Result<()>;
    async fn update_carpenter_run(&self, run: &CarpenterRun) -> Result<()>;

    // Carpenter record operations
    async fn create_carpenter_record(&self, record: &mut CarpenterRecord) -> Result<()>;
}

/// In-memory storage implementation for development/testing
pub struct InMemoryStorage {
    venues: Arc<Mutex<HashMap<Uuid, Venue>>>,
    artists: Arc<Mutex<HashMap<Uuid, Artist>>>,
    events: Arc<Mutex<HashMap<Uuid, Event>>>,
    raw_data: Arc<Mutex<HashMap<Uuid, RawData>>>,
    carpenter_runs: Arc<Mutex<HashMap<Uuid, CarpenterRun>>>,
    carpenter_records: Arc<Mutex<HashMap<Uuid, CarpenterRecord>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            venues: Arc::new(Mutex::new(HashMap::new())),
            artists: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(HashMap::new())),
            raw_data: Arc::new(Mutex::new(HashMap::new())),
            carpenter_runs: Arc::new(Mutex::new(HashMap::new())),
            carpenter_records: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn create_venue(&self, venue: &mut Venue) -> Result<()> {
        let id = Uuid::new_v4();
        venue.id = Some(id);
        
        let mut venues = self.venues.lock().unwrap();
        venues.insert(id, venue.clone());
        
        debug!("Created venue: {} with id {}", venue.name, id);
        Ok(())
    }

    async fn get_venue_by_name(&self, name: &str) -> Result<Option<Venue>> {
        let venues = self.venues.lock().unwrap();
        let venue = venues.values()
            .find(|v| v.name.to_lowercase() == name.to_lowercase())
            .cloned();
        Ok(venue)
    }

    async fn get_venue_by_coordinates(&self, lat: f64, lon: f64) -> Result<Option<Venue>> {
        let venues = self.venues.lock().unwrap();
        let rounded_lat = (lat * 100000.0).round() / 100000.0;
        let rounded_lon = (lon * 100000.0).round() / 100000.0;
        
        let venue = venues.values()
            .find(|v| {
                let venue_lat = (v.latitude * 100000.0).round() / 100000.0;
                let venue_lon = (v.longitude * 100000.0).round() / 100000.0;
                venue_lat == rounded_lat && venue_lon == rounded_lon
            })
            .cloned();
        Ok(venue)
    }

    async fn update_venue(&self, venue: &Venue) -> Result<()> {
        let venue_id = venue.id.ok_or_else(|| ScraperError::Api { 
            message: "Cannot update venue without ID".to_string() 
        })?;
        
        let mut venues = self.venues.lock().unwrap();
        venues.insert(venue_id, venue.clone());
        
        debug!("Updated venue: {} with id {}", venue.name, venue_id);
        Ok(())
    }

    async fn create_artist(&self, artist: &mut Artist) -> Result<()> {
        let id = Uuid::new_v4();
        artist.id = Some(id);
        
        let mut artists = self.artists.lock().unwrap();
        artists.insert(id, artist.clone());
        
        debug!("Created artist: {} with id {}", artist.name, id);
        Ok(())
    }

    async fn get_artist_by_name(&self, name: &str) -> Result<Option<Artist>> {
        let artists = self.artists.lock().unwrap();
        let artist = artists.values()
            .find(|a| a.name.to_lowercase() == name.to_lowercase())
            .cloned();
        Ok(artist)
    }

    async fn update_artist(&self, artist: &Artist) -> Result<()> {
        let artist_id = artist.id.ok_or_else(|| ScraperError::Api { 
            message: "Cannot update artist without ID".to_string() 
        })?;
        
        let mut artists = self.artists.lock().unwrap();
        artists.insert(artist_id, artist.clone());
        
        debug!("Updated artist: {} with id {}", artist.name, artist_id);
        Ok(())
    }

    async fn create_event(&self, event: &mut Event) -> Result<()> {
        let id = Uuid::new_v4();
        event.id = Some(id);
        
        let mut events = self.events.lock().unwrap();
        events.insert(id, event.clone());
        
        debug!("Created event: {} with id {}", event.title, id);
        Ok(())
    }

    async fn get_event_by_venue_date_title(&self, venue_id: Uuid, event_day: NaiveDate, title: &str) -> Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        let event = events.values()
            .find(|e| {
                e.venue_id == venue_id &&
                e.event_day == event_day &&
                e.title.to_lowercase() == title.to_lowercase()
            })
            .cloned();
        Ok(event)
    }

    async fn update_event(&self, event: &Event) -> Result<()> {
        let event_id = event.id.ok_or_else(|| ScraperError::Api { 
            message: "Cannot update event without ID".to_string() 
        })?;
        
        let mut events = self.events.lock().unwrap();
        events.insert(event_id, event.clone());
        
        debug!("Updated event: {} with id {}", event.title, event_id);
        Ok(())
    }

    async fn create_raw_data(&self, raw_data: &mut RawData) -> Result<()> {
        let id = Uuid::new_v4();
        raw_data.id = Some(id);
        
        let mut raw_data_map = self.raw_data.lock().unwrap();
        raw_data_map.insert(id, raw_data.clone());
        
        debug!("Created raw data: {} with id {}", raw_data.event_name, id);
        Ok(())
    }

    async fn get_raw_data_by_api_and_id(&self, api_name: &str, event_api_id: &str) -> Result<Option<RawData>> {
        let raw_data_map = self.raw_data.lock().unwrap();
        let raw_data = raw_data_map.values()
            .find(|rd| rd.api_name == api_name && rd.event_api_id == event_api_id)
            .cloned();
        Ok(raw_data)
    }

    async fn get_unprocessed_raw_data(&self, api_name: &str, min_date: Option<NaiveDate>) -> Result<Vec<RawData>> {
        let raw_data_map = self.raw_data.lock().unwrap();
        let mut filtered_data: Vec<RawData> = raw_data_map.values()
            .filter(|rd| {
                rd.api_name == api_name &&
                !rd.processed &&
                min_date.map_or(true, |date| rd.event_day > date)
            })
            .cloned()
            .collect();
        
        // Sort by event_day to process chronologically
        filtered_data.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(filtered_data)
    }

    async fn mark_raw_data_processed(&self, raw_data_id: Uuid) -> Result<()> {
        let mut raw_data_map = self.raw_data.lock().unwrap();
        if let Some(raw_data) = raw_data_map.get_mut(&raw_data_id) {
            raw_data.processed = true;
            debug!("Marked raw data {} as processed", raw_data_id);
        }
        Ok(())
    }

    async fn create_carpenter_run(&self, run: &mut CarpenterRun) -> Result<()> {
        let id = Uuid::new_v4();
        run.id = Some(id);
        
        let mut runs = self.carpenter_runs.lock().unwrap();
        runs.insert(id, run.clone());
        
        debug!("Created carpenter run: {} with id {}", run.name, id);
        Ok(())
    }

    async fn update_carpenter_run(&self, run: &CarpenterRun) -> Result<()> {
        let run_id = run.id.ok_or_else(|| ScraperError::Api { 
            message: "Cannot update carpenter run without ID".to_string() 
        })?;
        
        let mut runs = self.carpenter_runs.lock().unwrap();
        runs.insert(run_id, run.clone());
        
        debug!("Updated carpenter run: {} with id {}", run.name, run_id);
        Ok(())
    }

    async fn create_carpenter_record(&self, record: &mut CarpenterRecord) -> Result<()> {
        let id = Uuid::new_v4();
        record.id = Some(id);
        
        let mut records = self.carpenter_records.lock().unwrap();
        records.insert(id, record.clone());
        
        debug!("Created carpenter record with id {}", id);
        Ok(())
    }
}
