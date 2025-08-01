use crate::carpenter::*;
use crate::error::{Result, ScraperError};
use async_trait::async_trait;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use tracing::debug;

/// Storage trait for persisting carpenter data
#[async_trait]
pub trait Storage: Send + Sync {
    // Venue operations
    async fn create_venue(&self, venue: &mut Venue) -> Result<()>;
    async fn get_venue_by_name(&self, name: &str) -> Result<Option<Venue>>;
    // Artist operations
    async fn create_artist(&self, artist: &mut Artist) -> Result<()>;
    async fn get_artist_by_name(&self, name: &str) -> Result<Option<Artist>>;

    // Event operations  
    async fn create_event(&self, event: &mut Event) -> Result<()>;
    async fn get_event_by_venue_date_title(&self, venue_id: Uuid, event_day: NaiveDate, title: &str) -> Result<Option<Event>>;
    async fn update_event(&self, event: &Event) -> Result<()>;

    // Raw data operations
    async fn create_raw_data(&self, raw_data: &mut RawData) -> Result<()>;
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
