use super::traits::Storage;
use crate::domain::*;
use crate::common::error::{Result, ScraperError};
use async_trait::async_trait;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::debug;
use uuid::Uuid;

/// In-memory storage implementation for development/testing
pub struct InMemoryStorage {
    venues: Arc<Mutex<HashMap<Uuid, Venue>>>,
    artists: Arc<Mutex<HashMap<Uuid, Artist>>>,
    events: Arc<Mutex<HashMap<Uuid, Event>>>,
    raw_data: Arc<Mutex<HashMap<Uuid, RawData>>>,
    process_runs: Arc<Mutex<HashMap<Uuid, ProcessRun>>>,
    process_records: Arc<Mutex<HashMap<Uuid, ProcessRecord>>>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            venues: Arc::new(Mutex::new(HashMap::new())),
            artists: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(HashMap::new())),
            raw_data: Arc::new(Mutex::new(HashMap::new())),
            process_runs: Arc::new(Mutex::new(HashMap::new())),
            process_records: Arc::new(Mutex::new(HashMap::new())),
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
        let venue = venues
            .values()
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
        let artist = artists
            .values()
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

    async fn get_event_by_venue_date_title(
        &self,
        venue_id: Uuid,
        event_day: NaiveDate,
        title: &str,
    ) -> Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        let event = events
            .values()
            .find(|e| {
                e.venue_id == venue_id
                    && e.event_day == event_day
                    && e.title.to_lowercase() == title.to_lowercase()
            })
            .cloned();
        Ok(event)
    }

    async fn update_event(&self, event: &Event) -> Result<()> {
        let event_id = event.id.ok_or_else(|| ScraperError::Api {
            message: "Cannot update event without ID".to_string(),
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

    async fn get_unprocessed_raw_data(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<RawData>> {
        let raw_data_map = self.raw_data.lock().unwrap();
        let mut raw_data: Vec<RawData> = raw_data_map
            .values()
            .filter(|r| !r.processed)
            .cloned()
            .collect();

        // Sort by event_day for consistent processing order
        raw_data.sort_by_key(|r| r.event_day);

        // Apply limit if provided
        if let Some(limit) = limit {
            raw_data.truncate(limit as usize);
        }

        Ok(raw_data)
    }

    async fn mark_raw_data_processed(&self, raw_data_id: Uuid) -> Result<()> {
        let mut raw_data_map = self.raw_data.lock().unwrap();
        if let Some(raw_data) = raw_data_map.get_mut(&raw_data_id) {
            raw_data.processed = true;
            // Note: processed_at field doesn't exist on RawData struct
            // This would need to be tracked separately if needed
            debug!("Marked raw data {} as processed", raw_data_id);
        }
        Ok(())
    }

    async fn create_process_run(&self, run: &mut ProcessRun) -> Result<()> {
        let id = Uuid::new_v4();
        run.id = Some(id);

        let mut runs = self.process_runs.lock().unwrap();
        runs.insert(id, run.clone());

        debug!("Created process run with id {}", id);
        Ok(())
    }

    async fn update_process_run(&self, run: &ProcessRun) -> Result<()> {
        let run_id = run.id.ok_or_else(|| ScraperError::Api {
            message: "Cannot update process run without ID".to_string(),
        })?;

        let mut runs = self.process_runs.lock().unwrap();
        runs.insert(run_id, run.clone());

        debug!("Updated process run with id {}", run_id);
        Ok(())
    }

    async fn create_process_record(&self, record: &mut ProcessRecord) -> Result<()> {
        let id = Uuid::new_v4();
        record.id = Some(id);

        let mut records = self.process_records.lock().unwrap();
        records.insert(id, record.clone());

        debug!("Created process record with id {}", id);
        Ok(())
    }

    // Query methods implementation
    async fn get_venue_by_id(&self, venue_id: Uuid) -> Result<Option<Venue>> {
        let venues = self.venues.lock().unwrap();
        Ok(venues.get(&venue_id).cloned())
    }

    async fn get_artist_by_id(&self, artist_id: Uuid) -> Result<Option<Artist>> {
        let artists = self.artists.lock().unwrap();
        Ok(artists.get(&artist_id).cloned())
    }

    async fn get_event_by_id(&self, event_id: Uuid) -> Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events.get(&event_id).cloned())
    }

    async fn get_all_venues(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Venue>> {
        let venues = self.venues.lock().unwrap();
        let mut all_venues: Vec<Venue> = venues.values().cloned().collect();
        all_venues.sort_by(|a, b| a.name.cmp(&b.name));

        let offset = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(offset + limit, all_venues.len())
        } else {
            all_venues.len()
        };

        Ok(all_venues.get(offset..end).unwrap_or(&[]).to_vec())
    }

    async fn get_all_artists(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Artist>> {
        let artists = self.artists.lock().unwrap();
        let mut all_artists: Vec<Artist> = artists.values().cloned().collect();
        all_artists.sort_by(|a, b| a.name.cmp(&b.name));

        let offset = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(offset + limit, all_artists.len())
        } else {
            all_artists.len()
        };

        Ok(all_artists.get(offset..end).unwrap_or(&[]).to_vec())
    }

    async fn get_all_events(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        let mut all_events: Vec<Event> = events.values().cloned().collect();
        all_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));

        let offset = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(offset + limit, all_events.len())
        } else {
            all_events.len()
        };

        Ok(all_events.get(offset..end).unwrap_or(&[]).to_vec())
    }

    async fn get_events_by_venue_id(&self, venue_id: Uuid) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        let mut venue_events: Vec<Event> = events
            .values()
            .filter(|e| e.venue_id == venue_id)
            .cloned()
            .collect();
        venue_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(venue_events)
    }

    async fn get_events_by_artist_id(&self, artist_id: Uuid) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        let mut artist_events: Vec<Event> = events
            .values()
            .filter(|e| e.artist_ids.contains(&artist_id))
            .cloned()
            .collect();
        artist_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(artist_events)
    }

    async fn get_events_by_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        let mut date_events: Vec<Event> = events
            .values()
            .filter(|e| e.event_day >= start_date && e.event_day <= end_date)
            .cloned()
            .collect();
        date_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(date_events)
    }

    async fn search_artists(&self, query: &str) -> Result<Vec<Artist>> {
        let artists = self.artists.lock().unwrap();
        let query_lower = query.to_lowercase();
        let mut matching_artists: Vec<Artist> = artists
            .values()
            .filter(|a| a.name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();
        matching_artists.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(matching_artists)
    }
}
